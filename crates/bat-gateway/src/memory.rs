//! Memory file management — read/write workspace MD files.

use anyhow::{Context, Result};
use chrono::DateTime;

use bat_types::memory::MemoryFileInfo;

use crate::config;

/// Max number of timestamped backups to keep per file.
const MAX_BACKUPS: usize = 10;

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
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            // Skip backup files
            if name.contains(".md.") {
                continue;
            }
            let meta = entry.metadata()?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| {
                    let dt: DateTime<chrono::Utc> = t.into();
                    Some(dt.to_rfc3339())
                });

            files.push(MemoryFileInfo {
                name,
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

/// Write a workspace MD file. Creates a timestamped backup before overwriting.
/// Keeps up to MAX_BACKUPS per file, pruning oldest.
pub fn write_memory_file(name: &str, content: &str) -> Result<()> {
    validate_filename(name)?;
    let workspace = config::workspace_path();
    std::fs::create_dir_all(&workspace)?;
    let path = workspace.join(name);

    // Create timestamped backup of existing file
    if path.exists() {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
        let backup_name = format!("{}.{}.bak", name, ts);
        let backup_path = workspace.join(&backup_name);
        std::fs::copy(&path, &backup_path)
            .with_context(|| format!("Failed to backup {}", path.display()))?;

        // Also maintain the simple .bak for backwards compat
        let simple_bak = workspace.join(format!("{name}.bak"));
        let _ = std::fs::copy(&path, &simple_bak);

        // Prune old backups
        prune_backups(&workspace, name)?;
    }

    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))
}

/// List backup history for a memory file.
pub fn list_memory_history(name: &str) -> Result<Vec<MemoryBackupInfo>> {
    validate_filename(name)?;
    let workspace = config::workspace_path();
    let prefix = format!("{name}.");
    let mut backups = Vec::new();

    if !workspace.exists() {
        return Ok(backups);
    }

    for entry in std::fs::read_dir(&workspace)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with(&prefix) && fname.ends_with(".bak") && fname != format!("{name}.bak") {
            // Extract timestamp: NAME.md.TIMESTAMP.bak
            let ts_part = fname
                .strip_prefix(&prefix)
                .and_then(|s| s.strip_suffix(".bak"))
                .unwrap_or("");
            if !ts_part.is_empty() {
                let meta = entry.metadata()?;
                backups.push(MemoryBackupInfo {
                    timestamp: ts_part.to_string(),
                    size_bytes: meta.len(),
                });
            }
        }
    }

    // Sort newest first
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(backups)
}

/// Read a specific backup version.
pub fn read_memory_backup(name: &str, timestamp: &str) -> Result<String> {
    validate_filename(name)?;
    // Validate timestamp format to prevent path traversal
    if timestamp.contains('/') || timestamp.contains('\\') || timestamp.contains("..") {
        anyhow::bail!("Invalid timestamp");
    }
    let workspace = config::workspace_path();
    let backup_name = format!("{name}.{timestamp}.bak");
    let path = workspace.join(&backup_name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("Backup not found: {}", backup_name))
}

/// Restore a backup to the current file.
pub fn restore_memory_backup(name: &str, timestamp: &str) -> Result<()> {
    let backup_content = read_memory_backup(name, timestamp)?;
    write_memory_file(name, &backup_content)
}

/// Prune backups to keep only MAX_BACKUPS most recent.
fn prune_backups(workspace: &std::path::Path, name: &str) -> Result<()> {
    let prefix = format!("{name}.");
    let mut backup_files: Vec<(String, std::path::PathBuf)> = Vec::new();

    for entry in std::fs::read_dir(workspace)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with(&prefix) && fname.ends_with(".bak") && fname != format!("{name}.bak") {
            let ts_part = fname
                .strip_prefix(&prefix)
                .and_then(|s| s.strip_suffix(".bak"))
                .unwrap_or("")
                .to_string();
            if !ts_part.is_empty() {
                backup_files.push((ts_part, entry.path()));
            }
        }
    }

    // Sort by timestamp descending (newest first)
    backup_files.sort_by(|a, b| b.0.cmp(&a.0));

    // Remove excess
    for (_, path) in backup_files.iter().skip(MAX_BACKUPS) {
        let _ = std::fs::remove_file(path);
    }

    Ok(())
}

/// Info about a memory backup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBackupInfo {
    pub timestamp: String,
    pub size_bytes: u64,
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
