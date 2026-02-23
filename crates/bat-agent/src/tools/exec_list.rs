use anyhow::Result;

use bat_types::ipc::{ProcessAction, ProcessResult};
use crate::gateway_bridge::GatewayBridge;

pub struct ExecList {
    bridge: GatewayBridge,
}

impl ExecList {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for ExecList {
    fn name(&self) -> &str {
        "exec_list"
    }

    fn description(&self) -> &str {
        "List all managed background processes with their status."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute(&self, _input: &serde_json::Value) -> Result<String> {
        let result = self.bridge.request(ProcessAction::List);

        match result {
            ProcessResult::ProcessList { processes } => {
                if processes.is_empty() {
                    return Ok("No managed processes.".to_string());
                }
                let mut out = String::new();
                for p in &processes {
                    let status = if p.is_running {
                        "running".to_string()
                    } else {
                        format!("exited (code: {})", p.exit_code.unwrap_or(-1))
                    };
                    out.push_str(&format!(
                        "  {} | {} | {}\n",
                        p.session_id, status, p.command
                    ));
                }
                Ok(out)
            }
            ProcessResult::Error { message } => Err(anyhow::anyhow!(message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}
