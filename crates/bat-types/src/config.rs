use serde::{Deserialize, Serialize};

use crate::policy::PathPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatConfig {
    pub agent: AgentConfig,
    pub gateway: GatewayConfig,
    pub memory: MemoryConfig,
    pub sandbox: SandboxConfig,
    pub paths: Vec<PathPolicy>,
    #[serde(default)]
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub voice: VoiceConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: Option<TelegramChannelConfig>,
}

/// Voice I/O configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    /// Enable voice responses (TTS).
    #[serde(default)]
    pub tts_enabled: bool,
    /// TTS provider: "openai" or "elevenlabs".
    #[serde(default = "default_tts_provider")]
    pub tts_provider: String,
    /// OpenAI TTS voice name (e.g., "nova", "alloy", "shimmer").
    #[serde(default = "default_openai_voice")]
    pub openai_voice: String,
    /// OpenAI TTS model (e.g., "gpt-4o-mini-tts", "tts-1", "tts-1-hd").
    #[serde(default = "default_openai_tts_model")]
    pub openai_tts_model: String,
    /// ElevenLabs API key (if using ElevenLabs).
    #[serde(default)]
    pub elevenlabs_api_key: Option<String>,
    /// ElevenLabs voice ID.
    #[serde(default)]
    pub elevenlabs_voice_id: Option<String>,
    /// Enable voice input transcription (STT via Whisper).
    #[serde(default)]
    pub stt_enabled: bool,
    /// OpenAI API key for Whisper (falls back to agent API key if not set).
    #[serde(default)]
    pub openai_api_key: Option<String>,
}

fn default_tts_provider() -> String { "openai".to_string() }
fn default_openai_voice() -> String { "nova".to_string() }
fn default_openai_tts_model() -> String { "gpt-4o-mini-tts".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChannelConfig {
    pub enabled: bool,
    pub bot_token: String,
    /// Telegram user IDs allowed to interact with the bot.
    #[serde(default)]
    pub allow_from: Vec<i64>,
}

impl Default for BatConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig {
                name: "Aria".to_string(),
                model: "claude-sonnet-4-6".to_string(),
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
            channels: ChannelsConfig::default(),
            voice: VoiceConfig::default(),
        }
    }
}
