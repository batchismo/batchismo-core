use anyhow::{bail, Result};
use bat_types::policy::{check_access, strip_win_prefix, PathPolicy};
use std::path::Path;

pub struct FsList {
    policies: Vec<PathPolicy>,
}

impl FsList {
    pub fn new(policies: Vec<PathPolicy>) -> Self {
        Self { policies }
    }
}

impl super::ToolExecutor for FsList {
    fn name(&self) -> &str {
        "fs_list"
    }

    fn description(&self) -> &str {
        "List the contents of a directory. Returns file names, types, and sizes."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the directory to list"
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

        if !path.is_dir() {
            bail!("'{}' is not a directory", strip_win_prefix(&path).display());
        }

        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs())
                });

            entries.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "is_dir": metadata.is_dir(),
                "size": metadata.len(),
                "modified": modified,
            }));
        }

        Ok(serde_json::to_string_pretty(&entries)?)
    }
}
