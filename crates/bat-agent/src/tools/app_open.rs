use anyhow::Result;
use std::process::Command;

pub struct AppOpen;

impl AppOpen {
    pub fn new() -> Self {
        Self
    }
}

impl super::ToolExecutor for AppOpen {
    fn name(&self) -> &str {
        "app_open"
    }

    fn description(&self) -> &str {
        "Open a file, URL, or application using the system default handler. \
         Works like double-clicking a file or typing a URL in a browser."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "File path, URL, or application name to open"
                }
            },
            "required": ["target"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let target = input["target"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'target' parameter"))?;

        let result = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "start", "", target])
                .output()
        } else if cfg!(target_os = "macos") {
            Command::new("open")
                .arg(target)
                .output()
        } else {
            Command::new("xdg-open")
                .arg(target)
                .output()
        };

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(format!("Opened: {target}"))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(anyhow::anyhow!("Failed to open: {stderr}"))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to open: {e}")),
        }
    }
}
