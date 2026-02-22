use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AccessLevel {
    ReadOnly,
    ReadWrite,
    WriteOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathPolicy {
    pub path: PathBuf,
    pub access: AccessLevel,
    pub recursive: bool,
    pub description: Option<String>,
}

/// Strip the Windows extended-length path prefix (`\\?\`) so that
/// canonicalized paths compare correctly against user-supplied paths.
pub fn strip_win_prefix(p: &Path) -> &Path {
    p.to_str()
        .and_then(|s| s.strip_prefix(r"\\?\"))
        .map(Path::new)
        .unwrap_or(p)
}

/// Normalize a path for comparison: strip the `\\?\` prefix and lowercase
/// on Windows so that path checks are case-insensitive (matching the OS).
fn normalize(p: &Path) -> PathBuf {
    let stripped = strip_win_prefix(p);
    if cfg!(windows) {
        PathBuf::from(stripped.to_string_lossy().to_lowercase())
    } else {
        stripped.to_path_buf()
    }
}

impl PathPolicy {
    pub fn allows(&self, target: &Path, write: bool) -> bool {
        let target = normalize(target);
        let policy_path = normalize(&self.path);
        let matches = if self.recursive {
            target.starts_with(&policy_path)
        } else {
            target.parent() == Some(policy_path.as_path())
        };
        if !matches {
            return false;
        }
        match (self.access, write) {
            (AccessLevel::ReadWrite, _) => true,
            (AccessLevel::ReadOnly, false) => true,
            (AccessLevel::WriteOnly, true) => true,
            _ => false,
        }
    }
}

/// Check if any policy allows the given path for the given operation.
pub fn check_access(policies: &[PathPolicy], target: &Path, write: bool) -> bool {
    policies.iter().any(|p| p.allows(target, write))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(path: &str, access: AccessLevel, recursive: bool) -> PathPolicy {
        PathPolicy {
            path: PathBuf::from(path),
            access,
            recursive,
            description: None,
        }
    }

    #[test]
    fn read_write_allows_both() {
        let p = policy("/tmp/test", AccessLevel::ReadWrite, true);
        assert!(p.allows(Path::new("/tmp/test/file.txt"), false));
        assert!(p.allows(Path::new("/tmp/test/file.txt"), true));
    }

    #[test]
    fn read_only_denies_write() {
        let p = policy("/tmp/test", AccessLevel::ReadOnly, true);
        assert!(p.allows(Path::new("/tmp/test/file.txt"), false));
        assert!(!p.allows(Path::new("/tmp/test/file.txt"), true));
    }

    #[test]
    fn write_only_denies_read() {
        let p = policy("/tmp/test", AccessLevel::WriteOnly, true);
        assert!(!p.allows(Path::new("/tmp/test/file.txt"), false));
        assert!(p.allows(Path::new("/tmp/test/file.txt"), true));
    }

    #[test]
    fn non_recursive_only_direct_children() {
        let p = policy("/tmp/test", AccessLevel::ReadWrite, false);
        assert!(p.allows(Path::new("/tmp/test/file.txt"), false));
        assert!(!p.allows(Path::new("/tmp/test/sub/file.txt"), false));
    }

    #[test]
    fn outside_path_denied() {
        let p = policy("/tmp/test", AccessLevel::ReadWrite, true);
        assert!(!p.allows(Path::new("/tmp/other/file.txt"), false));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn case_insensitive_match() {
        let p = policy(r"C:\Users\Test\Documents", AccessLevel::ReadWrite, true);
        // Different casing should still match
        assert!(p.allows(Path::new(r"C:\users\test\documents\file.txt"), true));
        assert!(p.allows(Path::new(r"C:\USERS\TEST\DOCUMENTS\file.txt"), false));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn case_insensitive_non_recursive() {
        let p = policy(r"C:\Users\Test\Docs", AccessLevel::ReadWrite, false);
        assert!(p.allows(Path::new(r"C:\users\test\docs\file.txt"), true));
        assert!(!p.allows(Path::new(r"C:\users\test\docs\sub\file.txt"), true));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn win_prefix_stripped() {
        let p = policy(r"C:\Users\Test", AccessLevel::ReadWrite, true);
        assert!(p.allows(Path::new(r"\\?\C:\Users\Test\file.txt"), true));
    }

    #[test]
    fn check_access_multiple_policies() {
        let policies = vec![
            policy("/tmp/read", AccessLevel::ReadOnly, true),
            policy("/tmp/write", AccessLevel::WriteOnly, true),
        ];
        assert!(check_access(&policies, Path::new("/tmp/read/file.txt"), false));
        assert!(!check_access(&policies, Path::new("/tmp/read/file.txt"), true));
        assert!(check_access(&policies, Path::new("/tmp/write/file.txt"), true));
        assert!(!check_access(&policies, Path::new("/tmp/other/file.txt"), false));
    }
}
