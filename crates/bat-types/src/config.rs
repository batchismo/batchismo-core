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
    /// Ollama endpoint URL — used for local LLM inference (no API key needed).
    /// Default: http://localhost:11434
    #[serde(default)]
    pub ollama_endpoint: Option<String>,
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

    /// Get the Ollama endpoint URL, checking env var first.
    /// Returns the configured endpoint or the default localhost URL.
    pub fn ollama_endpoint(&self) -> String {
        std::env::var("OLLAMA_ENDPOINT").ok()
            .or_else(|| self.ollama_endpoint.clone())
            .unwrap_or_else(|| "http://localhost:11434".to_string())
    }

    /// Determine the LLM provider from a model name.
    pub fn provider_for_model(model: &str) -> LlmProvider {
        if model.starts_with("claude-") {
            LlmProvider::Anthropic
        } else if model.starts_with("gpt-") || model.starts_with("o3-") || model.starts_with("o1-") {
            LlmProvider::OpenAI
        } else {
            // Everything else routes to Ollama (local models like llama3, mistral, phi3, etc.)
            LlmProvider::Ollama
        }
    }
}

/// LLM provider enum for routing inference requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    Anthropic,
    OpenAI,
    Ollama,
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
    /// Model IDs enabled for multi-LLM routing (v0.4.0).
    #[serde(default)]
    pub enabled_models: Vec<String>,
    /// Per-task-type model routing configuration (v0.5.0).
    #[serde(default)]
    pub model_routing: ModelRoutingConfig,
}

/// Per-task-type model routing configuration.
/// Allows different models to be used for different types of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingConfig {
    /// Model for main chat sessions (orchestrator).
    /// If None, falls back to the default model.
    #[serde(default)]
    pub main_chat: Option<String>,
    /// Model for subagent worker sessions.
    /// If None, falls back to the default model.
    #[serde(default)]
    pub subagents: Option<String>,
    /// Model for memory consolidation tasks.
    /// If None, falls back to the default model.
    #[serde(default)]
    pub memory_consolidation: Option<String>,
    /// Routing strategy for automatic model selection.
    #[serde(default)]
    pub routing_strategy: RoutingStrategy,
    /// Daily budget limit in USD. If None, no budget limit.
    #[serde(default)]
    pub daily_budget_usd: Option<f32>,
    /// Per-session budget limit in USD. If None, no session limit.
    #[serde(default)]
    pub session_budget_usd: Option<f32>,
}

/// Routing strategy for automatic model selection based on request classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Always use the most cost-effective model capable of handling the request.
    CostOptimized,
    /// Always use the most capable model available.
    QualityOptimized,
    /// Balance cost and capability based on request complexity.
    Balanced,
    /// Use manual per-task-type routing (existing Track 3 behavior).
    Manual,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        RoutingStrategy::Balanced
    }
}

impl RoutingStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoutingStrategy::CostOptimized => "cost_optimized",
            RoutingStrategy::QualityOptimized => "quality_optimized",
            RoutingStrategy::Balanced => "balanced",
            RoutingStrategy::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "cost_optimized" => Ok(RoutingStrategy::CostOptimized),
            "quality_optimized" => Ok(RoutingStrategy::QualityOptimized),
            "balanced" => Ok(RoutingStrategy::Balanced),
            "manual" => Ok(RoutingStrategy::Manual),
            _ => Err(format!("Invalid routing strategy: {}", s)),
        }
    }
}

impl Default for ModelRoutingConfig {
    fn default() -> Self {
        Self {
            main_chat: None,
            subagents: None,
            memory_consolidation: None,
            routing_strategy: RoutingStrategy::default(),
            daily_budget_usd: None,
            session_budget_usd: None,
        }
    }
}

/// Task types for model routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    MainChat,
    Subagent,
    MemoryConsolidation,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::MainChat => "main_chat",
            TaskType::Subagent => "subagent", 
            TaskType::MemoryConsolidation => "memory_consolidation",
        }
    }
}

/// Model cost and capability information for routing decisions.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub provider: LlmProvider,
    /// Cost per 1K input tokens in USD.
    pub cost_per_1k_input: f32,
    /// Cost per 1K output tokens in USD.
    pub cost_per_1k_output: f32,
    /// Capability score (0-100) - higher is more capable.
    pub capability_score: u8,
    /// Context window size in tokens.
    pub context_window: u32,
    /// Whether this model supports tool use.
    pub supports_tools: bool,
}

impl ModelInfo {
    /// Get model information for known models.
    pub fn for_model(model_id: &str) -> Option<ModelInfo> {
        match model_id {
            // Anthropic models
            "claude-opus-4-6" => Some(ModelInfo {
                id: model_id.to_string(),
                provider: LlmProvider::Anthropic,
                cost_per_1k_input: 0.015,
                cost_per_1k_output: 0.075,
                capability_score: 95,
                context_window: 200_000,
                supports_tools: true,
            }),
            "claude-sonnet-4-6" => Some(ModelInfo {
                id: model_id.to_string(),
                provider: LlmProvider::Anthropic,
                cost_per_1k_input: 0.003,
                cost_per_1k_output: 0.015,
                capability_score: 85,
                context_window: 200_000,
                supports_tools: true,
            }),
            "claude-haiku-4-5-20251001" => Some(ModelInfo {
                id: model_id.to_string(),
                provider: LlmProvider::Anthropic,
                cost_per_1k_input: 0.0008,
                cost_per_1k_output: 0.004,
                capability_score: 70,
                context_window: 200_000,
                supports_tools: true,
            }),
            // OpenAI models
            "gpt-4o" => Some(ModelInfo {
                id: model_id.to_string(),
                provider: LlmProvider::OpenAI,
                cost_per_1k_input: 0.0025,
                cost_per_1k_output: 0.01,
                capability_score: 90,
                context_window: 128_000,
                supports_tools: true,
            }),
            "gpt-4o-mini" => Some(ModelInfo {
                id: model_id.to_string(),
                provider: LlmProvider::OpenAI,
                cost_per_1k_input: 0.00015,
                cost_per_1k_output: 0.0006,
                capability_score: 75,
                context_window: 128_000,
                supports_tools: true,
            }),
            "o3-mini" => Some(ModelInfo {
                id: model_id.to_string(),
                provider: LlmProvider::OpenAI,
                cost_per_1k_input: 0.003,
                cost_per_1k_output: 0.015,
                capability_score: 80,
                context_window: 128_000,
                supports_tools: false, // Reasoning models may not support tools initially
            }),
            _ => {
                // Local/Ollama models - assume free but lower capability
                if ApiKeys::provider_for_model(model_id) == LlmProvider::Ollama {
                    Some(ModelInfo {
                        id: model_id.to_string(),
                        provider: LlmProvider::Ollama,
                        cost_per_1k_input: 0.0, // Local models are free
                        cost_per_1k_output: 0.0,
                        capability_score: 60, // Conservative estimate
                        context_window: 32_000, // Typical for local models
                        supports_tools: true, // Most local models support function calling
                    })
                } else {
                    None
                }
            }
        }
    }

    /// Calculate the cost score for routing (lower is better for cost optimization).
    pub fn cost_score(&self) -> f32 {
        // Weight input and output costs (assume 3:1 input:output ratio)
        (self.cost_per_1k_input * 3.0 + self.cost_per_1k_output) / 4.0
    }

    /// Calculate a balanced score combining cost and capability.
    pub fn balanced_score(&self) -> f32 {
        // Higher capability score and lower cost score is better
        let normalized_capability = self.capability_score as f32 / 100.0;
        let normalized_cost = if self.cost_score() > 0.0 {
            1.0 / (1.0 + self.cost_score() * 100.0) // Invert so lower cost = higher score
        } else {
            1.0 // Free models get max cost score
        };
        
        // Weight 60% capability, 40% cost efficiency
        (normalized_capability * 0.6) + (normalized_cost * 0.4)
    }
}

impl ModelRoutingConfig {
    /// Get the model for a specific task type, falling back to the default if not configured.
    /// This method handles manual routing (Track 3 behavior).
    pub fn model_for_task(&self, task_type: TaskType, default_model: &str) -> String {
        match task_type {
            TaskType::MainChat => {
                self.main_chat.clone().unwrap_or_else(|| default_model.to_string())
            }
            TaskType::Subagent => {
                self.subagents.clone().unwrap_or_else(|| default_model.to_string())
            }
            TaskType::MemoryConsolidation => {
                self.memory_consolidation.clone().unwrap_or_else(|| default_model.to_string())
            }
        }
    }

    /// Get the best model for a request using the configured routing strategy.
    /// Falls back to manual routing if classification or model selection fails.
    pub fn model_for_request(
        &self,
        classification: &crate::classifier::RequestClassification,
        task_type: TaskType,
        available_models: &[String],
        default_model: &str,
        current_daily_usage: f32,
        current_session_usage: f32,
    ) -> String {
        // If using manual routing, use the existing logic
        if self.routing_strategy == RoutingStrategy::Manual {
            return self.model_for_task(task_type, default_model);
        }

        // Check budget constraints and determine if we need to use cheaper models
        let budget_constrained = self.is_budget_constrained(current_daily_usage, current_session_usage);

        // Get model info for available models
        let model_infos: Vec<ModelInfo> = available_models
            .iter()
            .filter_map(|model| ModelInfo::for_model(model))
            .collect();

        if model_infos.is_empty() {
            // Fallback to manual routing if no model info available
            return self.model_for_task(task_type, default_model);
        }

        // Select model based on routing strategy
        let selected_model = match self.routing_strategy {
            RoutingStrategy::CostOptimized => {
                self.select_cost_optimized(&model_infos, classification, budget_constrained)
            }
            RoutingStrategy::QualityOptimized => {
                self.select_quality_optimized(&model_infos, classification, budget_constrained)
            }
            RoutingStrategy::Balanced => {
                self.select_balanced(&model_infos, classification, budget_constrained)
            }
            RoutingStrategy::Manual => {
                // Already handled above, but keep for completeness
                return self.model_for_task(task_type, default_model);
            }
        };

        selected_model.unwrap_or_else(|| default_model.to_string())
    }

    /// Check if budget constraints should downgrade model selection.
    fn is_budget_constrained(&self, daily_usage: f32, session_usage: f32) -> bool {
        if let Some(daily_limit) = self.daily_budget_usd {
            if daily_usage >= daily_limit * 0.8 { // 80% of daily budget
                return true;
            }
        }
        
        if let Some(session_limit) = self.session_budget_usd {
            if session_usage >= session_limit * 0.8 { // 80% of session budget
                return true;
            }
        }
        
        false
    }

    /// Select the most cost-effective model that can handle the request.
    fn select_cost_optimized(
        &self,
        models: &[ModelInfo],
        classification: &crate::classifier::RequestClassification,
        budget_constrained: bool,
    ) -> Option<String> {
        let mut suitable_models: Vec<&ModelInfo> = models
            .iter()
            .filter(|model| self.model_meets_requirements(model, classification))
            .collect();

        if budget_constrained {
            // Prefer free models when budget constrained
            suitable_models.sort_by(|a, b| a.cost_score().partial_cmp(&b.cost_score()).unwrap());
        } else {
            // Sort by cost, then capability as tiebreaker
            suitable_models.sort_by(|a, b| {
                match a.cost_score().partial_cmp(&b.cost_score()) {
                    Some(std::cmp::Ordering::Equal) => {
                        b.capability_score.cmp(&a.capability_score)
                    }
                    Some(order) => order,
                    None => std::cmp::Ordering::Equal,
                }
            });
        }

        suitable_models.first().map(|model| model.id.clone())
    }

    /// Select the most capable model available.
    fn select_quality_optimized(
        &self,
        models: &[ModelInfo],
        classification: &crate::classifier::RequestClassification,
        budget_constrained: bool,
    ) -> Option<String> {
        let mut suitable_models: Vec<&ModelInfo> = models
            .iter()
            .filter(|model| self.model_meets_requirements(model, classification))
            .collect();

        if budget_constrained {
            // When budget constrained, prefer free models but still prioritize capability among them
            suitable_models.sort_by(|a, b| {
                match a.cost_score().partial_cmp(&b.cost_score()) {
                    Some(std::cmp::Ordering::Equal) => {
                        b.capability_score.cmp(&a.capability_score)
                    }
                    Some(order) => order,
                    None => std::cmp::Ordering::Equal,
                }
            });
        } else {
            // Sort by capability score (highest first)
            suitable_models.sort_by(|a, b| b.capability_score.cmp(&a.capability_score));
        }

        suitable_models.first().map(|model| model.id.clone())
    }

    /// Select a balanced model based on cost and capability.
    fn select_balanced(
        &self,
        models: &[ModelInfo],
        classification: &crate::classifier::RequestClassification,
        budget_constrained: bool,
    ) -> Option<String> {
        let mut suitable_models: Vec<&ModelInfo> = models
            .iter()
            .filter(|model| self.model_meets_requirements(model, classification))
            .collect();

        if budget_constrained {
            // When budget constrained, strongly favor cost over capability
            suitable_models.sort_by(|a, b| {
                let score_a = if a.cost_score() == 0.0 { 1.0 } else { 0.3 }; // Free models get high priority
                let score_b = if b.cost_score() == 0.0 { 1.0 } else { 0.3 };
                score_b.partial_cmp(&score_a).unwrap()
            });
        } else {
            // Sort by balanced score (highest first)
            suitable_models.sort_by(|a, b| {
                b.balanced_score().partial_cmp(&a.balanced_score()).unwrap()
            });
        }

        suitable_models.first().map(|model| model.id.clone())
    }

    /// Check if a model meets the requirements of the request classification.
    fn model_meets_requirements(
        &self,
        model: &ModelInfo,
        classification: &crate::classifier::RequestClassification,
    ) -> bool {
        use crate::classifier::{Capability, Complexity};

        // Check tool use requirement
        if classification.capabilities.contains(&Capability::ToolUse) && !model.supports_tools {
            return false;
        }

        // Check context window requirement
        if classification.capabilities.contains(&Capability::LongContext) && model.context_window < 32_000 {
            return false;
        }

        // Check if model capability is sufficient for complexity
        let min_capability_score = match classification.complexity {
            Complexity::Simple => 60,
            Complexity::Moderate => 70,
            Complexity::Complex => 80,
        };

        model.capability_score >= min_capability_score
    }

    /// Set the model for a specific task type.
    pub fn set_model_for_task(&mut self, task_type: TaskType, model: Option<String>) {
        match task_type {
            TaskType::MainChat => self.main_chat = model,
            TaskType::Subagent => self.subagents = model,
            TaskType::MemoryConsolidation => self.memory_consolidation = model,
        }
    }
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
    /// Enable automatic consolidation based on thresholds.
    #[serde(default = "default_true")]
    pub auto_consolidation: bool,
    /// Trigger consolidation after this many sessions.
    #[serde(default = "default_session_threshold")]
    pub consolidation_session_threshold: u32,
    /// Trigger consolidation after this many observations accumulate.
    #[serde(default = "default_observation_threshold")]
    pub consolidation_observation_threshold: u32,
}

fn default_true() -> bool { true }
fn default_session_threshold() -> u32 { 10 }
fn default_observation_threshold() -> u32 { 50 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub memory_limit_mb: u32,
    pub cpu_shares: u32,
    pub max_concurrent_subagents: u32,
    #[serde(default = "default_subagent_timeout")]
    pub subagent_timeout_minutes: u32,
}

fn default_subagent_timeout() -> u32 { 60 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: Option<TelegramChannelConfig>,
    #[serde(default)]
    pub discord: Option<DiscordChannelConfig>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelConfig {
    pub enabled: bool,
    pub bot_token: String,
    /// Discord user IDs allowed to interact with the bot.
    #[serde(default)]
    pub allow_from: Vec<u64>,
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
                enabled_models: vec![],
                model_routing: ModelRoutingConfig::default(),
            },
            gateway: GatewayConfig {
                port: 19000,
                log_level: "info".to_string(),
            },
            memory: MemoryConfig {
                update_mode: "auto".to_string(),
                consolidation_schedule: "daily".to_string(),
                max_memory_file_size_kb: 512,
                auto_consolidation: true,
                consolidation_session_threshold: 10,
                consolidation_observation_threshold: 50,
            },
            sandbox: SandboxConfig {
                memory_limit_mb: 512,
                cpu_shares: 512,
                max_concurrent_subagents: 5,
                subagent_timeout_minutes: 60,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_routing_defaults() {
        let routing = ModelRoutingConfig::default();
        
        // All should default to None (fallback to default model)
        assert_eq!(routing.main_chat, None);
        assert_eq!(routing.subagents, None);
        assert_eq!(routing.memory_consolidation, None);
    }

    #[test]
    fn test_model_for_task_fallback() {
        let routing = ModelRoutingConfig::default();
        let default_model = "claude-sonnet-4-6";
        
        // All tasks should fall back to default model when not configured
        assert_eq!(routing.model_for_task(TaskType::MainChat, default_model), default_model);
        assert_eq!(routing.model_for_task(TaskType::Subagent, default_model), default_model);
        assert_eq!(routing.model_for_task(TaskType::MemoryConsolidation, default_model), default_model);
    }

    #[test]
    fn test_model_for_task_configured() {
        let mut routing = ModelRoutingConfig::default();
        routing.main_chat = Some("claude-opus-4-6".to_string());
        routing.subagents = Some("claude-haiku-4-5-20251001".to_string());
        routing.memory_consolidation = Some("gpt-4o-mini".to_string());
        
        let default_model = "claude-sonnet-4-6";
        
        // Should return configured models
        assert_eq!(routing.model_for_task(TaskType::MainChat, default_model), "claude-opus-4-6");
        assert_eq!(routing.model_for_task(TaskType::Subagent, default_model), "claude-haiku-4-5-20251001");
        assert_eq!(routing.model_for_task(TaskType::MemoryConsolidation, default_model), "gpt-4o-mini");
    }

    #[test]
    fn test_set_model_for_task() {
        let mut routing = ModelRoutingConfig::default();
        
        // Set models for each task type
        routing.set_model_for_task(TaskType::MainChat, Some("claude-opus-4-6".to_string()));
        routing.set_model_for_task(TaskType::Subagent, Some("claude-haiku-4-5-20251001".to_string()));
        routing.set_model_for_task(TaskType::MemoryConsolidation, Some("gpt-4o-mini".to_string()));
        
        assert_eq!(routing.main_chat, Some("claude-opus-4-6".to_string()));
        assert_eq!(routing.subagents, Some("claude-haiku-4-5-20251001".to_string()));
        assert_eq!(routing.memory_consolidation, Some("gpt-4o-mini".to_string()));
        
        // Clear a model (set to None)
        routing.set_model_for_task(TaskType::MainChat, None);
        assert_eq!(routing.main_chat, None);
    }

    #[test]
    fn test_task_type_as_str() {
        assert_eq!(TaskType::MainChat.as_str(), "main_chat");
        assert_eq!(TaskType::Subagent.as_str(), "subagent");
        assert_eq!(TaskType::MemoryConsolidation.as_str(), "memory_consolidation");
    }

    #[test]
    fn test_api_keys_provider_routing() {
        // Test that provider detection still works for routing
        assert_eq!(ApiKeys::provider_for_model("claude-opus-4-6"), LlmProvider::Anthropic);
        assert_eq!(ApiKeys::provider_for_model("gpt-4o"), LlmProvider::OpenAI);
        assert_eq!(ApiKeys::provider_for_model("llama3"), LlmProvider::Ollama);
        assert_eq!(ApiKeys::provider_for_model("mistral"), LlmProvider::Ollama);
    }

    #[test]
    fn test_config_with_model_routing() {
        let mut config = BatConfig::default();
        
        // Configure model routing
        config.agent.model_routing.main_chat = Some("claude-opus-4-6".to_string());
        config.agent.model_routing.subagents = Some("gpt-4o-mini".to_string());
        config.agent.model_routing.memory_consolidation = Some("claude-haiku-4-5-20251001".to_string());
        
        // Verify the routing works
        let default_model = &config.agent.model;
        assert_eq!(config.agent.model_routing.model_for_task(TaskType::MainChat, default_model), "claude-opus-4-6");
        assert_eq!(config.agent.model_routing.model_for_task(TaskType::Subagent, default_model), "gpt-4o-mini");
        assert_eq!(config.agent.model_routing.model_for_task(TaskType::MemoryConsolidation, default_model), "claude-haiku-4-5-20251001");
    }
}
