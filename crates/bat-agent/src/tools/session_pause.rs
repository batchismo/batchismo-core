use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct SessionPause {
    bridge: GatewayBridge,
}

impl SessionPause {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionPause {
    fn name(&self) -> &str { "session_pause" }

    fn description(&self) -> &str {
        "Pause a running sub-agent. The sub-agent will stop after its current step."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "The session key of the sub-agent to pause"
                }
            },
            "required": ["session_key"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let session_key = input.get("session_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'session_key' parameter"))?;

        let action = ProcessAction::PauseSubagent {
            session_key: session_key.to_string(),
        };

        match self.bridge.request(action) {
            ProcessResult::SubagentPaused => {
                Ok(json!({
                    "status": "paused",
                    "session_key": session_key,
                    "message": "Sub-agent has been paused"
                }).to_string())
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to pause sub-agent: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}