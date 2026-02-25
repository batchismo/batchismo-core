use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;

pub struct SessionAnswer {
    #[allow(dead_code)]
    bridge: GatewayBridge,
}

impl SessionAnswer {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionAnswer {
    fn name(&self) -> &str { "session_answer" }

    fn description(&self) -> &str {
        "Answer a pending question from a sub-agent. Use the session_key to identify which sub-agent to answer."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "The session key of the sub-agent that asked the question"
                },
                "answer": {
                    "type": "string",
                    "description": "Your answer to the sub-agent's question"
                }
            },
            "required": ["session_key", "answer"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let session_key = input.get("session_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'session_key' parameter"))?;

        let answer = input.get("answer")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'answer' parameter"))?;

        // For now, return a placeholder - this will need proper implementation
        // when we have the full message routing system
        Ok(json!({
            "status": "answered",
            "session_key": session_key,
            "message": format!("Answer sent to sub-agent {}: {}", session_key, answer)
        }).to_string())
    }
}