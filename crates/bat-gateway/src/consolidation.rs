//! Memory consolidation — uses the LLM to update MEMORY.md and PATTERNS.md
//! based on accumulated observations.

use anyhow::{Context, Result};
use tracing::info;

use bat_types::audit::{AuditCategory, AuditLevel};
use bat_types::memory::ObservationFilter;

use crate::db::Database;
use crate::events::EventBus;
use crate::memory;

/// Run memory consolidation. Reads observations + current memory files,
/// calls the LLM to produce updated versions, and writes them back.
pub async fn run_consolidation(
    db: &Database,
    _event_bus: &EventBus,
    api_key: &str,
    model: &str,
) -> Result<ConsolidationResult> {
    info!("Starting memory consolidation");

    // 1. Gather recent observations
    let observations = db.get_observations(&ObservationFilter {
        limit: Some(100),
        ..Default::default()
    })?;

    if observations.is_empty() {
        info!("No observations to consolidate");
        return Ok(ConsolidationResult {
            files_updated: vec![],
            observations_processed: 0,
        });
    }

    // 2. Build observation summary text
    let mut obs_text = String::new();
    for obs in &observations {
        obs_text.push_str(&format!(
            "- [{}] {}: {} (count: {})\n",
            obs.kind, obs.key, obs.value.as_deref().unwrap_or(""), obs.count
        ));
    }

    // 3. Read current memory files
    let current_memory = memory::read_memory_file("MEMORY.md").unwrap_or_default();
    let current_patterns = memory::read_memory_file("PATTERNS.md").unwrap_or_default();

    // 4. Build the consolidation prompt
    let system_prompt = r#"You are a memory consolidation agent. Your job is to update the user's memory files based on observed behavioral patterns.

Rules:
- Only add facts that are clearly supported by the observations
- Never include raw conversation content, API keys, or credentials
- Keep entries concise and actionable
- Preserve existing entries unless they contradict new observations
- Use markdown formatting consistent with the existing file style
- If there's nothing meaningful to update, return the file unchanged

Respond with exactly two sections:
===MEMORY.md===
(full updated contents of MEMORY.md)
===PATTERNS.md===
(full updated contents of PATTERNS.md)"#;

    let user_message = format!(
        "Here are the recent behavioral observations:\n\n{}\n\nCurrent MEMORY.md:\n```\n{}\n```\n\nCurrent PATTERNS.md:\n```\n{}\n```\n\nPlease produce updated versions of both files based on these observations.",
        obs_text, current_memory, current_patterns
    );

    // 5. Call the LLM
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_message}]
        }))
        .send()
        .await
        .context("Failed to call Anthropic API for consolidation")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Consolidation API error ({}): {}", status, body);
    }

    let body: serde_json::Value = resp.json().await.context("Failed to parse consolidation response")?;
    let response_text = body["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // 6. Parse the response into file updates
    let mut files_updated = Vec::new();

    if let Some(memory_content) = extract_section(&response_text, "MEMORY.md") {
        let trimmed = memory_content.trim();
        if !trimmed.is_empty() && trimmed != current_memory.trim() {
            memory::write_memory_file("MEMORY.md", trimmed)?;
            files_updated.push("MEMORY.md".to_string());
            info!("Updated MEMORY.md");
        }
    }

    if let Some(patterns_content) = extract_section(&response_text, "PATTERNS.md") {
        let trimmed = patterns_content.trim();
        if !trimmed.is_empty() && trimmed != current_patterns.trim() {
            memory::write_memory_file("PATTERNS.md", trimmed)?;
            files_updated.push("PATTERNS.md".to_string());
            info!("Updated PATTERNS.md");
        }
    }

    let result = ConsolidationResult {
        observations_processed: observations.len(),
        files_updated,
    };

    // 7. Audit log
    let ts = chrono::Utc::now().to_rfc3339();
    let summary = format!(
        "Memory consolidation: {} observations processed, {} files updated",
        result.observations_processed,
        result.files_updated.len(),
    );
    let _ = db.insert_audit_log(
        &ts, None, AuditLevel::Info, AuditCategory::Agent,
        "memory_consolidation", &summary, None,
    );

    info!("{}", summary);
    Ok(result)
}

/// Extract content between ===FILENAME=== markers.
fn extract_section(text: &str, filename: &str) -> Option<String> {
    let marker = format!("==={}===", filename);
    let start = text.find(&marker)?;
    let content_start = start + marker.len();

    // Find the next === marker or end of text
    let content_end = text[content_start..]
        .find("===")
        .map(|i| content_start + i)
        .unwrap_or(text.len());

    Some(text[content_start..content_end].to_string())
}

pub struct ConsolidationResult {
    pub files_updated: Vec<String>,
    pub observations_processed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_section_basic() {
        let text = "===MEMORY.md===\n# Memory\nSome content\n===PATTERNS.md===\n# Patterns\nMore content";
        let memory = extract_section(text, "MEMORY.md").unwrap();
        assert!(memory.contains("# Memory"));
        assert!(memory.contains("Some content"));
        assert!(!memory.contains("# Patterns"));

        let patterns = extract_section(text, "PATTERNS.md").unwrap();
        assert!(patterns.contains("# Patterns"));
    }

    #[test]
    fn extract_section_missing() {
        let text = "no markers here";
        assert!(extract_section(text, "MEMORY.md").is_none());
    }
}
