use anyhow::Result;

use bat_types::ipc::{ProcessAction, ProcessResult};
use crate::gateway_bridge::GatewayBridge;

pub struct ExecKill {
    bridge: GatewayBridge,
}

impl ExecKill {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for ExecKill {
    fn name(&self) -> &str {
        "exec_kill"
    }

    fn description(&self) -> &str {
        "Kill a running background process."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The session ID of the process to kill"
                }
            },
            "required": ["session_id"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let session_id = input["session_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'session_id' parameter"))?;

        let result = self.bridge.request(ProcessAction::Kill {
            session_id: session_id.to_string(),
        });

        match result {
            ProcessResult::Killed => Ok(format!("Process {session_id} killed")),
            ProcessResult::Error { message } => Err(anyhow::anyhow!(message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}
