use serde::{Deserialize, Serialize};

/// A single behavioral observation (metadata only, never raw conversation content).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub id: i64,
    pub ts: String,
    pub session_id: Option<String>,
    pub kind: ObservationKind,
    pub key: String,
    pub value: Option<String>,
    pub count: i64,
}

/// Types of observations the system records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    ToolUse,
    PathAccess,
    UserCorrection,
    TaskPattern,
    Preference,
}

impl std::fmt::Display for ObservationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolUse => write!(f, "tool_use"),
            Self::PathAccess => write!(f, "path_access"),
            Self::UserCorrection => write!(f, "user_correction"),
            Self::TaskPattern => write!(f, "task_pattern"),
            Self::Preference => write!(f, "preference"),
        }
    }
}

impl std::str::FromStr for ObservationKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tool_use" => Ok(Self::ToolUse),
            "path_access" => Ok(Self::PathAccess),
            "user_correction" => Ok(Self::UserCorrection),
            "task_pattern" => Ok(Self::TaskPattern),
            "preference" => Ok(Self::Preference),
            _ => Err(format!("unknown observation kind: {s}")),
        }
    }
}

/// Filter for querying observations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationFilter {
    pub kind: Option<ObservationKind>,
    pub since: Option<String>,
    pub key: Option<String>,
    pub limit: Option<i64>,
}

/// Aggregated observation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummary {
    pub total_observations: i64,
    pub total_sessions: i64,
    pub top_tools: Vec<(String, i64)>,
    pub top_paths: Vec<(String, i64)>,
    pub last_consolidation: Option<String>,
}

/// Info about a memory/workspace MD file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFileInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

/// A line in a simple diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: DiffKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Added,
    Removed,
    Context,
}

/// Compute a simple line-level diff between old and new text.
pub fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut result = Vec::new();

    // Simple LCS-based diff would be ideal, but for MVP use a basic approach:
    // mark removed lines from old, added lines from new, matching lines as context.
    let mut old_idx = 0;
    let mut new_idx = 0;

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        match (old_lines.get(old_idx), new_lines.get(new_idx)) {
            (Some(o), Some(n)) if o == n => {
                result.push(DiffLine {
                    kind: DiffKind::Context,
                    content: o.to_string(),
                });
                old_idx += 1;
                new_idx += 1;
            }
            (Some(o), Some(n)) => {
                // Check if old line appears later in new (it was moved/kept)
                let old_in_new = new_lines[new_idx..].contains(o);
                let new_in_old = old_lines[old_idx..].contains(n);

                if old_in_new && !new_in_old {
                    // New line was inserted
                    result.push(DiffLine {
                        kind: DiffKind::Added,
                        content: n.to_string(),
                    });
                    new_idx += 1;
                } else if new_in_old && !old_in_new {
                    // Old line was removed
                    result.push(DiffLine {
                        kind: DiffKind::Removed,
                        content: o.to_string(),
                    });
                    old_idx += 1;
                } else {
                    // Line changed
                    result.push(DiffLine {
                        kind: DiffKind::Removed,
                        content: o.to_string(),
                    });
                    result.push(DiffLine {
                        kind: DiffKind::Added,
                        content: n.to_string(),
                    });
                    old_idx += 1;
                    new_idx += 1;
                }
            }
            (Some(o), None) => {
                result.push(DiffLine {
                    kind: DiffKind::Removed,
                    content: o.to_string(),
                });
                old_idx += 1;
            }
            (None, Some(n)) => {
                result.push(DiffLine {
                    kind: DiffKind::Added,
                    content: n.to_string(),
                });
                new_idx += 1;
            }
            (None, None) => break,
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_kind_roundtrip() {
        for kind in [
            ObservationKind::ToolUse,
            ObservationKind::PathAccess,
            ObservationKind::UserCorrection,
            ObservationKind::TaskPattern,
            ObservationKind::Preference,
        ] {
            let s = kind.to_string();
            let parsed: ObservationKind = s.parse().unwrap();
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn line_diff_basic() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline2_modified\nline3\nline4";
        let diff = line_diff(old, new);

        assert!(diff.iter().any(|d| d.kind == DiffKind::Context && d.content == "line1"));
        assert!(diff.iter().any(|d| d.kind == DiffKind::Removed && d.content == "line2"));
        assert!(diff.iter().any(|d| d.kind == DiffKind::Added && d.content == "line2_modified"));
        assert!(diff.iter().any(|d| d.kind == DiffKind::Added && d.content == "line4"));
    }

    #[test]
    fn line_diff_identical() {
        let text = "hello\nworld";
        let diff = line_diff(text, text);
        assert!(diff.iter().all(|d| d.kind == DiffKind::Context));
        assert_eq!(diff.len(), 2);
    }
}
