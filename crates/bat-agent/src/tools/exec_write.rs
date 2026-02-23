use anyhow::Result;

use bat_types::ipc::{ProcessAction, ProcessResult};
use crate::gateway_bridge::GatewayBridge;

pub struct ExecWrite {
    bridge: GatewayBridge,
}

impl ExecWrite {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for ExecWrite {
    fn name(&self) -> &str {
        "exec_write"
    }

    fn description(&self) -> &str {
        "Write data to the stdin of a running background process."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The session ID of the background process"
                },
                "data": {
                    "type": "string",
                    "description": "Data to write to stdin (a newline is NOT automatically appended)"
                }
            },
            "required": ["session_id", "data"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let session_id = input["session_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'session_id' parameter"))?;
        let data = input["data"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'data' parameter"))?;

        let result = self.bridge.request(ProcessAction::WriteStdin {
            session_id: session_id.to_string(),
            data: data.to_string(),
        });

        match result {
            ProcessResult::Written => Ok("Data written to stdin".to_string()),
            ProcessResult::Error { message } => Err(anyhow::anyhow!(message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}
