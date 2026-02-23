use anyhow::Result;

use bat_types::ipc::{ProcessAction, ProcessResult};
use crate::gateway_bridge::GatewayBridge;

pub struct ExecOutput {
    bridge: GatewayBridge,
}

impl ExecOutput {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for ExecOutput {
    fn name(&self) -> &str {
        "exec_output"
    }

    fn description(&self) -> &str {
        "Get output from a background process started with exec_run. Returns stdout, stderr, \
         running status, and exit code."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The session ID returned by exec_run"
                }
            },
            "required": ["session_id"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let session_id = input["session_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'session_id' parameter"))?;

        let result = self.bridge.request(ProcessAction::GetOutput {
            session_id: session_id.to_string(),
        });

        match result {
            ProcessResult::Output { stdout, stderr, is_running, exit_code, .. } => {
                let status = if is_running {
                    "Running".to_string()
                } else {
                    format!("Exited (code: {})", exit_code.unwrap_or(-1))
                };
                let mut out = format!("Status: {status}\n");
                if !stdout.is_empty() {
                    out.push_str("--- stdout ---\n");
                    out.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    out.push_str("\n--- stderr ---\n");
                    out.push_str(&stderr);
                }
                if stdout.is_empty() && stderr.is_empty() {
                    out.push_str("(no output yet)");
                }
                Ok(out)
            }
            ProcessResult::Error { message } => Err(anyhow::anyhow!(message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}
