//! Active memory reflection — after each orchestrator turn, the LLM decides
//! if anything from the conversation is worth remembering long-term.

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::memory;

/// Run a lightweight reflection check after an orchestrator turn.
/// Takes the user message and assistant response, asks a cheap model
/// if anything is worth remembering, and if so appends to MEMORY.md.
pub async fn maybe_remember(
    api_key: &str,
    user_message: &str,
    assistant_response: &str,
) -> Result<()> {
    let memory_content = memory::read_memory_file("MEMORY.md").unwrap_or_default();

    let prompt = format!(
        r#"You are a memory reflection system. You just observed this exchange:

USER: {user_message}

ASSISTANT: {assistant_response}

Current MEMORY.md contents:
---
{memory_content}
---

Based on this exchange, is there anything worth adding to long-term memory?

Worth remembering:
- User preferences or corrections ("don't do X", "I prefer Y")
- Decisions made ("we decided to use X approach")
- Lessons learned (something failed and was resolved)
- Important facts about the user or their projects
- Behavioral feedback ("always do X when Y happens")

NOT worth remembering:
- Routine task delegation
- Small talk or greetings
- Information already in MEMORY.md
- Temporary/one-off requests

If there IS something worth remembering, respond with ONLY the line(s) to append to MEMORY.md. Use concise bullet points starting with "- ". Keep it brief — one or two lines max.

If there is NOTHING worth remembering, respond with exactly: NOTHING"#
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-latest",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await
        .context("Reflection API request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        warn!("Reflection API error ({status}): {body}");
        return Ok(()); // Don't fail the turn over a reflection error
    }

    let body: serde_json::Value = response.json().await.context("Failed to parse reflection response")?;
    let text = body["content"][0]["text"]
        .as_str()
        .unwrap_or("NOTHING")
        .trim();

    if text == "NOTHING" || text.is_empty() {
        info!("Reflection: nothing worth remembering");
        return Ok(());
    }

    // Append to MEMORY.md
    info!("Reflection: adding to memory — {}", text);
    let updated = if memory_content.is_empty() {
        format!("# Memory\n\n{text}\n")
    } else {
        format!("{memory_content}\n{text}\n")
    };

    memory::write_memory_file("MEMORY.md", &updated)
        .context("Failed to write reflection to MEMORY.md")?;

    Ok(())
}
