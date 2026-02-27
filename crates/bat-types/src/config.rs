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
    #[serde(default)]
    pub api_keys: ApiKeys,
}

/// Named API keys for external providers.
/// Each feature looks up the key it needs from here.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    /// Anthropic API key — used for Claude models (agent LLM).
    #[serde(default)]
    pub anthropic: Option<String>,
    /// OpenAI API key — used for Whisper (STT), TTS, and future OpenAI models.
    #[serde(default)]
    pub openai: Option<String>,
    /// ElevenLabs API key — used for ElevenLabs TTS voices.
    #[serde(default)]
    pub elevenlabs: Option<String>,
}

impl ApiKeys {
    /// Get the Anthropic key, checking env var first.
    pub fn anthropic_key(&self) -> Option<String> {
        std::env::var("ANTHROPIC_API_KEY").ok().or_else(|| self.anthropic.clone())
    }

    /// Get the OpenAI key, checking env var first.
    pub fn openai_key(&self) -> Option<String> {
        std::env::var("OPENAI_API_KEY").ok().or_else(|| self.openai.clone())
    }

    /// Get the ElevenLabs key, checking env var first.
    pub fn elevenlabs_key(&self) -> Option<String> {
        std::env::var("ELEVENLABS_API_KEY").ok().or_else(|| self.elevenlabs.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub thinking_level: String,
    /// Legacy API key field — migrated to api_keys.anthropic on load.
    /// Kept for backwards compatibility with existing config files.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Free-text personality prompt injected into the system prompt.
    #[serde(default)]
    pub personality_prompt: Option<String>,
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
/// Voice features are automatically disabled if the required API key is not present
/// in the api_keys registry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    /// TTS provider: "openai" or "elevenlabs".
    #[serde(default = "default_tts_provider")]
    pub tts_provider: String,
    /// OpenAI TTS voice name.
    /// Available voices: alloy, ash, ballad, coral, echo, fable, nova, onyx, sage, shimmer.
    #[serde(default = "default_openai_voice")]
    pub openai_voice: String,
    /// OpenAI TTS model (e.g., "gpt-4o-mini-tts", "tts-1", "tts-1-hd").
    #[serde(default = "default_openai_tts_model")]
    pub openai_tts_model: String,
    /// ElevenLabs voice ID.
    #[serde(default)]
    pub elevenlabs_voice_id: Option<String>,
    /// Enable voice responses (TTS). Only works if the required API key is present.
    #[serde(default)]
    pub tts_enabled: bool,
    /// Enable voice input transcription (STT via Whisper). Only works if OpenAI key is present.
    #[serde(default)]
    pub stt_enabled: bool,

    // Legacy fields — kept for backwards compat, ignored in favor of api_keys
    #[serde(default, skip_serializing)]
    pub openai_api_key: Option<String>,
    #[serde(default, skip_serializing)]
    pub elevenlabs_api_key: Option<String>,
}

impl VoiceConfig {
    /// Check if TTS can actually run given available API keys.
    pub fn tts_available(&self, keys: &ApiKeys) -> bool {
        if !self.tts_enabled { return false; }
        match self.tts_provider.as_str() {
            "elevenlabs" => keys.elevenlabs_key().is_some() && self.elevenlabs_voice_id.is_some(),
            _ => keys.openai_key().is_some(), // "openai" default
        }
    }

    /// Check if STT can actually run given available API keys.
    pub fn stt_available(&self, keys: &ApiKeys) -> bool {
        self.stt_enabled && keys.openai_key().is_some()
    }
}

fn default_tts_provider() -> String { "openai".to_string() }
fn default_openai_voice() -> String { "alloy".to_string() }
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
                personality_prompt: None,
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
            api_keys: ApiKeys::default(),
        }
    }
}

impl BatConfig {
    /// Migrate legacy api_key fields into the api_keys registry.
    /// Called after loading config from disk.
    pub fn migrate_legacy_keys(&mut self) {
        // agent.api_key → api_keys.anthropic
        if self.api_keys.anthropic.is_none() {
            if let Some(ref key) = self.agent.api_key {
                self.api_keys.anthropic = Some(key.clone());
            }
        }
        // voice.openai_api_key → api_keys.openai
        if self.api_keys.openai.is_none() {
            if let Some(ref key) = self.voice.openai_api_key {
                self.api_keys.openai = Some(key.clone());
            }
        }
        // voice.elevenlabs_api_key → api_keys.elevenlabs
        if self.api_keys.elevenlabs.is_none() {
            if let Some(ref key) = self.voice.elevenlabs_api_key {
                self.api_keys.elevenlabs = Some(key.clone());
            }
        }
    }
}
