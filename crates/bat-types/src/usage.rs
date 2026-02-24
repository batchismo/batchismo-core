use serde::{Deserialize, Serialize};

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    /// Total tokens across all sessions.
    pub total_input: i64,
    pub total_output: i64,
    /// Per-session breakdown.
    pub sessions: Vec<SessionUsage>,
    /// Per-model breakdown.
    pub by_model: Vec<ModelUsage>,
    /// Estimated cost in USD (based on Anthropic pricing).
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsage {
    pub key: String,
    pub model: String,
    pub token_input: i64,
    pub token_output: i64,
    pub message_count: i64,
    pub last_active: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub token_input: i64,
    pub token_output: i64,
    pub session_count: i64,
}

/// Estimate cost for Anthropic Claude models.
pub fn estimate_cost(model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
    let (input_per_m, output_per_m) = match model {
        m if m.contains("opus") => (15.0, 75.0),
        m if m.contains("sonnet") => (3.0, 15.0),
        m if m.contains("haiku") => (0.25, 1.25),
        _ => (3.0, 15.0), // default to sonnet pricing
    };
    (input_tokens as f64 / 1_000_000.0 * input_per_m)
        + (output_tokens as f64 / 1_000_000.0 * output_per_m)
}
