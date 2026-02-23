//! Memory file management — read/write workspace MD files.

use anyhow::{Context, Result};
use chrono::DateTime;

use bat_types::memory::MemoryFileInfo;

use crate::config;

/// List all MD files in the workspace directory.
pub fn list_memory_files() -> Result<Vec<MemoryFileInfo>> {
    let workspace = config::workspace_path();
    if !workspace.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&workspace)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            let meta = entry.metadata()?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| {
                    let dt: DateTime<chrono::Utc> = t.into();
                    Some(dt.to_rfc3339())
                });

            files.push(MemoryFileInfo {
                name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                size_bytes: meta.len(),
                modified_at: modified,
            });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Read a workspace MD file by name (e.g. "MEMORY.md").
pub fn read_memory_file(name: &str) -> Result<String> {
    validate_filename(name)?;
    let path = config::workspace_path().join(name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))
}

/// Write a workspace MD file. Creates a .bak backup before overwriting.
pub fn write_memory_file(name: &str, content: &str) -> Result<()> {
    validate_filename(name)?;
    let workspace = config::workspace_path();
    std::fs::create_dir_all(&workspace)?;
    let path = workspace.join(name);

    // Backup existing file
    if path.exists() {
        let backup = workspace.join(format!("{name}.bak"));
        std::fs::copy(&path, &backup)
            .with_context(|| format!("Failed to backup {}", path.display()))?;
    }

    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))
}

/// Validate that a filename is safe (no path traversal).
fn validate_filename(name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.is_empty() {
        anyhow::bail!("Invalid memory file name: {name}");
    }
    if !name.ends_with(".md") {
        anyhow::bail!("Memory files must end with .md");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_good_filenames() {
        assert!(validate_filename("MEMORY.md").is_ok());
        assert!(validate_filename("PATTERNS.md").is_ok());
        assert!(validate_filename("my-notes.md").is_ok());
    }

    #[test]
    fn validate_bad_filenames() {
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("foo/bar.md").is_err());
        assert!(validate_filename("").is_err());
        assert!(validate_filename("notes.txt").is_err());
    }
}
