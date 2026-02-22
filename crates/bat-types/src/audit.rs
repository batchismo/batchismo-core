use serde::{Deserialize, Serialize};

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub id: i64,
    pub ts: String,
    pub session_id: Option<String>,
    pub level: AuditLevel,
    pub category: AuditCategory,
    pub event: String,
    pub summary: String,
    pub detail_json: Option<String>,
}

/// Severity level for audit entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for AuditLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug => write!(f, "debug"),
            Self::Info => write!(f, "info"),
            Self::Warn => write!(f, "warn"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for AuditLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(format!("unknown audit level: {s}")),
        }
    }
}

/// Category for audit entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditCategory {
    Agent,
    Tool,
    Gateway,
    Ipc,
    Config,
}

impl std::fmt::Display for AuditCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agent => write!(f, "agent"),
            Self::Tool => write!(f, "tool"),
            Self::Gateway => write!(f, "gateway"),
            Self::Ipc => write!(f, "ipc"),
            Self::Config => write!(f, "config"),
        }
    }
}

impl std::str::FromStr for AuditCategory {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "agent" => Ok(Self::Agent),
            "tool" => Ok(Self::Tool),
            "gateway" => Ok(Self::Gateway),
            "ipc" => Ok(Self::Ipc),
            "config" => Ok(Self::Config),
            _ => Err(format!("unknown audit category: {s}")),
        }
    }
}

/// Filter for querying audit logs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditFilter {
    pub level: Option<AuditLevel>,
    pub category: Option<AuditCategory>,
    pub session_id: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Summary statistics for audit logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditStats {
    pub total: i64,
    pub by_level: AuditLevelCounts,
    pub by_category: AuditCategoryCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLevelCounts {
    pub debug: i64,
    pub info: i64,
    pub warn: i64,
    pub error: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditCategoryCounts {
    pub agent: i64,
    pub tool: i64,
    pub gateway: i64,
    pub ipc: i64,
    pub config: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_roundtrip() {
        for level in [AuditLevel::Debug, AuditLevel::Info, AuditLevel::Warn, AuditLevel::Error] {
            let s = level.to_string();
            let parsed: AuditLevel = s.parse().unwrap();
            assert_eq!(level, parsed);
        }
    }

    #[test]
    fn category_roundtrip() {
        for cat in [AuditCategory::Agent, AuditCategory::Tool, AuditCategory::Gateway, AuditCategory::Ipc, AuditCategory::Config] {
            let s = cat.to_string();
            let parsed: AuditCategory = s.parse().unwrap();
            assert_eq!(cat, parsed);
        }
    }
}
