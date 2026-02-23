use anyhow::Result;
use serde_json::{Value, json};
use std::path::PathBuf;

pub struct Screenshot;

impl Screenshot {
    pub fn new() -> Self { Self }

    fn screenshots_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("batchismo")
            .join("screenshots");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

impl super::ToolExecutor for Screenshot {
    fn name(&self) -> &str { "screenshot" }
    fn description(&self) -> &str {
        "Take a screenshot of the current screen. Returns the path to the saved screenshot file."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {
                    "type": "string",
                    "description": "Optional filename (without extension). Defaults to timestamp."
                }
            },
            "required": []
        })
    }
    fn execute(&self, input: &Value) -> Result<String> {
        let filename = input.get("filename")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                chrono::Local::now().format("screenshot_%Y%m%d_%H%M%S").to_string()
            });

        let dir = Self::screenshots_dir()?;
        let path = dir.join(format!("{filename}.png"));
        let path_str = path.to_string_lossy().to_string();

        #[cfg(target_os = "windows")]
        {
            // Use PowerShell to capture screen
            let ps_script = format!(
                r#"
                Add-Type -AssemblyName System.Windows.Forms
                $screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
                $bitmap = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
                $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
                $graphics.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
                $bitmap.Save('{}')
                $graphics.Dispose()
                $bitmap.Dispose()
                "#,
                path_str.replace('\\', "\\\\").replace('\'', "''")
            );
            let output = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_script])
                .output()?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Screenshot failed: {err}"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("screencapture")
                .args(["-x", &path_str])
                .output()?;
            if !output.status.success() {
                return Err(anyhow::anyhow!("screencapture failed"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try multiple screenshot tools
            let result = std::process::Command::new("gnome-screenshot")
                .args(["-f", &path_str])
                .output()
                .or_else(|_| {
                    std::process::Command::new("scrot")
                        .arg(&path_str)
                        .output()
                })
                .or_else(|_| {
                    std::process::Command::new("import")
                        .args(["-window", "root", &path_str])
                        .output()
                });
            match result {
                Ok(output) if output.status.success() => {}
                _ => return Err(anyhow::anyhow!("No screenshot tool found. Install gnome-screenshot, scrot, or imagemagick.")),
            }
        }

        Ok(json!({
            "path": path_str,
            "filename": format!("{filename}.png"),
        }).to_string())
    }
}
