use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct SessionCancel {
    bridge: GatewayBridge,
}

impl SessionCancel {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionCancel {
    fn name(&self) -> &str { "session_cancel" }

    fn description(&self) -> &str {
        "Cancel a sub-agent and clean up. The sub-agent will be terminated."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "The session key of the sub-agent to cancel"
                }
            },
            "required": ["session_key"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let session_key = input.get("session_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'session_key' parameter"))?;

        let action = ProcessAction::CancelSubagent {
            session_key: session_key.to_string(),
        };

        match self.bridge.request(action) {
            ProcessResult::SubagentCancelled => {
                Ok(json!({
                    "status": "cancelled",
                    "session_key": session_key,
                    "message": "Sub-agent has been cancelled"
                }).to_string())
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to cancel sub-agent: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}