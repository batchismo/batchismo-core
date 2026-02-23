use anyhow::Result;

use bat_types::ipc::{ProcessAction, ProcessResult};
use crate::gateway_bridge::GatewayBridge;

pub struct ExecRun {
    bridge: GatewayBridge,
}

impl ExecRun {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for ExecRun {
    fn name(&self) -> &str {
        "exec_run"
    }

    fn description(&self) -> &str {
        "Start a shell command. By default runs in foreground and waits for completion. \
         Set background=true for long-running tasks — returns a session_id you can use \
         with exec_output, exec_write, and exec_kill to manage the process."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "background": {
                    "type": "boolean",
                    "description": "If true, run in background and return session_id immediately. Default: false."
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory for the command (optional)"
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'command' parameter"))?;

        let background = input["background"].as_bool().unwrap_or(false);
        let workdir = input["workdir"].as_str().map(|s| s.to_string());

        let result = self.bridge.request(ProcessAction::Start {
            command: command.to_string(),
            workdir,
            background,
        });

        match result {
            ProcessResult::Started { session_id } => {
                Ok(format!("Process started in background. Session ID: {session_id}\n\
                    Use exec_output to check progress, exec_write to send input, exec_kill to terminate."))
            }
            ProcessResult::Output { stdout, stderr, exit_code, .. } => {
                let mut out = String::new();
                if !stdout.is_empty() {
                    out.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !out.is_empty() { out.push('\n'); }
                    out.push_str("[stderr] ");
                    out.push_str(&stderr);
                }
                if let Some(code) = exit_code {
                    if code != 0 {
                        out.push_str(&format!("\n(exit code {code})"));
                    }
                }
                if out.is_empty() {
                    out = format!("(no output, exit code {})", exit_code.unwrap_or(0));
                }
                Ok(out)
            }
            ProcessResult::Error { message } => Err(anyhow::anyhow!(message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}
