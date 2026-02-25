use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct SessionInstruct {
    bridge: GatewayBridge,
}

impl SessionInstruct {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionInstruct {
    fn name(&self) -> &str { "session_instruct" }

    fn description(&self) -> &str {
        "Send new instructions to a running sub-agent mid-task."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "The session key of the sub-agent to instruct"
                },
                "instruction": {
                    "type": "string",
                    "description": "The instruction to send to the sub-agent"
                }
            },
            "required": ["session_key", "instruction"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let session_key = input.get("session_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'session_key' parameter"))?;

        let instruction = input.get("instruction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'instruction' parameter"))?;

        let action = ProcessAction::InstructSubagent {
            session_key: session_key.to_string(),
            instruction: instruction.to_string(),
        };

        match self.bridge.request(action) {
            ProcessResult::SubagentInstructed => {
                Ok(json!({
                    "status": "instructed",
                    "session_key": session_key,
                    "instruction": instruction,
                    "message": "Instruction sent to sub-agent"
                }).to_string())
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to instruct sub-agent: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}