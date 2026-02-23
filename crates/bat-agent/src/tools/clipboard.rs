use anyhow::Result;
use serde_json::{Value, json};

pub struct Clipboard;

impl Clipboard {
    pub fn new() -> Self { Self }
}

impl super::ToolExecutor for Clipboard {
    fn name(&self) -> &str { "clipboard" }
    fn description(&self) -> &str {
        "Read or write the system clipboard. Use action 'read' to get clipboard contents, or 'write' with 'text' to set clipboard contents."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write"],
                    "description": "Whether to read from or write to the clipboard."
                },
                "text": {
                    "type": "string",
                    "description": "Text to write to clipboard (required for 'write' action)."
                }
            },
            "required": ["action"]
        })
    }
    fn execute(&self, input: &Value) -> Result<String> {
        let action = input.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        match action {
            "read" => {
                // Use platform-specific clipboard access
                #[cfg(target_os = "windows")]
                {
                    let output = std::process::Command::new("powershell")
                        .args(["-NoProfile", "-Command", "Get-Clipboard"])
                        .output()?;
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        Ok(json!({ "content": text }).to_string())
                    } else {
                        Err(anyhow::anyhow!("Failed to read clipboard"))
                    }
                }
                #[cfg(target_os = "macos")]
                {
                    let output = std::process::Command::new("pbpaste").output()?;
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        Ok(json!({ "content": text }).to_string())
                    } else {
                        Err(anyhow::anyhow!("Failed to read clipboard"))
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    let output = std::process::Command::new("xclip")
                        .args(["-selection", "clipboard", "-o"])
                        .output()
                        .or_else(|_| {
                            std::process::Command::new("xsel")
                                .args(["--clipboard", "--output"])
                                .output()
                        })?;
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        Ok(json!({ "content": text }).to_string())
                    } else {
                        Err(anyhow::anyhow!("Failed to read clipboard. Install xclip or xsel."))
                    }
                }
            }
            "write" => {
                let text = input.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text' parameter for write action"))?;

                #[cfg(target_os = "windows")]
                {
                    use std::process::Stdio;
                    let mut child = std::process::Command::new("powershell")
                        .args(["-NoProfile", "-Command", &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"))])
                        .stdin(Stdio::null())
                        .spawn()?;
                    child.wait()?;
                    Ok(json!({ "status": "written", "length": text.len() }).to_string())
                }
                #[cfg(target_os = "macos")]
                {
                    use std::io::Write;
                    use std::process::Stdio;
                    let mut child = std::process::Command::new("pbcopy")
                        .stdin(Stdio::piped())
                        .spawn()?;
                    child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
                    child.wait()?;
                    Ok(json!({ "status": "written", "length": text.len() }).to_string())
                }
                #[cfg(target_os = "linux")]
                {
                    use std::io::Write;
                    use std::process::Stdio;
                    let mut child = std::process::Command::new("xclip")
                        .args(["-selection", "clipboard"])
                        .stdin(Stdio::piped())
                        .spawn()
                        .or_else(|_| {
                            std::process::Command::new("xsel")
                                .args(["--clipboard", "--input"])
                                .stdin(Stdio::piped())
                                .spawn()
                        })?;
                    child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
                    child.wait()?;
                    Ok(json!({ "status": "written", "length": text.len() }).to_string())
                }
            }
            _ => Err(anyhow::anyhow!("Invalid action '{}'. Use 'read' or 'write'.", action)),
        }
    }
}
