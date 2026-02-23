use anyhow::Result;
use std::process::Command;

pub struct ShellRun;

impl ShellRun {
    pub fn new() -> Self {
        Self
    }
}

impl super::ToolExecutor for ShellRun {
    fn name(&self) -> &str {
        "shell_run"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. The command runs in the system shell (cmd on Windows, sh on Unix). Times out after 30 seconds."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'command' parameter"))?;

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .output()
        } else {
            Command::new("sh")
                .args(["-c", command])
                .output()
        }
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("[stderr] ");
            result.push_str(&stderr);
        }
        if result.is_empty() {
            result = format!("(no output, exit code {})", code);
        } else if code != 0 {
            result.push_str(&format!("\n(exit code {})", code));
        }

        // Truncate
        if result.len() > 50_000 {
            Ok(format!(
                "{}\n\n[Truncated: output is {} bytes]",
                &result[..50_000],
                result.len()
            ))
        } else {
            Ok(result)
        }
    }
}
