use anyhow::{bail, Result};
use bat_types::policy::{check_access, strip_win_prefix, PathPolicy};
use std::path::Path;

pub struct FsWrite {
    policies: Vec<PathPolicy>,
}

impl FsWrite {
    pub fn new(policies: Vec<PathPolicy>) -> Self {
        Self { policies }
    }
}

impl super::ToolExecutor for FsWrite {
    fn name(&self) -> &str {
        "fs_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file if it doesn't exist. Creates parent directories if needed."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to write to"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' parameter"))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' parameter"))?;

        let path = Path::new(path_str);

        // For new files, we can't canonicalize yet — check parent dir
        let check_path = if path.exists() {
            path.canonicalize()?
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
            parent.canonicalize()?.join(path.file_name().unwrap())
        };

        if !check_access(&self.policies, &check_path, true) {
            bail!(
                "Access denied: '{}' is not in any allowed write policy",
                strip_win_prefix(&check_path).display()
            );
        }

        std::fs::write(&check_path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", strip_win_prefix(&check_path).display(), e))?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            strip_win_prefix(&check_path).display()
        ))
    }
}
