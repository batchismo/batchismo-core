use anyhow::{bail, Result};
use bat_types::policy::{check_access, strip_win_prefix, PathPolicy};
use std::path::Path;

pub struct FsRead {
    policies: Vec<PathPolicy>,
}

impl FsRead {
    pub fn new(policies: Vec<PathPolicy>) -> Self {
        Self { policies }
    }
}

impl super::ToolExecutor for FsRead {
    fn name(&self) -> &str {
        "fs_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Returns the file content as text."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' parameter"))?;

        let path = Path::new(path_str)
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("Cannot resolve path '{}': {}", path_str, e))?;

        if !check_access(&self.policies, &path, false) {
            bail!(
                "Access denied: '{}' is not in any allowed read policy",
                strip_win_prefix(&path).display()
            );
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", strip_win_prefix(&path).display(), e))?;

        // Truncate to 100KB to avoid blowing up context
        if content.len() > 100_000 {
            Ok(format!(
                "{}\n\n[Truncated: file is {} bytes, showing first 100,000 characters]",
                &content[..100_000],
                content.len()
            ))
        } else {
            Ok(content)
        }
    }
}
