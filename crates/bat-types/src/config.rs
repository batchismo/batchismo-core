use serde::{Deserialize, Serialize};

use crate::policy::PathPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatConfig {
    pub agent: AgentConfig,
    pub gateway: GatewayConfig,
    pub memory: MemoryConfig,
    pub sandbox: SandboxConfig,
    pub paths: Vec<PathPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub thinking_level: String,
    /// Optional API key stored in config (env var takes priority at runtime).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Tools disabled by the user via the Settings panel.
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// Whether the onboarding wizard has been completed.
    #[serde(default)]
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub update_mode: String,
    pub consolidation_schedule: String,
    pub max_memory_file_size_kb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub memory_limit_mb: u32,
    pub cpu_shares: u32,
    pub max_concurrent_subagents: u32,
}

impl Default for BatConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig {
                name: "Aria".to_string(),
                model: "claude-opus-4-6".to_string(),
                thinking_level: "medium".to_string(),
                api_key: None,
                disabled_tools: vec![],
                onboarding_complete: false,
            },
            gateway: GatewayConfig {
                port: 19000,
                log_level: "info".to_string(),
            },
            memory: MemoryConfig {
                update_mode: "auto".to_string(),
                consolidation_schedule: "daily".to_string(),
                max_memory_file_size_kb: 512,
            },
            sandbox: SandboxConfig {
                memory_limit_mb: 512,
                cpu_shares: 512,
                max_concurrent_subagents: 5,
            },
            paths: vec![],
        }
    }
}
