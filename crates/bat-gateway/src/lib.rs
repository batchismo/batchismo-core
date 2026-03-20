pub mod channels;
pub mod classifier;
pub mod config;
pub mod consolidation;
pub mod correction;
pub mod cost_governor;
pub mod db;
pub mod events;
pub mod ipc;
pub mod memory;
pub mod process_manager;
pub mod sandbox;
pub mod session;
pub mod skills;
pub mod stt;
pub mod reflection;
pub mod system_prompt;
pub mod tts;

pub use events::EventBus;

use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use bat_types::{
    audit::{AuditCategory, AuditEntry, AuditFilter, AuditLevel, AuditStats},
    config::BatConfig,
    ipc::{AgentToGateway, GatewayToAgent},
    memory::{MemoryFileInfo, Observation, ObservationFilter, ObservationSummary, ObservationKind},
    message::Message,
    policy::{AccessLevel, PathPolicy},
    session::SessionMeta,
};

use db::Database;
use session::SessionManager;

/// Metadata about a registered tool (for the Settings UI).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub icon: String,
    pub enabled: bool,
}

/// An ElevenLabs voice entry (returned to the UI for voice picker).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevenLabsVoice {
    pub voice_id: String,
    pub name: String,
    pub category: String,
    pub preview_url: Option<String>,
}

/// An Ollama model entry (returned to the UI for model picker).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub modified_at: Option<String>,
    pub parameter_size: Option<String>,
}

/// Shared state for routing agent questions through the active Telegram channel.
/// Created once per `start_channels()` call and threaded through `run_agent_turn`.
struct TelegramState {
    /// Channel for sending messages out to Telegram.
    outbound: tokio::sync::mpsc::UnboundedSender<channels::telegram::OutboundMessage>,
    /// chat_id of the currently active Telegram conversation (0 = none yet).
    active_chat_id: Arc<std::sync::Mutex<i64>>,
    /// If a subagent question is pending a Telegram reply, the answer goes here.
    pending_question: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<String>>>>,
}

/// Message router for inter-session communication.
/// Handles routing Questions from sub-agents to their parent orchestrators,
/// and routing Answers and Instructions back to the appropriate sessions.
struct MessageRouter {
    /// Map from session_id to the message sender for that session.
    /// When a session is running, it registers its message queue here.
    session_queues: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<GatewayToAgent>>>>,
}

impl MessageRouter {
    fn new() -> Self {
        Self {
            session_queues: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a message queue for an active session.
    fn register_session(&self, session_id: Uuid, tx: mpsc::UnboundedSender<GatewayToAgent>) {
        let mut queues = self.session_queues.write().unwrap();
        queues.insert(session_id, tx);
        debug!("Registered message queue for session {}", session_id);
    }

    /// Unregister a session when it completes.
    fn unregister_session(&self, session_id: Uuid) {
        let mut queues = self.session_queues.write().unwrap();
        queues.remove(&session_id);
        debug!("Unregistered message queue for session {}", session_id);
    }

    /// Route a question from a sub-agent to its parent orchestrator.
    fn route_question(&self, from_session_id: Uuid, parent_session_id: Uuid, question_id: String, question: String, context: String) -> Result<()> {
        let queues = self.session_queues.read().unwrap();
        if let Some(tx) = queues.get(&parent_session_id) {
            let msg = GatewayToAgent::UserMessage {
                content: format!("🤖 **Sub-agent question from session {}:**\n\n❓ {}\n\n**Context:** {}\n\n[Question ID: {}]", 
                    from_session_id.to_string()[..8].to_string(),
                    question,
                    context,
                    question_id
                ),
                images: vec![],
            };
            tx.send(msg).map_err(|e| anyhow::anyhow!("Failed to route question: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Parent orchestrator session {} not found or not active", parent_session_id))
        }
    }

    /// Route an answer from an orchestrator back to a waiting sub-agent.
    fn route_answer(&self, to_session_id: Uuid, question_id: String, answer: String) -> Result<()> {
        let queues = self.session_queues.read().unwrap();
        if let Some(tx) = queues.get(&to_session_id) {
            let msg = GatewayToAgent::Answer {
                question_id,
                answer,
            };
            tx.send(msg).map_err(|e| anyhow::anyhow!("Failed to route answer: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Target session {} not found or not active", to_session_id))
        }
    }

    /// Route an instruction from an orchestrator to a sub-agent.
    fn route_instruction(&self, to_session_id: Uuid, instruction_id: String, content: String) -> Result<()> {
        let queues = self.session_queues.read().unwrap();
        if let Some(tx) = queues.get(&to_session_id) {
            let msg = GatewayToAgent::Instruction {
                instruction_id,
                content,
            };
            tx.send(msg).map_err(|e| anyhow::anyhow!("Failed to route instruction: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Target session {} not found or not active", to_session_id))
        }
    }
}

/// The central gateway — owns the database, session state, and event bus.
/// The Tauri shell holds an `Arc<Gateway>` in `AppState`.
pub struct Gateway {
    session_manager: Arc<SessionManager>,
    db: Arc<Database>,
    config: Arc<RwLock<BatConfig>>,
    event_bus: EventBus,
    process_manager: process_manager::ProcessManager,
    /// Key of the currently active session.
    active_session_key: Arc<RwLock<String>>,
    /// Last consolidation diffs (for diff view in UI).
    last_consolidation_diffs: Arc<RwLock<Vec<consolidation::FileDiff>>>,
    /// Message router for inter-session communication.
    message_router: Arc<MessageRouter>,
    /// Skill manager for loading and hot-reloading skills.
    skill_manager: Arc<skills::SkillManager>,
}

impl Gateway {
    /// Create a new gateway. This does NOT start the agent process.
    pub fn new(config: BatConfig, db: Arc<Database>) -> Result<Self> {
        let default_model = config.agent.model.clone();
        let session_manager =
            Arc::new(SessionManager::new(Arc::clone(&db), default_model));
        let event_bus = EventBus::new();

        // Initialize skill manager
        let workspace_path = crate::config::workspace_path();
        let skill_manager = Arc::new(skills::SkillManager::new(workspace_path)?);

        // Create example skills if the skills directory is empty
        let skills_count = skill_manager.list_skills().len();
        if skills_count == 0 {
            let workspace_path = crate::config::workspace_path();
            if let Err(e) = skills::create_example_skills(&workspace_path) {
                warn!("Failed to create example skills: {}", e);
            } else {
                // Reload skills after creating examples
                if let Err(e) = skill_manager.scan_and_load_skills() {
                    warn!("Failed to reload skills after creating examples: {}", e);
                }
            }
        }

        Ok(Self {
            session_manager,
            db,
            config: Arc::new(RwLock::new(config)),
            event_bus,
            process_manager: process_manager::ProcessManager::new(),
            active_session_key: Arc::new(RwLock::new("main".to_string())),
            last_consolidation_diffs: Arc::new(RwLock::new(Vec::new())),
            message_router: Arc::new(MessageRouter::new()),
            skill_manager,
        })
    }

    /// Start channel adapters (Telegram, etc.) based on config.
    pub fn start_channels(&self) {
        let cfg = self.config.read().unwrap().clone();

        // Telegram
        if let Some(ref tg_cfg) = cfg.channels.telegram {
            if tg_cfg.enabled && !tg_cfg.bot_token.is_empty() {
                let stt_available = cfg.voice.stt_available(&cfg.api_keys);
                let stt_api_key = cfg.api_keys.openai_key().unwrap_or_default();
                let tg_config = channels::telegram::TelegramConfig {
                    bot_token: tg_cfg.bot_token.clone(),
                    allow_from: tg_cfg.allow_from.clone(),
                    stt_enabled: stt_available,
                    stt_api_key,
                };
                let bot_token_for_typing = tg_cfg.bot_token.clone();
                let (mut inbound_rx, outbound_tx) = channels::telegram::TelegramAdapter::start(tg_config);

                // Route inbound Telegram messages to the gateway
                let event_bus = self.event_bus.clone();
                let db = Arc::clone(&self.db);
                let session_manager = Arc::clone(&self.session_manager);
                let config = Arc::clone(&self.config);
                let proc_mgr = self.process_manager.clone();
                let outbound = outbound_tx.clone();
                let typing_client = reqwest::Client::new();

                // Shared state for question routing — used by both the inbound loop
                // and handle_subagent_action when a subagent calls ask_orchestrator.
                let active_chat_id_shared = Arc::new(std::sync::Mutex::new(0i64));
                let pending_question_shared: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<String>>>> =
                    Arc::new(std::sync::Mutex::new(None));
                let telegram_state = Arc::new(TelegramState {
                    outbound: outbound_tx.clone(),
                    active_chat_id: Arc::clone(&active_chat_id_shared),
                    pending_question: Arc::clone(&pending_question_shared),
                });

                tokio::spawn(async move {
                    // Track which chat_id to respond to (shared with TelegramState)
                    let active_chat_id = active_chat_id_shared;
                    let pending_question = pending_question_shared;

                    // Inbound message routing
                    while let Some(mut msg) = inbound_rx.recv().await {
                        *active_chat_id.lock().unwrap() = msg.chat_id;

                        // Transcribe voice messages if STT is enabled
                        if let Some(ref voice_text) = msg.voice_text {
                            // Voice was already transcribed by the adapter
                            if msg.text.is_empty() {
                                msg.text = voice_text.clone();
                            }
                        }

                        if msg.text.is_empty() {
                            continue; // Skip empty messages
                        }

                        // If a subagent question is pending, this reply is the answer —
                        // don't start a new agent turn.
                        {
                            let answer_tx = pending_question.lock().unwrap().take();
                            if let Some(tx) = answer_tx {
                                info!("Telegram reply routing as answer to pending agent question");
                                let _ = tx.send(msg.text.clone());
                                continue;
                            }
                        }

                        info!("Telegram inbound from user {}: {}", msg.user_id, &msg.text[..msg.text.len().min(50)]);

                        // Get the active session and route the message
                        let active_key = "main"; // Telegram routes to main session
                        let model = config.read().unwrap().agent.model.clone();
                        let session = match db.get_session_by_key(active_key) {
                            Ok(Some(s)) => s,
                            Ok(None) => match db.create_session(active_key, &model) {
                                Ok(s) => s,
                                Err(e) => { error!("Failed to create session: {e}"); continue; }
                            },
                            Err(e) => { error!("DB error: {e}"); continue; }
                        };

                        let history = session_manager.get_history(session.id).unwrap_or_default();

                        let user_msg = bat_types::message::Message::user(session.id, &msg.text);
                        let _ = session_manager.append_message(&user_msg);

                        let (cfg_model, disabled_tools, agent_env) = {
                            let cfg = config.read().unwrap();
                            (cfg.agent.model.clone(), cfg.agent.disabled_tools.clone(), build_agent_env(&cfg))
                        };
                        let system_prompt = {
                            let cfg = config.read().unwrap();
                            let policies = db.get_path_policies().unwrap_or_default();
                            // Telegram sessions are always main/orchestrator sessions
                            // TODO: Pass skills section from skill manager
                            crate::system_prompt::build_orchestrator_prompt(&cfg, &policies, None).unwrap_or_default()
                        };
                        let path_policies = db.get_path_policies().unwrap_or_default();

                        let eb = event_bus.clone();
                        let sm = Arc::new(SessionManager::new(Arc::clone(&db), cfg_model.clone()));
                        let db2 = Arc::clone(&db);
                        let pm = proc_mgr.clone();
                        let cfg2 = Arc::clone(&config);
                        let tg_state = Arc::clone(&telegram_state);

                        // Start typing indicator (repeats every 4s until cancelled)
                        let typing_cancel = channels::telegram::spawn_typing_loop(
                            typing_client.clone(),
                            bot_token_for_typing.clone(),
                            msg.chat_id,
                        );

                        // Create a dedicated mpsc channel for this turn's replies
                        // so we don't rely on the broadcast EventBus (which can lag and drop).
                        let (turn_tx, turn_rx) = tokio::sync::mpsc::unbounded_channel::<AgentToGateway>();

                        // Spawn per-turn reply sender
                        let outbound_for_turn = outbound.clone();
                        let chat_id_for_turn = *active_chat_id.lock().unwrap();
                        let config_for_turn = Arc::clone(&config);
                        tokio::spawn(async move {
                            handle_telegram_turn_events(
                                turn_rx,
                                outbound_for_turn,
                                chat_id_for_turn,
                                config_for_turn,
                            ).await;
                        });

                        let outbound_for_error = outbound.clone();
                        let error_chat_id = msg.chat_id;
                        tokio::spawn(async move {
                            let result = run_agent_turn(
                                session.id, cfg_model, system_prompt, history, msg.text,
                                vec![],  // Telegram messages don't carry images yet
                                path_policies, disabled_tools, agent_env,
                                eb, sm, db2, pm, cfg2,
                                "main".to_string(),  // Telegram sessions are main/orchestrator
                                Some(tg_state),
                                Some(turn_tx),
                            ).await;
                            // Cancel typing indicator
                            drop(typing_cancel);
                            if let Err(e) = result {
                                error!("Telegram agent turn failed: {e}");
                                // Fix 3: send error message back to Telegram
                                let _ = outbound_for_error.send(
                                    channels::telegram::OutboundMessage {
                                        chat_id: error_chat_id,
                                        text: format!("⚠️ Agent error: {e}"),
                                        reply_to: None,
                                        voice_data: None,
                                    },
                                );
                            }
                        });
                    }
                });

                info!("Telegram channel adapter started");
            }
        }

        // Discord
        if let Some(ref discord_cfg) = cfg.channels.discord {
            if discord_cfg.enabled && !discord_cfg.bot_token.is_empty() {
                let discord_config = channels::discord::DiscordConfig {
                    bot_token: discord_cfg.bot_token.clone(),
                    allow_from: discord_cfg.allow_from.clone(),
                };
                
                let (_inbound_rx, _outbound_tx) = channels::discord::DiscordAdapter::start(discord_config);
                
                // TODO: Implement Discord message routing similar to Telegram
                // This would require:
                // 1. Route inbound Discord messages to the gateway
                // 2. Handle outbound responses back to Discord
                // 3. Support for Discord-specific features (embeds, reactions, etc.)
                
                info!("Discord channel adapter started (stub implementation)");
            }
        }
    }

    /// Subscribe to the event bus for streaming events from agents.
    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentToGateway> {
        self.event_bus.subscribe()
    }

    /// Get a reference to the process manager.
    pub fn process_manager(&self) -> &process_manager::ProcessManager {
        &self.process_manager
    }

    // ─── Commands exposed to Tauri ────────────────────────────────────────────

    /// Send a user message. Persists the message, spawns the agent in the
    /// background, and returns immediately. Events arrive via the event bus.
    /// Get or create a session by key.
    fn get_or_create_session(&self, key: &str) -> Result<SessionMeta> {
        if key == "main" {
            self.session_manager.get_or_create_main()
        } else {
            match self.db.get_session_by_key(key)? {
                Some(s) => Ok(s),
                None => {
                    let model = self.config.read().unwrap().agent.model.clone();
                    self.db.create_session(key, &model)
                }
            }
        }
    }

    /// Clean up timed-out and archivable subagent sessions.
    pub fn cleanup_subagents(&self) {
        let timeout_minutes = {
            let cfg = self.config.read().unwrap();
            cfg.sandbox.subagent_timeout_minutes
        };

        // Time out running subagents that exceeded the timeout
        if let Ok(timed_out) = self.db.get_timed_out_subagents(timeout_minutes) {
            for (id, key) in &timed_out {
                info!("Timing out subagent: key={key}");
                let _ = self.db.update_subagent_status(
                    *id,
                    bat_types::session::SubagentStatus::TimedOut,
                    Some("Subagent exceeded timeout limit"),
                );
                self.event_bus.send(AgentToGateway::AuditLog {
                    level: "warn".into(),
                    category: "agent".into(),
                    event: "subagent_timeout".into(),
                    summary: format!("[Subagent: {key} — timed out after {timeout_minutes}m]"),
                    detail_json: None,
                });
            }
        }

        // Archive old completed sessions (24 hours)
        if let Ok(archivable) = self.db.get_archivable_subagents(24) {
            for id in &archivable {
                let _ = self.db.update_subagent_status(
                    *id,
                    bat_types::session::SubagentStatus::Archived,
                    None,
                );
            }
        }
    }

    pub async fn send_user_message(
        &self,
        content: &str,
        images: Vec<bat_types::message::ImageAttachment>,
    ) -> Result<()> {
        // Run subagent cleanup on each new message
        self.cleanup_subagents();

        let active_key = self.active_session_key();
        let session = self.get_or_create_session(&active_key)?;

        // Collect history BEFORE persisting the new user message
        // (the agent will append the user message itself, so we avoid duplicates)
        let history = self
            .session_manager
            .get_history(session.id)
            .context("Failed to get session history")?;

        // Persist the user message (with images if any)
        let user_msg = if images.is_empty() {
            Message::user(session.id, content)
        } else {
            Message::user_with_images(session.id, content, images.clone())
        };
        self.session_manager
            .append_message(&user_msg)
            .context("Failed to persist user message")?;

        let path_policies = self
            .db
            .get_path_policies()
            .context("Failed to get path policies")?;

        // Classify the request for intelligent routing
        let classification = classifier::RequestClassifier::classify(content, &images);
        
        // Read config once under lock
        let (model, disabled_tools, agent_env) = {
            let cfg = self.config.read().unwrap();
            
            // Use intelligent model routing if not manual
            let task_model = if cfg.agent.model_routing.routing_strategy == bat_types::config::RoutingStrategy::Manual {
                // Use existing Track 3 manual routing
                cfg.agent.model_routing.model_for_task(
                    bat_types::config::TaskType::MainChat,
                    &cfg.agent.model,
                )
            } else {
                // Use intelligent routing based on request classification
                let available_models = cfg.agent.enabled_models.iter()
                    .chain(std::iter::once(&cfg.agent.model))
                    .cloned()
                    .collect::<Vec<_>>();
                
                // Get current usage for budget consideration
                let cost_governor = cost_governor::CostGovernor::new(Arc::clone(&self.db));
                let daily_usage = cost_governor.get_daily_usage().unwrap_or_else(|_| {
                    cost_governor::DailyUsage {
                        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                        total_cost_usd: 0.0,
                        model_usage: std::collections::HashMap::new(),
                    }
                });
                let session_usage = cost_governor.get_session_usage(session.id)
                    .unwrap_or_else(|_| cost_governor::SessionUsage {
                        session_id: session.id,
                        total_cost_usd: 0.0,
                        model_usage: std::collections::HashMap::new(),
                        started_at: chrono::Utc::now(),
                    });
                
                cfg.agent.model_routing.model_for_request(
                    &classification,
                    bat_types::config::TaskType::MainChat,
                    &available_models,
                    &cfg.agent.model,
                    daily_usage.total_cost_usd,
                    session_usage.total_cost_usd,
                )
            };
            
            (
                task_model,
                cfg.agent.disabled_tools.clone(),
                build_agent_env(&cfg),
            )
        };

        // Validate that the required API key is available for the chosen model's provider
        validate_provider_key(&model, &agent_env)?;

        let system_prompt = {
            let cfg = self.config.read().unwrap();
            let skills_section = self.get_skills_prompt_section();
            let skills_section_opt = if skills_section.is_empty() { None } else { Some(skills_section) };
            
            // Use orchestrator prompt for main sessions, worker prompt for subagents
            match session.kind {
                bat_types::session::SessionKind::Main => {
                    system_prompt::build_orchestrator_prompt(&cfg, &path_policies, skills_section_opt)
                        .context("Failed to build orchestrator prompt")?
                }
                bat_types::session::SessionKind::Subagent { ref task, .. } => {
                    system_prompt::build_worker_prompt(&cfg, &path_policies, task, skills_section_opt.clone())
                        .context("Failed to build worker prompt")?
                }
            }
        };

        let event_bus = self.event_bus.clone();
        let session_manager = Arc::clone(&self.session_manager);
        let content_owned = content.to_string();
        let images_owned = images;

        // Log the user message event with routing decision
        self.log_event(
            AuditLevel::Info,
            AuditCategory::Gateway,
            "user_message",
            &format!(
                "User message received ({} chars) → {} [{}|{}|{:?}]", 
                content.len(), 
                model,
                classification.complexity.as_str(),
                classification.domain.as_str(),
                classification.capabilities.iter().map(|c| c.as_str()).collect::<Vec<_>>()
            ),
            Some(&session.id.to_string()),
            None,
        );

        let db = Arc::clone(&self.db);
        let proc_mgr = self.process_manager.clone();
        let gw_config = Arc::clone(&self.config);

        // Spawn the agent turn in a background task
        tokio::spawn(async move {
            event_bus.send(AgentToGateway::TextDelta {
                content: String::new(), // Signal "thinking started"
            });

            // Determine session kind based on session key
            let session_kind = if active_key == "main" {
                "main".to_string()
            } else {
                // Check if this is a subagent session
                match session.kind {
                    bat_types::session::SessionKind::Main => "main".to_string(),
                    bat_types::session::SessionKind::Subagent { .. } => "subagent".to_string(),
                }
            };

            let is_main = session_kind == "main";
            let user_msg_for_reflection = if is_main { Some(content_owned.clone()) } else { None };
            let anthropic_key_for_reflection = if is_main { agent_env.anthropic_key.clone() } else { None };
            let model_for_reflection = if is_main { Some(model.clone()) } else { None };

            // Clone for post-turn usage (reflection + auto-consolidation)
            let db_post = Arc::clone(&db);
            let gw_config_post = Arc::clone(&gw_config);

            if let Err(e) = run_agent_turn(
                session.id,
                model,
                system_prompt,
                history,
                content_owned,
                images_owned,
                path_policies,
                disabled_tools,
                agent_env,
                event_bus.clone(),
                session_manager.clone(),
                db,
                proc_mgr,
                gw_config,
                session_kind,
                None, // No Telegram state for UI-originated turns
                None, // No dedicated Telegram reply channel
            )
            .await
            {
                error!("Agent turn failed: {}", e);
                event_bus.send(AgentToGateway::Error {
                    message: format!("Agent error: {e}"),
                });
            } else if let (Some(user_msg), Some(key), Some(mdl)) = (user_msg_for_reflection, anthropic_key_for_reflection, model_for_reflection) {
                // Post-turn reflection: check if anything is worth remembering
                info!("Running post-turn reflection for main session");
                match session_manager.get_history(session.id) {
                    Ok(updated_history) => {
                        if let Some(last_msg) = updated_history.iter().rev().find(|m| m.role == bat_types::message::Role::Assistant) {
                            info!("Reflection: found assistant response ({} chars), calling maybe_remember", last_msg.content.len());
                            if let Err(e) = reflection::maybe_remember(&key, &mdl, &user_msg, &last_msg.content).await {
                                warn!("Reflection failed (non-fatal): {e}");
                            }
                        } else {
                            warn!("Reflection: no assistant message found in history ({} messages total)", updated_history.len());
                        }
                    }
                    Err(e) => {
                        warn!("Reflection: failed to get history: {e}");
                    }
                }
            }

            // Check if automatic consolidation should trigger
            {
                let (auto_enabled, obs_threshold, session_threshold) = {
                    let cfg = gw_config_post.read().unwrap();
                    (
                        cfg.memory.auto_consolidation,
                        cfg.memory.consolidation_observation_threshold as i64,
                        cfg.memory.consolidation_session_threshold as i64,
                    )
                };
                if auto_enabled {
                    let last = db_post.get_metadata("last_consolidation").unwrap_or(None);
                    let obs_count = db_post.count_observations_since(last.as_deref()).unwrap_or(0);
                    let session_count = db_post.count_sessions_since(last.as_deref()).unwrap_or(0);

                    if obs_count >= obs_threshold || session_count >= session_threshold {
                        info!("Auto-consolidation triggered: {} obs, {} sessions", obs_count, session_count);
                        let (api_key_c, model_c) = {
                            let cfg = gw_config_post.read().unwrap();
                            // Use task-specific model routing for memory consolidation
                            let consolidation_model = cfg.agent.model_routing.model_for_task(
                                bat_types::config::TaskType::MemoryConsolidation,
                                &cfg.agent.model,
                            );
                            (
                                cfg.api_keys.anthropic_key().unwrap_or_default(),
                                consolidation_model,
                            )
                        };
                        if !api_key_c.is_empty() {
                            match consolidation::run_consolidation(&db_post, &event_bus, &api_key_c, &model_c).await {
                                Ok(r) => info!("Auto-consolidation: {} files updated", r.files_updated.len()),
                                Err(e) => warn!("Auto-consolidation failed: {e}"),
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Get the full message history for the main session.
    pub async fn get_main_history(&self) -> Result<Vec<Message>> {
        let active_key = self.active_session_key();
        let session = self.get_or_create_session(&active_key)?;
        self.session_manager.get_history(session.id)
    }

    /// Get the active session metadata.
    pub async fn get_main_session(&self) -> Result<SessionMeta> {
        let active_key = self.active_session_key();
        self.get_or_create_session(&active_key)
    }

    /// List all user sessions.
    pub fn list_sessions(&self) -> Result<Vec<SessionMeta>> {
        self.db.list_sessions()
    }

    /// Create a new named session.
    pub fn create_named_session(&self, name: &str) -> Result<SessionMeta> {
        let model = self.config.read().unwrap().agent.model.clone();
        self.db.create_session(name, &model)
    }

    /// Switch active session by key. Returns the session.
    pub fn switch_session(&self, key: &str) -> Result<SessionMeta> {
        let model = self.config.read().unwrap().agent.model.clone();
        // Get or create the session
        let session = match self.db.get_session_by_key(key)? {
            Some(s) => s,
            None => self.db.create_session(key, &model)?,
        };
        *self.active_session_key.write().unwrap() = key.to_string();
        Ok(session)
    }

    /// Get the active session key.
    pub fn active_session_key(&self) -> String {
        self.active_session_key.read().unwrap().clone()
    }

    /// Delete a session by key.
    pub fn delete_session(&self, key: &str) -> Result<()> {
        if key == "main" {
            return Err(anyhow::anyhow!("Cannot delete the main session"));
        }
        let session = self.db.get_session_by_key(key)?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {key}"))?;
        self.db.delete_session(session.id)?;
        // If we deleted the active session, switch back to main
        if *self.active_session_key.read().unwrap() == key {
            *self.active_session_key.write().unwrap() = "main".to_string();
        }
        Ok(())
    }

    /// Rename a session.
    pub fn rename_session(&self, old_key: &str, new_key: &str) -> Result<()> {
        if old_key == "main" {
            return Err(anyhow::anyhow!("Cannot rename the main session"));
        }
        let session = self.db.get_session_by_key(old_key)?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {old_key}"))?;
        self.db.rename_session(session.id, new_key)?;
        if *self.active_session_key.read().unwrap() == old_key {
            *self.active_session_key.write().unwrap() = new_key.to_string();
        }
        Ok(())
    }

    /// Get token usage statistics.
    pub fn get_usage_stats(&self) -> Result<bat_types::usage::UsageStats> {
        self.db.get_usage_stats()
    }

    /// Cancel a running subagent by session ID.
    pub async fn cancel_subagent(&self, session_id: uuid::Uuid) -> Result<()> {
        self.db.update_subagent_status(
            session_id,
            bat_types::session::SubagentStatus::Cancelled,
            Some("Cancelled by user"),
        )?;
        Ok(())
    }

    /// Pause a running subagent by session ID.
    pub async fn pause_subagent(&self, session_id: uuid::Uuid) -> Result<()> {
        self.db.update_subagent_status(
            session_id,
            bat_types::session::SubagentStatus::Paused,
            Some("Paused by user"),
        )?;
        // TODO: Send pause signal to the running agent process
        Ok(())
    }

    /// Resume a paused subagent by session ID.
    pub async fn resume_subagent(&self, session_id: uuid::Uuid, instructions: Option<String>) -> Result<()> {
        self.db.update_subagent_status(
            session_id,
            bat_types::session::SubagentStatus::Running,
            instructions.as_deref(),
        )?;
        // TODO: Send resume signal with optional new instructions to the agent process
        Ok(())
    }

    /// Get all subagent sessions for the main session.
    pub async fn get_subagents(&self) -> Result<Vec<bat_types::session::SubagentInfo>> {
        let session = self.session_manager.get_or_create_main()?;
        self.db.get_subagents(session.id)
    }

    /// Get all configured path policies.
    pub async fn get_path_policies(&self) -> Result<Vec<PathPolicy>> {
        self.db.get_path_policies()
    }

    /// Get all configured path policies (sync version for TUI init).
    pub fn get_path_policies_sync(&self) -> Result<Vec<PathPolicy>> {
        self.db.get_path_policies()
    }

    /// Add a new path policy.
    pub async fn add_path_policy(&self, path: &str, access: &str, recursive: bool) -> Result<()> {
        let access_level = match access {
            "read-only" => AccessLevel::ReadOnly,
            "read-write" => AccessLevel::ReadWrite,
            "write-only" => AccessLevel::WriteOnly,
            other => anyhow::bail!("Unknown access level: {other}"),
        };
        let policy = PathPolicy {
            id: None,
            path: path.into(),
            access: access_level,
            recursive,
            description: None,
        };
        self.db.add_path_policy(&policy)
    }

    /// Delete a path policy by its path string.
    pub async fn delete_path_policy(&self, id: i64) -> Result<()> {
        self.db.delete_path_policy(id)
    }

    /// Get a clone of the current config.
    pub fn get_config(&self) -> BatConfig {
        self.config.read().unwrap().clone()
    }

    /// Update config in-memory and persist to disk.
    /// Also writes personality_prompt to IDENTITY.md if set.
    pub fn update_config(&self, new_config: BatConfig) -> Result<()> {
        config::save_config(&new_config)?;

        // Sync personality prompt to IDENTITY.md
        let identity_path = config::workspace_path().join("IDENTITY.md");
        if let Some(ref prompt) = new_config.agent.personality_prompt {
            if !prompt.trim().is_empty() {
                std::fs::write(&identity_path, prompt)?;
            }
        }

        *self.config.write().unwrap() = new_config;
        Ok(())
    }

    /// Returns info about all known tools (name, display name, description, enabled state).
    pub fn get_tools_info(&self) -> Vec<ToolInfo> {
        let disabled = self.config.read().unwrap().agent.disabled_tools.clone();
        vec![
            ToolInfo {
                name: "fs_read".to_string(),
                display_name: "Read File".to_string(),
                description: "Read the contents of a file on disk.".to_string(),
                icon: "📄".to_string(),
                enabled: !disabled.contains(&"fs_read".to_string()),
            },
            ToolInfo {
                name: "fs_write".to_string(),
                display_name: "Write File".to_string(),
                description: "Write or create files on disk.".to_string(),
                icon: "✏️".to_string(),
                enabled: !disabled.contains(&"fs_write".to_string()),
            },
            ToolInfo {
                name: "fs_list".to_string(),
                display_name: "List Directory".to_string(),
                description: "List the contents of a directory.".to_string(),
                icon: "📁".to_string(),
                enabled: !disabled.contains(&"fs_list".to_string()),
            },
            ToolInfo {
                name: "web_fetch".to_string(),
                display_name: "Fetch URL".to_string(),
                description: "Fetch content from a web URL.".to_string(),
                icon: "🌐".to_string(),
                enabled: !disabled.contains(&"web_fetch".to_string()),
            },
            ToolInfo {
                name: "shell_run".to_string(),
                display_name: "Run Command".to_string(),
                description: "Execute a shell command and return output.".to_string(),
                icon: "⚡".to_string(),
                enabled: !disabled.contains(&"shell_run".to_string()),
            },
            ToolInfo {
                name: "exec_run".to_string(),
                display_name: "Exec Run".to_string(),
                description: "Start a process (foreground or background).".to_string(),
                icon: "🖥️".to_string(),
                enabled: !disabled.contains(&"exec_run".to_string()),
            },
            ToolInfo {
                name: "exec_output".to_string(),
                display_name: "Exec Output".to_string(),
                description: "Get output from a background process.".to_string(),
                icon: "📋".to_string(),
                enabled: !disabled.contains(&"exec_output".to_string()),
            },
            ToolInfo {
                name: "exec_write".to_string(),
                display_name: "Exec Write".to_string(),
                description: "Write to stdin of a background process.".to_string(),
                icon: "✍️".to_string(),
                enabled: !disabled.contains(&"exec_write".to_string()),
            },
            ToolInfo {
                name: "exec_kill".to_string(),
                display_name: "Exec Kill".to_string(),
                description: "Kill a running background process.".to_string(),
                icon: "🛑".to_string(),
                enabled: !disabled.contains(&"exec_kill".to_string()),
            },
            ToolInfo {
                name: "exec_list".to_string(),
                display_name: "Exec List".to_string(),
                description: "List all managed processes.".to_string(),
                icon: "📊".to_string(),
                enabled: !disabled.contains(&"exec_list".to_string()),
            },
            ToolInfo {
                name: "app_open".to_string(),
                display_name: "Open App/File".to_string(),
                description: "Open a file, URL, or app with the system default handler.".to_string(),
                icon: "🚀".to_string(),
                enabled: !disabled.contains(&"app_open".to_string()),
            },
            ToolInfo {
                name: "system_info".to_string(),
                display_name: "System Info".to_string(),
                description: "Get OS, CPU, memory, and disk information.".to_string(),
                icon: "💻".to_string(),
                enabled: !disabled.contains(&"system_info".to_string()),
            },
        ]
    }

    /// Toggle a tool on or off. Persists to config on disk.
    pub fn toggle_tool(&self, name: &str, enabled: bool) -> Result<()> {
        let saved = {
            let mut cfg = self.config.write().unwrap();
            if enabled {
                cfg.agent.disabled_tools.retain(|t| t != name);
            } else if !cfg.agent.disabled_tools.contains(&name.to_string()) {
                cfg.agent.disabled_tools.push(name.to_string());
            }
            cfg.clone()
        };
        config::save_config(&saved)?;
        Ok(())
    }

    /// Build and return the current system prompt (for preview in Settings).
    pub fn get_system_prompt(&self) -> Result<String> {
        let cfg = self.config.read().unwrap();
        let path_policies = self.db.get_path_policies()?;
        let skills_section = self.get_skills_prompt_section();
        let skills_section_opt = if skills_section.is_empty() { None } else { Some(skills_section) };
        
        // Default to orchestrator prompt for this API
        system_prompt::build_orchestrator_prompt(&cfg, &path_policies, skills_section_opt)
    }

    // ─── Onboarding ───────────────────────────────────────────────────────

    /// Check if onboarding has been completed.
    pub fn is_onboarding_complete(&self) -> bool {
        self.config.read().unwrap().agent.onboarding_complete
    }

    // ─── Ollama ────────────────────────────────────────────────────────────

    /// Check Ollama connectivity and list available models.
    pub async fn ollama_list_models(&self) -> Result<Vec<OllamaModel>> {
        let endpoint = self.config.read().unwrap().api_keys.ollama_endpoint();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
        let resp = client.get(&url).send().await
            .context("Failed to connect to Ollama")?;
        if !resp.status().is_success() {
            anyhow::bail!("Ollama returned status {}", resp.status());
        }
        let body: serde_json::Value = resp.json().await?;
        let models = body.get("models").and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|m| {
                    let name = m.get("name")?.as_str()?.to_string();
                    let size = m.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                    let modified_at = m.get("modified_at").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let parameter_size = m.get("details")
                        .and_then(|d| d.get("parameter_size"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(OllamaModel { name, size, modified_at, parameter_size })
                }).collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    /// Check if Ollama is reachable.
    pub async fn ollama_status(&self) -> Result<bool> {
        let endpoint = self.config.read().unwrap().api_keys.ollama_endpoint();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()?;
        let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
        match client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Validate an Anthropic API key by making a minimal API call.
    pub async fn validate_api_key(key: &str) -> Result<()> {
        Self::validate_anthropic_key(key).await
    }

    pub async fn validate_anthropic_key(key: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-haiku-4-5-20251001",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await
            .context("Failed to reach Anthropic API")?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                anyhow::bail!("Invalid API key")
            } else {
                anyhow::bail!("API error ({}): {}", status, body)
            }
        }
    }

    pub async fn validate_openai_key(key: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.openai.com/v1/models")
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
            .context("Failed to reach OpenAI API")?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                anyhow::bail!("Invalid API key")
            } else {
                anyhow::bail!("API error ({}): {}", status, body)
            }
        }
    }

    /// Complete onboarding: set name, API key, path policies, write IDENTITY.md.
    pub async fn complete_onboarding(
        &self,
        name: String,
        api_key: String,
        openai_api_key: Option<String>,
        folders: Vec<(String, String, bool)>, // (path, access, recursive)
    ) -> Result<()> {
        // Update config
        {
            let mut cfg = self.config.write().unwrap();
            cfg.agent.name = name.clone();
            
            // Determine provider and set appropriate keys and model
            if !api_key.is_empty() {
                // Anthropic provider
                cfg.agent.api_key = Some(api_key.clone()); // legacy compat
                cfg.api_keys.anthropic = Some(api_key);
                // Keep current Claude model or set default
                if !cfg.agent.model.starts_with("claude-") {
                    cfg.agent.model = "claude-sonnet-4-6".to_string();
                }
            } else if let Some(ref oai_key) = openai_api_key {
                if !oai_key.is_empty() {
                    // OpenAI provider
                    cfg.api_keys.openai = Some(oai_key.clone());
                    cfg.agent.model = "gpt-4o".to_string();
                    // Auto-enable voice features when OpenAI key is provided
                    cfg.voice.tts_enabled = true;
                    cfg.voice.stt_enabled = true;
                }
            } else {
                // Ollama provider - no API key needed
                cfg.agent.model = "llama3.2".to_string(); // Default Ollama model
            }
            
            cfg.agent.onboarding_complete = true;
            config::save_config(&cfg)?;
        }

        // Add path policies
        for (path, access, recursive) in &folders {
            self.add_path_policy(path, access, *recursive).await?;
        }

        // Write IDENTITY.md
        let workspace_dir = config::workspace_path();
        std::fs::create_dir_all(&workspace_dir)?;
        let identity_path = workspace_dir.join("IDENTITY.md");
        let identity_content = format!(
            "# Identity\n\nName: {}\n\nYou are {}, a personal AI agent running on this computer. \
             You help your user by reading and writing files, answering questions, and completing tasks.\n",
            name, name
        );
        std::fs::write(&identity_path, identity_content)?;

        self.log_event(
            AuditLevel::Info,
            AuditCategory::Config,
            "onboarding_complete",
            &format!("Onboarding completed — agent named '{name}'"),
            None,
            None,
        );

        Ok(())
    }

    // ─── Voice / ElevenLabs ─────────────────────────────────────────────

    /// Fetch available ElevenLabs voices using the configured API key.
    pub async fn fetch_elevenlabs_voices(&self) -> Result<Vec<ElevenLabsVoice>> {
        let api_key = self.config.read().unwrap().api_keys.elevenlabs_key()
            .ok_or_else(|| anyhow::anyhow!("No ElevenLabs API key configured"))?;

        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", &api_key)
            .send()
            .await
            .context("Failed to reach ElevenLabs API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs API error ({status}): {body}");
        }

        let body: serde_json::Value = resp.json().await?;
        let voices = body["voices"].as_array()
            .map(|arr| {
                arr.iter().filter_map(|v| {
                    Some(ElevenLabsVoice {
                        voice_id: v["voice_id"].as_str()?.to_string(),
                        name: v["name"].as_str()?.to_string(),
                        category: v["category"].as_str().unwrap_or("premade").to_string(),
                        preview_url: v["preview_url"].as_str().map(|s| s.to_string()),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(voices)
    }

    // ─── Audit / Observability ────────────────────────────────────────────

    /// Log a structured audit event. Writes to SQLite and broadcasts to the event bus.
    pub fn log_event(
        &self,
        level: AuditLevel,
        category: AuditCategory,
        event: &str,
        summary: &str,
        session_id: Option<&str>,
        detail_json: Option<&str>,
    ) {
        let ts = chrono::Utc::now().to_rfc3339();

        // Persist to DB (best-effort — don't crash the gateway over logging)
        if let Err(e) = self.db.insert_audit_log(
            &ts,
            session_id,
            level,
            category,
            event,
            summary,
            detail_json,
        ) {
            error!("Failed to write audit log: {}", e);
        }

        // Broadcast to UI subscribers
        self.event_bus.send(AgentToGateway::AuditLog {
            level: level.to_string(),
            category: category.to_string(),
            event: event.to_string(),
            summary: summary.to_string(),
            detail_json: detail_json.map(|s| s.to_string()),
        });
    }

    /// Query audit log entries.
    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        self.db.query_audit_log(filter)
    }

    /// Get audit log summary statistics.
    pub fn get_audit_stats(&self) -> Result<AuditStats> {
        self.db.get_audit_stats()
    }

    // ─── Memory / Observations ────────────────────────────────────────────

    /// Record a behavioral observation.
    pub fn record_observation(
        &self,
        kind: ObservationKind,
        key: &str,
        value: Option<&str>,
        session_id: Option<&str>,
    ) {
        if let Err(e) = self.db.record_observation(kind, key, value, session_id) {
            error!("Failed to record observation: {}", e);
        }
    }

    /// Query observations.
    pub fn get_observations(&self, filter: &ObservationFilter) -> Result<Vec<Observation>> {
        self.db.get_observations(filter)
    }

    /// Get observation summary stats.
    pub fn get_observation_summary(&self) -> Result<ObservationSummary> {
        self.db.get_observation_summary()
    }

    /// List workspace memory files.
    pub fn list_memory_files(&self) -> Result<Vec<MemoryFileInfo>> {
        memory::list_memory_files()
    }

    /// Read a memory file.
    pub fn read_memory_file(&self, name: &str) -> Result<String> {
        memory::read_memory_file(name)
    }

    /// Trigger memory consolidation (LLM-powered update of MEMORY.md + PATTERNS.md).
    pub async fn trigger_consolidation(&self) -> Result<consolidation::ConsolidationResult> {
        let (api_key, model) = {
            let cfg = self.config.read().unwrap();
            let key = cfg.api_keys.anthropic_key()
                .ok_or_else(|| anyhow::anyhow!("No Anthropic API key configured"))?;
            // Use task-specific model routing for memory consolidation
            let consolidation_model = cfg.agent.model_routing.model_for_task(
                bat_types::config::TaskType::MemoryConsolidation,
                &cfg.agent.model,
            );
            (key, consolidation_model)
        };

        let result = consolidation::run_consolidation(&self.db, &self.event_bus, &api_key, &model).await?;

        // Store diffs for the UI
        *self.last_consolidation_diffs.write().unwrap() = result.diffs.clone();

        Ok(result)
    }

    /// Get the diffs from the last consolidation.
    pub fn get_last_consolidation_diffs(&self) -> Vec<consolidation::FileDiff> {
        self.last_consolidation_diffs.read().unwrap().clone()
    }

    /// Get a memory diff for a specific file (compares .bak with current).
    pub fn get_memory_diff(&self, name: &str) -> Result<Vec<bat_types::memory::DiffLine>> {
        // First check last consolidation diffs
        let diffs = self.last_consolidation_diffs.read().unwrap();
        for d in diffs.iter() {
            if d.name == name {
                return Ok(bat_types::memory::line_diff(&d.old_content, &d.new_content));
            }
        }
        // Fallback: diff .bak vs current
        let current = memory::read_memory_file(name)?;
        let bak_name = format!("{name}.bak");
        let workspace = config::workspace_path();
        let bak_path = workspace.join(&bak_name);
        if bak_path.exists() {
            let old = std::fs::read_to_string(&bak_path)?;
            Ok(bat_types::memory::line_diff(&old, &current))
        } else {
            // No backup = all lines are "added"
            Ok(bat_types::memory::line_diff("", &current))
        }
    }

    /// Check if automatic consolidation should be triggered, and run it if so.
    pub async fn maybe_auto_consolidate(&self) {
        let (enabled, session_threshold, obs_threshold) = {
            let cfg = self.config.read().unwrap();
            (
                cfg.memory.auto_consolidation,
                cfg.memory.consolidation_session_threshold as i64,
                cfg.memory.consolidation_observation_threshold as i64,
            )
        };
        if !enabled {
            return;
        }

        let last = self.db.get_metadata("last_consolidation").unwrap_or(None);
        let obs_count = self.db.count_observations_since(last.as_deref()).unwrap_or(0);
        let session_count = self.db.count_sessions_since(last.as_deref()).unwrap_or(0);

        if obs_count >= obs_threshold || session_count >= session_threshold {
            info!("Auto-consolidation triggered: {} observations, {} sessions since last consolidation",
                obs_count, session_count);
            match self.trigger_consolidation().await {
                Ok(result) => {
                    info!("Auto-consolidation complete: {} files updated", result.files_updated.len());
                }
                Err(e) => {
                    warn!("Auto-consolidation failed: {e}");
                }
            }
        }
    }

    /// List backup history for a memory file.
    pub fn get_memory_history(&self, name: &str) -> Result<Vec<memory::MemoryBackupInfo>> {
        memory::list_memory_history(name)
    }

    /// Restore a memory file from a backup.
    pub fn restore_memory_backup(&self, name: &str, timestamp: &str) -> Result<()> {
        memory::restore_memory_backup(name, timestamp)?;
        self.log_event(
            AuditLevel::Info,
            AuditCategory::Config,
            "memory_restore",
            &format!("Memory file restored: {name} from {timestamp}"),
            None,
            None,
        );
        Ok(())
    }

    /// Read a specific backup version (for preview).
    pub fn preview_memory_backup(&self, name: &str, timestamp: &str) -> Result<String> {
        memory::read_memory_backup(name, timestamp)
    }

    /// Write a memory file.
    pub fn write_memory_file(&self, name: &str, content: &str) -> Result<()> {
        memory::write_memory_file(name, content)?;
        self.log_event(
            AuditLevel::Info,
            AuditCategory::Config,
            "memory_update",
            &format!("Memory file updated: {name}"),
            None,
            None,
        );
        Ok(())
    }

    // ─── Skills ────────────────────────────────────────────────────────

    /// List all loaded skills.
    pub fn list_skills(&self) -> Vec<skills::Skill> {
        self.skill_manager.list_skills()
    }

    /// Get a specific skill by name.
    pub fn get_skill(&self, name: &str) -> Option<skills::Skill> {
        self.skill_manager.get_skill(name)
    }

    /// Enable or disable a skill.
    pub fn set_skill_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        self.skill_manager.set_skill_enabled(name, enabled)
    }

    /// Get all enabled skills.
    pub fn get_enabled_skills(&self) -> Vec<skills::Skill> {
        self.skill_manager.get_enabled_skills()
    }

    /// Subscribe to skill events.
    pub fn subscribe_skill_events(&self) -> broadcast::Receiver<skills::SkillEvent> {
        self.skill_manager.subscribe_events()
    }

    /// Get the skills section for the system prompt.
    pub fn get_skills_prompt_section(&self) -> String {
        self.skill_manager.build_skills_prompt_section()
    }

    /// Get all tools defined by skills.
    pub fn get_skill_tools(&self) -> Vec<skills::SkillTool> {
        self.skill_manager.get_skill_tools()
    }
}

// ─── Agent turn runner ────────────────────────────────────────────────────────

/// Helper to log an audit event from the agent turn (fire-and-forget to DB + event bus).
fn audit(
    db: &Database,
    event_bus: &EventBus,
    level: AuditLevel,
    category: AuditCategory,
    event: &str,
    summary: &str,
    session_id: Option<&str>,
    detail_json: Option<&str>,
) {
    let ts = chrono::Utc::now().to_rfc3339();
    let _ = db.insert_audit_log(&ts, session_id, level, category, event, summary, detail_json);
    event_bus.send(AgentToGateway::AuditLog {
        level: level.to_string(),
        category: category.to_string(),
        event: event.to_string(),
        summary: summary.to_string(),
        detail_json: detail_json.map(|s| s.to_string()),
    });
}

/// Handle subagent-related actions synchronously (the actual subagent runs in a spawned task).
fn handle_subagent_action(
    action: bat_types::ipc::ProcessAction,
    session_id: Uuid,
    db: Arc<Database>,
    event_bus: EventBus,
    proc_mgr: process_manager::ProcessManager,
    config: Arc<RwLock<BatConfig>>,
    telegram_state: Option<Arc<TelegramState>>,
) -> bat_types::ipc::ProcessResult {
    use bat_types::ipc::{ProcessAction, ProcessResult};
    use bat_types::session::SubagentStatus;

    match action {
        ProcessAction::SpawnSubagent { task, label } => {
            // Enforce max concurrent subagents limit
            let max_concurrent = {
                let cfg = config.read().unwrap();
                cfg.sandbox.max_concurrent_subagents
            };
            match db.count_running_subagents() {
                Ok(running) if running >= max_concurrent as i64 => {
                    return ProcessResult::Error {
                        message: format!(
                            "Maximum concurrent subagents ({max_concurrent}) reached. Wait for a running task to complete."
                        ),
                    };
                }
                Err(e) => {
                    return ProcessResult::Error {
                        message: format!("Failed to check subagent count: {e}"),
                    };
                }
                _ => {}
            }

            let label = label.unwrap_or_else(|| task.chars().take(40).collect::<String>());
            let (model, sub_agent_env, disabled_tools_base) = {
                let cfg = config.read().unwrap();
                // Use task-specific model routing for subagents
                let task_model = cfg.agent.model_routing.model_for_task(
                    bat_types::config::TaskType::Subagent,
                    &cfg.agent.model,
                );
                (
                    task_model,
                    build_agent_env(&cfg),
                    cfg.agent.disabled_tools.clone(),
                )
            };
            match db.create_subagent_session(session_id, &model, &label, &task) {
                Ok(sub_session) => {
                    let sub_key = sub_session.key.clone();
                    let sub_key2 = sub_key.clone();
                    let sub_id = sub_session.id;
                    let path_policies = db.get_path_policies().unwrap_or_default();
                    let sub_prompt = {
                        let cfg = config.read().unwrap();
                        // TODO: Pass skills section from skill manager for subagents
                        crate::system_prompt::build_worker_prompt(&cfg, &path_policies, &task, None)
                            .unwrap_or_else(|e| {
                                tracing::warn!("Failed to build worker prompt: {e}");
                                format!("You are a subagent. Complete this task: {task}")
                            })
                    };
                    let mut disabled_tools = disabled_tools_base;
                    disabled_tools.push("session_spawn".to_string());

                    let eb = event_bus.clone();
                    let db2 = db.clone();
                    let sm = Arc::new(session::SessionManager::new(db.clone(), model.clone()));
                    let pm = proc_mgr.clone();
                    let cfg2 = config.clone();
                    let tg_state = telegram_state.clone();

                    tokio::spawn(async move {
                        info!("Subagent starting: key={sub_key}, task={}", &task[..task.len().min(60)]);
                        // Clone values needed after run_agent_turn (which moves them)
                        let sm_post = sm.clone();
                        let pm_post = pm.clone();
                        let cfg2_post = cfg2.clone();
                        let tg_state_post = tg_state.clone();
                        let result = run_agent_turn(
                            sub_id, model, sub_prompt, vec![], task.clone(),
                            vec![],  // Subagents don't receive images
                            path_policies, disabled_tools, sub_agent_env,
                            eb.clone(), sm, db2.clone(), pm, cfg2,
                            "subagent".to_string(),  // This is a subagent/worker session
                            tg_state,
                            None, // Subagents don't have dedicated Telegram reply channels
                        ).await;
                        // Rebind for use in match arms
                        let sm = sm_post;
                        let pm = pm_post;
                        let cfg2 = cfg2_post;
                        let tg_state = tg_state_post;
                        match result {
                            Ok(()) => {
                                // IMPL-01: Read subagent's actual last assistant message as summary
                                let summary = sm.get_history(sub_id)
                                    .ok()
                                    .and_then(|h| h.iter().rev()
                                        .find(|m| m.role == bat_types::message::Role::Assistant)
                                        .map(|m| {
                                            let content = &m.content;
                                            if content.len() > 2000 {
                                                format!("{}...", &content[..2000])
                                            } else {
                                                content.clone()
                                            }
                                        }))
                                    .unwrap_or_else(|| "Task completed (no output captured)".to_string());
                                let _ = db2.update_subagent_status(sub_id, SubagentStatus::Completed, Some(&summary));
                                eb.send(AgentToGateway::AuditLog {
                                    level: "info".into(), category: "agent".into(),
                                    event: "subagent_complete".into(),
                                    summary: format!("[Subagent: {label} — completed]"),
                                    detail_json: None,
                                });
                                info!("Subagent completed: key={sub_key}");

                                // === IMPL-02: Announce results back to parent orchestrator ===
                                let announce_summary = sm.get_history(sub_id)
                                    .ok()
                                    .and_then(|h| h.iter().rev()
                                        .find(|m| m.role == bat_types::message::Role::Assistant)
                                        .map(|m| {
                                            if m.content.len() > 3000 {
                                                format!("{}...(truncated)", &m.content[..3000])
                                            } else {
                                                m.content.clone()
                                            }
                                        }))
                                    .unwrap_or_else(|| "(no output)".to_string());

                                let announce_msg = format!(
                                    "[Subagent completed: \"{label}\"]\n\nResults:\n{announce_summary}\n\nSummarize this for the user naturally and briefly."
                                );

                                let parent_session = db2.get_session(session_id);
                                if let Ok(Some(_parent)) = parent_session {
                                    let history = sm.get_history(session_id).unwrap_or_default();
                                    let user_msg = bat_types::message::Message::user(session_id, &announce_msg);
                                    let _ = sm.append_message(&user_msg);

                                    let (cfg_model, disabled_tools, agent_env) = {
                                        let cfg = cfg2.read().unwrap();
                                        (cfg.agent.model.clone(), cfg.agent.disabled_tools.clone(), build_agent_env(&cfg))
                                    };
                                    let system_prompt = {
                                        let cfg = cfg2.read().unwrap();
                                        let policies = db2.get_path_policies().unwrap_or_default();
                                        crate::system_prompt::build_orchestrator_prompt(&cfg, &policies, None).unwrap_or_default()
                                    };
                                    let path_policies = db2.get_path_policies().unwrap_or_default();

                                    let eb2 = eb.clone();
                                    let sm2 = Arc::new(session::SessionManager::new(db2.clone(), cfg_model.clone()));
                                    let db3 = db2.clone();
                                    let pm2 = pm.clone();
                                    let cfg3 = cfg2.clone();
                                    let tg2 = tg_state.clone();

                                    tokio::spawn(async move {
                                        if let Err(e) = run_agent_turn(
                                            session_id, cfg_model, system_prompt, history, announce_msg,
                                            vec![], path_policies, disabled_tools, agent_env,
                                            eb2, sm2, db3, pm2, cfg3,
                                            "main".to_string(),
                                            tg2,
                                            None,
                                        ).await {
                                            error!("Failed to announce subagent results: {e}");
                                        }
                                    });
                                }
                            }
                            Err(e) => {
                                let _ = db2.update_subagent_status(sub_id, SubagentStatus::Failed, Some(&format!("Error: {e}")));
                                eb.send(AgentToGateway::AuditLog {
                                    level: "error".into(), category: "agent".into(),
                                    event: "subagent_failed".into(),
                                    summary: format!("[Subagent: {label} — failed] {e}"),
                                    detail_json: None,
                                });
                                error!("Subagent failed: key={sub_key}, err={e}");
                            }
                        }
                    });
                    ProcessResult::SubagentSpawned { session_key: sub_key2, session_id: sub_id.to_string() }
                }
                Err(e) => ProcessResult::Error { message: e.to_string() },
            }
        }
        ProcessAction::ListSubagents => {
            match db.get_subagents(session_id) {
                Ok(subagents) => ProcessResult::SubagentList { subagents },
                Err(e) => ProcessResult::Error { message: e.to_string() },
            }
        }
        ProcessAction::CancelSubagent { .. } => ProcessResult::SubagentCancelled,
        ProcessAction::AskOrchestrator { question, context, blocking } => {
            if let Some(ref ts) = telegram_state {
                let chat_id = *ts.active_chat_id.lock().unwrap();
                if chat_id != 0 {
                    // Send the question to the active Telegram chat
                    let text = format!(
                        "❓ *Agent Question*\n\n{question}\n\n_Context: {context}_\n\nPlease reply with your answer.",
                    );
                    let _ = ts.outbound.send(channels::telegram::OutboundMessage {
                        chat_id,
                        text,
                        reply_to: None,
                        voice_data: None,
                    });

                    if blocking {
                        // Register as pending and block until Telegram replies (10-min timeout).
                        // Uses block_in_place since handle_subagent_action is sync.
                        let (answer_tx, answer_rx) = tokio::sync::oneshot::channel::<String>();
                        {
                            let mut pending = ts.pending_question.lock().unwrap();
                            if pending.is_some() {
                                warn!("AskOrchestrator: overwriting an existing pending question");
                            }
                            *pending = Some(answer_tx);
                        }
                        return tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(600),
                                    answer_rx,
                                )
                                .await
                                {
                                    Ok(Ok(answer)) => {
                                        info!("AskOrchestrator: received Telegram answer");
                                        ProcessResult::OrchestratorAnswer { answer }
                                    }
                                    _ => {
                                        warn!("AskOrchestrator: timeout waiting for Telegram reply");
                                        ProcessResult::OrchestratorAnswer {
                                            answer: "No answer received within the timeout. Please proceed with your best judgment or ask again.".to_string(),
                                        }
                                    }
                                }
                            })
                        });
                    } else {
                        // Non-blocking: question sent, return immediately
                        return ProcessResult::OrchestratorAnswer {
                            answer: "Question sent to user via Telegram. Continuing without waiting for a reply.".to_string(),
                        };
                    }
                }
            }
            // Fallback: no Telegram active — inform the subagent
            ProcessResult::OrchestratorAnswer {
                answer: "No human channel is currently available. Please proceed with your best judgment, or surface this question in your final response so the user can address it.".to_string(),
            }
        }
        ProcessAction::PauseSubagent { session_key } => {
            // TODO: Implement actual pause logic when mid-turn message injection is ready
            tracing::info!("Pausing subagent: {}", session_key);
            ProcessResult::SubagentPaused
        }
        ProcessAction::ResumeSubagent { session_key, instructions } => {
            // TODO: Implement actual resume logic when mid-turn message injection is ready
            tracing::info!("Resuming subagent: {} with instructions: {:?}", session_key, instructions);
            ProcessResult::SubagentResumed
        }
        ProcessAction::InstructSubagent { session_key, instruction } => {
            // TODO: Implement actual instruction sending when mid-turn message injection is ready
            tracing::info!("Instructing subagent: {} with: {}", session_key, instruction);
            ProcessResult::SubagentInstructed
        }
        ProcessAction::AnswerSubagent { session_key, question_id, answer } => {
            // TODO: Implement actual answer routing when mid-turn message injection is ready
            tracing::info!("Answering subagent: {} question_id: {} with: {}", session_key, question_id, answer);
            ProcessResult::SubagentAnswered
        }
        _ => ProcessResult::Error { message: "Not a subagent action".into() },
    }
}

/// Handle a process management request from the agent.
async fn handle_process_request(
    proc_mgr: process_manager::ProcessManager,
    action: bat_types::ipc::ProcessAction,
) -> bat_types::ipc::ProcessResult {
    use bat_types::ipc::{ProcessAction, ProcessResult};

    match action {
        ProcessAction::Start { command, workdir, background } => {
            if background {
                match proc_mgr.spawn(&command, workdir.as_deref()).await {
                    Ok(session_id) => ProcessResult::Started { session_id },
                    Err(e) => ProcessResult::Error { message: e.to_string() },
                }
            } else {
                match proc_mgr.run_foreground(&command, workdir.as_deref()).await {
                    Ok((stdout, stderr, exit_code)) => ProcessResult::Output {
                        session_id: String::new(),
                        stdout,
                        stderr,
                        is_running: false,
                        exit_code,
                    },
                    Err(e) => ProcessResult::Error { message: e.to_string() },
                }
            }
        }
        ProcessAction::GetOutput { session_id } => {
            match proc_mgr.get_output(&session_id).await {
                Ok((stdout, stderr, is_running, exit_code)) => ProcessResult::Output {
                    session_id,
                    stdout,
                    stderr,
                    is_running,
                    exit_code,
                },
                Err(e) => ProcessResult::Error { message: e.to_string() },
            }
        }
        ProcessAction::WriteStdin { session_id, data } => {
            match proc_mgr.write_stdin(&session_id, &data).await {
                Ok(()) => ProcessResult::Written,
                Err(e) => ProcessResult::Error { message: e.to_string() },
            }
        }
        ProcessAction::Kill { session_id } => {
            match proc_mgr.kill(&session_id).await {
                Ok(()) => ProcessResult::Killed,
                Err(e) => ProcessResult::Error { message: e.to_string() },
            }
        }
        ProcessAction::List => {
            ProcessResult::ProcessList {
                processes: proc_mgr.list().await,
            }
        }
        // Subagent actions are handled in the IPC loop directly (not here)
        // because they need access to gateway state that would make this future !Send.
        ProcessAction::SpawnSubagent { .. } | ProcessAction::ListSubagents | ProcessAction::CancelSubagent { .. } | ProcessAction::AskOrchestrator { .. } | ProcessAction::PauseSubagent { .. } | ProcessAction::ResumeSubagent { .. } | ProcessAction::InstructSubagent { .. } | ProcessAction::AnswerSubagent { .. } => {
            ProcessResult::Error { message: "Subagent actions must be handled by the gateway directly".to_string() }
        }
    }
}

/// Handle events from a single agent turn and send replies to Telegram.
/// Uses a dedicated mpsc channel instead of the broadcast EventBus to avoid lag-related drops.
async fn handle_telegram_turn_events(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<AgentToGateway>,
    outbound: tokio::sync::mpsc::UnboundedSender<channels::telegram::OutboundMessage>,
    chat_id: i64,
    config: Arc<RwLock<BatConfig>>,
) {
    if chat_id == 0 {
        return;
    }
    let mut pending_text = String::new();
    debug!("Telegram turn handler: listening (chat_id={chat_id})");
    while let Some(event) = rx.recv().await {
        match event {
            AgentToGateway::TextDelta { content } => {
                pending_text.push_str(&content);
            }
            AgentToGateway::TurnComplete { ref message } => {
                info!("Telegram turn handler: TurnComplete, content_len={}", message.content.len());
                let text = if !message.content.is_empty() {
                    message.content.clone()
                } else if !pending_text.is_empty() {
                    pending_text.clone()
                } else {
                    // Fix 4: fallback message when agent completes with empty content
                    "I processed your request but had nothing to say.".to_string()
                };

                // Try TTS if enabled and API key available
                let (tts_available, voice_cfg, tts_api_key) = {
                    let cfg = config.read().unwrap();
                    let available = cfg.voice.tts_available(&cfg.api_keys);
                    let vc = cfg.voice.clone();
                    let key = match cfg.voice.tts_provider.as_str() {
                        "elevenlabs" => cfg.api_keys.elevenlabs_key().unwrap_or_default(),
                        _ => cfg.api_keys.openai_key().unwrap_or_default(),
                    };
                    (available, vc, key)
                };
                let voice_data = if tts_available {
                    match tts::synthesize(&text, &voice_cfg, &tts_api_key).await {
                        Ok(audio) => {
                            info!("Telegram TTS: {} bytes", audio.data.len());
                            Some(audio.data)
                        }
                        Err(e) => {
                            warn!("Telegram TTS failed: {e}");
                            None
                        }
                    }
                } else {
                    None
                };
                let _ = outbound.send(channels::telegram::OutboundMessage {
                    chat_id,
                    text,
                    reply_to: None,
                    voice_data,
                });
                break;
            }
            AgentToGateway::Error { message } => {
                let _ = outbound.send(channels::telegram::OutboundMessage {
                    chat_id,
                    text: format!("⚠️ {message}"),
                    reply_to: None,
                    voice_data: None,
                });
                break;
            }
            _ => {}
        }
    }
}

async fn run_agent_turn(
    session_id: Uuid,
    model: String,
    system_prompt: String,
    history: Vec<Message>,
    user_content: String,
    user_images: Vec<bat_types::message::ImageAttachment>,
    path_policies: Vec<PathPolicy>,
    disabled_tools: Vec<String>,
    agent_env: ipc::AgentEnv,
    event_bus: EventBus,
    session_manager: Arc<SessionManager>,
    db: Arc<Database>,
    proc_mgr: process_manager::ProcessManager,
    gw_config: Arc<RwLock<BatConfig>>,
    session_kind: String,  // "main" or "subagent"
    telegram_state: Option<Arc<TelegramState>>,
    telegram_reply_tx: Option<tokio::sync::mpsc::UnboundedSender<AgentToGateway>>,
) -> Result<()> {
    let sid = session_id.to_string();

    // Detect user corrections/preferences in the message
    let detections = correction::detect(&user_content);
    for det in &detections {
        let _ = db.record_observation(det.kind, &det.key, Some(&det.value), Some(&sid));
        info!("Detected {:?}: {} = {}", det.kind, det.key, det.value);
    }

    // 1. Create named pipe server
    let (server, pipe_name) = ipc::create_pipe_server(session_id)
        .context("Failed to create agent pipe")?;

    info!("Created pipe: {}", pipe_name);

    // 2. Spawn the agent child process
    let mut child = ipc::spawn_agent(&pipe_name, &agent_env)
        .context("Failed to spawn bat-agent")?;

    let pid = child.id().unwrap_or(0);
    info!("Spawned bat-agent (pid: {})", pid);
    audit(&db, &event_bus, AuditLevel::Info, AuditCategory::Agent, "agent_spawn",
        &format!("Agent spawned (pid: {pid}, model: {model})"), Some(&sid), None);

    // Apply OS-native sandbox
    let sandbox_cfg = {
        let cfg = gw_config.read().unwrap();
        sandbox::SandboxConfig {
            memory_limit_mb: cfg.sandbox.memory_limit_mb as u64,
            ..Default::default()
        }
    };
    let _sandbox_handle = match sandbox::apply_sandbox(pid, &sandbox_cfg) {
        Ok(handle) => {
            audit(&db, &event_bus, AuditLevel::Info, AuditCategory::Agent, "sandbox_applied",
                &format!("Sandbox applied (pid: {pid}, mem: {}MB)", sandbox_cfg.memory_limit_mb), Some(&sid), None);
            Some(handle)
        }
        Err(e) => {
            warn!("Failed to apply sandbox: {e}");
            audit(&db, &event_bus, AuditLevel::Warn, AuditCategory::Agent, "sandbox_failed",
                &format!("Sandbox failed: {e}"), Some(&sid), None);
            None
        }
    };

    // 3. Wait for agent to connect
    let mut pipe = ipc::wait_for_agent(server)
        .await
        .context("Failed while waiting for agent connection")?;

    info!("Agent connected");
    audit(&db, &event_bus, AuditLevel::Debug, AuditCategory::Ipc, "pipe_connected",
        "Agent connected to IPC pipe", Some(&sid), None);

    // 4. Send Init
    info!("Sending Init to agent (session_kind: {})", session_kind);
    pipe.send(&GatewayToAgent::Init {
        session_id: session_id.to_string(),
        model: model.clone(),
        system_prompt,
        history,
        path_policies,
        disabled_tools,
        session_kind,
    })
    .await
    .context("Failed to send Init to agent")?;
    info!("Init sent successfully");

    // 5. Send UserMessage
    info!("Sending UserMessage to agent: {}", &user_content[..user_content.len().min(80)]);
    pipe.send(&GatewayToAgent::UserMessage {
        content: user_content,
        images: user_images,
    })
    .await
    .context("Failed to send UserMessage to agent")?;
    info!("UserMessage sent successfully");

    // 6. Read events until TurnComplete or Error
    loop {
        match pipe.recv().await? {
            Some(event) => {
                let is_terminal = matches!(
                    event,
                    AgentToGateway::TurnComplete { .. } | AgentToGateway::Error { .. }
                );

                // Audit tool call events + record observations
                match &event {
                    AgentToGateway::ToolCallStart { tool_call } => {
                        audit(&db, &event_bus, AuditLevel::Info, AuditCategory::Tool, "tool_call_start",
                            &format!("Tool call: {}", tool_call.name), Some(&sid),
                            Some(&serde_json::to_string(&tool_call.input).unwrap_or_default()));

                        // Record tool use observation
                        let _ = db.record_observation(
                            ObservationKind::ToolUse, &tool_call.name, None, Some(&sid),
                        );

                        // Record path access for fs tools
                        if let Some(path) = tool_call.input.get("path").and_then(|v| v.as_str()) {
                            let _ = db.record_observation(
                                ObservationKind::PathAccess, path, Some(&tool_call.name), Some(&sid),
                            );
                        }
                    }
                    AgentToGateway::ToolCallResult { result } => {
                        let status = if result.is_error { "error" } else { "success" };
                        let summary = format!("Tool result ({}): {} chars", status, result.content.len());
                        audit(&db, &event_bus, AuditLevel::Info, AuditCategory::Tool, "tool_call_result",
                            &summary, Some(&sid), None);
                    }
                    AgentToGateway::TurnComplete { ref message } => {
                        let tokens = format!("in: {}, out: {}",
                            message.token_input.unwrap_or(0),
                            message.token_output.unwrap_or(0));
                        audit(&db, &event_bus, AuditLevel::Info, AuditCategory::Agent, "turn_complete",
                            &format!("Turn complete ({tokens})"), Some(&sid), None);
                    }
                    AgentToGateway::Error { message } => {
                        audit(&db, &event_bus, AuditLevel::Error, AuditCategory::Agent, "agent_error",
                            message, Some(&sid), None);
                    }
                    AgentToGateway::ProcessRequest { ref request_id, ref action } => {
                        use bat_types::ipc::ProcessAction;

                        // Handle subagent actions synchronously, process actions async
                        let result = if matches!(action, ProcessAction::SpawnSubagent { .. } | ProcessAction::ListSubagents | ProcessAction::CancelSubagent { .. } | ProcessAction::AskOrchestrator { .. } | ProcessAction::PauseSubagent { .. } | ProcessAction::ResumeSubagent { .. } | ProcessAction::InstructSubagent { .. }) {
                            handle_subagent_action(action.clone(), session_id, db.clone(), event_bus.clone(), proc_mgr.clone(), gw_config.clone(), telegram_state.clone())
                        } else {
                            handle_process_request(proc_mgr.clone(), action.clone()).await
                        };
                        let _ = pipe.send(&GatewayToAgent::ProcessResponse {
                            request_id: request_id.clone(),
                            result,
                        }).await;
                        continue;
                    }
                    _ => {}
                }

                // Persist TurnComplete message to database
                if let AgentToGateway::TurnComplete { ref message } = event {
                    session_manager
                        .append_message(message)
                        .context("Failed to persist assistant message")?;

                    if let (Some(inp), Some(out)) = (message.token_input, message.token_output) {
                        session_manager
                            .update_token_usage(session_id, inp, out)
                            .context("Failed to update token usage")?;
                        
                        // Record detailed usage with cost for cost governance
                        let cost_governor = cost_governor::CostGovernor::new(db.clone());
                        if let Err(e) = cost_governor.record_usage(
                            session_id,
                            &model,
                            inp as u32,
                            out as u32,
                        ).await {
                            warn!("Failed to record cost usage: {}", e);
                        }
                    }
                }

                // Forward to the dedicated Telegram reply channel if present
                if let Some(ref tx) = telegram_reply_tx {
                    if let Err(e) = tx.send(event.clone()) {
                        error!("Failed to forward event to Telegram reply channel: {e}");
                    }
                }

                event_bus.send(event);

                if is_terminal {
                    break;
                }
            }
            None => {
                // Pipe closed without TurnComplete
                audit(&db, &event_bus, AuditLevel::Error, AuditCategory::Ipc, "pipe_disconnected",
                    "Agent disconnected unexpectedly", Some(&sid), None);
                event_bus.send(AgentToGateway::Error {
                    message: "Agent disconnected unexpectedly".to_string(),
                });
                break;
            }
        }
    }

    // 7. Wait for the child process to exit and capture stderr
    let exit_status = child.wait().await;
    match exit_status {
        Ok(status) => {
            let code = status.code().unwrap_or(-1);
            if code != 0 {
                error!("Agent process exited with code {}", code);
                audit(&db, &event_bus, AuditLevel::Error, AuditCategory::Agent, "agent_exit",
                    &format!("Agent exited with code {code}"), Some(&sid), None);
            } else {
                info!("Agent process exited cleanly");
                audit(&db, &event_bus, AuditLevel::Debug, AuditCategory::Agent, "agent_exit",
                    "Agent exited cleanly", Some(&sid), None);
            }
        }
        Err(e) => {
            error!("Failed to wait for agent process: {}", e);
            audit(&db, &event_bus, AuditLevel::Error, AuditCategory::Agent, "agent_exit",
                &format!("Failed to wait for agent: {e}"), Some(&sid), None);
        }
    }

    Ok(())
}

/// Build the environment variables for the agent process from config.
fn build_agent_env(cfg: &BatConfig) -> ipc::AgentEnv {
    ipc::AgentEnv {
        anthropic_key: cfg.api_keys.anthropic_key(),
        openai_key: cfg.api_keys.openai_key(),
        ollama_endpoint: Some(cfg.api_keys.ollama_endpoint()),
    }
}

/// Validate that the required API key is available for the model's provider.
fn validate_provider_key(model: &str, env: &ipc::AgentEnv) -> Result<()> {
    use bat_types::config::{ApiKeys, LlmProvider};
    match ApiKeys::provider_for_model(model) {
        LlmProvider::Anthropic => {
            if env.anthropic_key.as_ref().map_or(true, |k| k.is_empty()) {
                anyhow::bail!(
                    "No Anthropic API key found. Add it in Settings → API Keys or set ANTHROPIC_API_KEY env var."
                );
            }
        }
        LlmProvider::OpenAI => {
            if env.openai_key.as_ref().map_or(true, |k| k.is_empty()) {
                anyhow::bail!(
                    "No OpenAI API key found. Add it in Settings → API Keys or set OPENAI_API_KEY env var."
                );
            }
        }
        LlmProvider::Ollama => {
            // Ollama doesn't need an API key — just an endpoint
        }
    }
    Ok(())
}
