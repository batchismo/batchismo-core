use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct SessionResume {
    bridge: GatewayBridge,
}

impl SessionResume {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionResume {
    fn name(&self) -> &str { "session_resume" }

    fn description(&self) -> &str {
        "Resume a paused sub-agent, optionally with new instructions."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "The session key of the sub-agent to resume"
                },
                "instructions": {
                    "type": "string",
                    "description": "Optional new instructions to send when resuming"
                }
            },
            "required": ["session_key"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let session_key = input.get("session_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'session_key' parameter"))?;

        let instructions = input.get("instructions")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let action = ProcessAction::ResumeSubagent {
            session_key: session_key.to_string(),
            instructions,
        };

        match self.bridge.request(action) {
            ProcessResult::SubagentResumed => {
                Ok(json!({
                    "status": "resumed",
                    "session_key": session_key,
                    "message": "Sub-agent has been resumed"
                }).to_string())
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to resume sub-agent: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}