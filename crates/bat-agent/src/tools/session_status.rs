use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct SessionStatus {
    bridge: GatewayBridge,
}

impl SessionStatus {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionStatus {
    fn name(&self) -> &str { "session_status" }
    fn description(&self) -> &str {
        "Get the status of all spawned subagents, including their task, label, status, and summary."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    fn execute(&self, _input: &Value) -> Result<String> {
        let action = ProcessAction::ListSubagents;
        match self.bridge.request(action) {
            ProcessResult::SubagentList { subagents } => {
                if subagents.is_empty() {
                    Ok("No subagents have been spawned.".to_string())
                } else {
                    Ok(json!({ "subagents": subagents }).to_string())
                }
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Error: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}
