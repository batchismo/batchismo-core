use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct SessionSpawn {
    bridge: GatewayBridge,
}

impl SessionSpawn {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionSpawn {
    fn name(&self) -> &str { "session_spawn" }
    fn description(&self) -> &str {
        "Spawn a background subagent to handle a task concurrently. Returns immediately with a session key. The subagent runs independently and announces results when done."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The task for the subagent to complete. Be specific and detailed."
                },
                "label": {
                    "type": "string",
                    "description": "Short label for this subagent (shown in UI). Defaults to first 40 chars of task."
                }
            },
            "required": ["task"]
        })
    }
    fn execute(&self, input: &Value) -> Result<String> {
        let task = input.get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'task' parameter"))?;
        let label = input.get("label").and_then(|v| v.as_str()).map(|s| s.to_string());

        let action = ProcessAction::SpawnSubagent { task: task.to_string(), label: label.clone() };
        match self.bridge.request(action) {
            ProcessResult::SubagentSpawned { session_key, session_id } => {
                Ok(json!({
                    "status": "spawned",
                    "session_key": session_key,
                    "session_id": session_id,
                    "label": label.unwrap_or_else(|| task.chars().take(40).collect()),
                    "message": "Subagent spawned and running in background. You'll receive a notification when it completes."
                }).to_string())
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to spawn subagent: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}
