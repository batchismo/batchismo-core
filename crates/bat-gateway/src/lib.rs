pub mod channels;
pub mod config;
pub mod consolidation;
pub mod db;
pub mod events;
pub mod ipc;
pub mod memory;
pub mod process_manager;
pub mod sandbox;
pub mod session;
pub mod stt;
pub mod reflection;
pub mod system_prompt;
pub mod tts;

pub use events::EventBus;

use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
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
}

impl Gateway {
    /// Create a new gateway. This does NOT start the agent process.
    pub fn new(config: BatConfig, db: Arc<Database>) -> Result<Self> {
        let default_model = config.agent.model.clone();
        let session_manager =
            Arc::new(SessionManager::new(Arc::clone(&db), default_model));
        let event_bus = EventBus::new();

        Ok(Self {
            session_manager,
            db,
            config: Arc::new(RwLock::new(config)),
            event_bus,
            process_manager: process_manager::ProcessManager::new(),
            active_session_key: Arc::new(RwLock::new("main".to_string())),
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
                let (mut inbound_rx, outbound_tx) = channels::telegram::TelegramAdapter::start(tg_config);

                // Route inbound Telegram messages to the gateway
                let event_bus = self.event_bus.clone();
                let db = Arc::clone(&self.db);
                let session_manager = Arc::clone(&self.session_manager);
                let config = Arc::clone(&self.config);
                let proc_mgr = self.process_manager.clone();
                let outbound = outbound_tx.clone();

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
                    // Subscribe to events for outbound responses
                    let mut event_rx = event_bus.subscribe();

                    // Track which chat_id to respond to (shared with TelegramState)
                    let active_chat_id = active_chat_id_shared;
                    let pending_question = pending_question_shared;

                    let chat_id_for_events = active_chat_id.clone();
                    let outbound_for_events = outbound.clone();

                    // Event forwarding task — send agent responses to Telegram
                    let config_for_events = Arc::clone(&config);
                    tokio::spawn(async move {
                        let mut pending_text = String::new();
                        loop {
                            match event_rx.recv().await {
                                Ok(AgentToGateway::TextDelta { content }) => {
                                    pending_text.push_str(&content);
                                }
                                Ok(AgentToGateway::TurnComplete { ref message }) => {
                                    let chat_id = *chat_id_for_events.lock().unwrap();
                                    if chat_id != 0 {
                                        let text = if !message.content.is_empty() {
                                            message.content.clone()
                                        } else {
                                            pending_text.clone()
                                        };
                                        if !text.is_empty() {
                                            // Try TTS if enabled and API key available
                                            let (tts_available, voice_cfg, tts_api_key) = {
                                                let cfg = config_for_events.read().unwrap();
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
                                                    Ok(audio) => Some(audio.data),
                                                    Err(e) => {
                                                        warn!("TTS failed: {e}");
                                                        None
                                                    }
                                                }
                                            } else {
                                                None
                                            };
                                            let _ = outbound_for_events.send(
                                                channels::telegram::OutboundMessage {
                                                    chat_id,
                                                    text,
                                                    reply_to: None,
                                                    voice_data,
                                                },
                                            );
                                        }
                                    }
                                    pending_text.clear();
                                }
                                Ok(AgentToGateway::Error { message }) => {
                                    let chat_id = *chat_id_for_events.lock().unwrap();
                                    if chat_id != 0 {
                                        let _ = outbound_for_events.send(
                                            channels::telegram::OutboundMessage {
                                                chat_id,
                                                text: format!("⚠️ {message}"),
                                                reply_to: None,
                                                voice_data: None,
                                            },
                                        );
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });

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

                        let (cfg_model, disabled_tools, api_key) = {
                            let cfg = config.read().unwrap();
                            let key = cfg.api_keys.anthropic_key().unwrap_or_default();
                            (cfg.agent.model.clone(), cfg.agent.disabled_tools.clone(), key)
                        };
                        let system_prompt = {
                            let cfg = config.read().unwrap();
                            let policies = db.get_path_policies().unwrap_or_default();
                            // Telegram sessions are always main/orchestrator sessions
                            crate::system_prompt::build_orchestrator_prompt(&cfg, &policies).unwrap_or_default()
                        };
                        let path_policies = db.get_path_policies().unwrap_or_default();

                        let eb = event_bus.clone();
                        let sm = Arc::new(SessionManager::new(Arc::clone(&db), cfg_model.clone()));
                        let db2 = Arc::clone(&db);
                        let pm = proc_mgr.clone();
                        let cfg2 = Arc::clone(&config);
                        let tg_state = Arc::clone(&telegram_state);

                        tokio::spawn(async move {
                            if let Err(e) = run_agent_turn(
                                session.id, cfg_model, system_prompt, history, msg.text,
                                path_policies, disabled_tools, api_key,
                                eb, sm, db2, pm, cfg2,
                                "main".to_string(),  // Telegram sessions are main/orchestrator
                                Some(tg_state),
                            ).await {
                                error!("Telegram agent turn failed: {e}");
                            }
                        });
                    }
                });

                info!("Telegram channel adapter started");
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

    pub async fn send_user_message(&self, content: &str) -> Result<()> {
        let active_key = self.active_session_key();
        let session = self.get_or_create_session(&active_key)?;

        // Collect history BEFORE persisting the new user message
        // (the agent will append the user message itself, so we avoid duplicates)
        let history = self
            .session_manager
            .get_history(session.id)
            .context("Failed to get session history")?;

        // Persist the user message
        let user_msg = Message::user(session.id, content);
        self.session_manager
            .append_message(&user_msg)
            .context("Failed to persist user message")?;

        let path_policies = self
            .db
            .get_path_policies()
            .context("Failed to get path policies")?;

        // Read config once under lock
        let (model, disabled_tools, api_key) = {
            let cfg = self.config.read().unwrap();
            let key = cfg.api_keys.anthropic_key();
            (
                cfg.agent.model.clone(),
                cfg.agent.disabled_tools.clone(),
                key,
            )
        };

        let api_key = api_key.ok_or_else(|| {
            anyhow::anyhow!(
                "No Anthropic API key found. Add it in Settings → API Keys or set ANTHROPIC_API_KEY env var."
            )
        })?;

        let system_prompt = {
            let cfg = self.config.read().unwrap();
            // Use orchestrator prompt for main sessions, worker prompt for subagents
            match session.kind {
                bat_types::session::SessionKind::Main => {
                    system_prompt::build_orchestrator_prompt(&cfg, &path_policies)
                        .context("Failed to build orchestrator prompt")?
                }
                bat_types::session::SessionKind::Subagent { ref task, .. } => {
                    system_prompt::build_worker_prompt(&cfg, &path_policies, task)
                        .context("Failed to build worker prompt")?
                }
            }
        };

        let event_bus = self.event_bus.clone();
        let session_manager = Arc::clone(&self.session_manager);
        let content_owned = content.to_string();

        // Log the user message event
        self.log_event(
            AuditLevel::Info,
            AuditCategory::Gateway,
            "user_message",
            &format!("User message received ({} chars)", content.len()),
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
            let api_key_for_reflection = if is_main { Some(api_key.clone()) } else { None };
            let model_for_reflection = if is_main { Some(model.clone()) } else { None };

            if let Err(e) = run_agent_turn(
                session.id,
                model,
                system_prompt,
                history,
                content_owned,
                path_policies,
                disabled_tools,
                api_key,
                event_bus.clone(),
                session_manager.clone(),
                db,
                proc_mgr,
                gw_config,
                session_kind,
                None, // No Telegram state for UI-originated turns
            )
            .await
            {
                error!("Agent turn failed: {}", e);
                event_bus.send(AgentToGateway::Error {
                    message: format!("Agent error: {e}"),
                });
            } else if let (Some(user_msg), Some(key), Some(mdl)) = (user_msg_for_reflection, api_key_for_reflection, model_for_reflection) {
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
        // Default to orchestrator prompt for this API
        system_prompt::build_orchestrator_prompt(&cfg, &path_policies)
    }

    // ─── Onboarding ───────────────────────────────────────────────────────

    /// Check if onboarding has been completed.
    pub fn is_onboarding_complete(&self) -> bool {
        self.config.read().unwrap().agent.onboarding_complete
    }

    /// Validate an Anthropic API key by making a minimal API call.
    pub async fn validate_api_key(key: &str) -> Result<()> {
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
            cfg.agent.api_key = Some(api_key.clone()); // legacy compat
            cfg.api_keys.anthropic = Some(api_key);
            if let Some(ref oai_key) = openai_api_key {
                if !oai_key.is_empty() {
                    cfg.api_keys.openai = Some(oai_key.clone());
                    // Auto-enable voice features when OpenAI key is provided
                    cfg.voice.tts_enabled = true;
                    cfg.voice.stt_enabled = true;
                }
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
            // Use the user's configured model for consolidation
            let model = cfg.agent.model.clone();
            (key, model)
        };

        consolidation::run_consolidation(&self.db, &self.event_bus, &api_key, &model).await
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
            let label = label.unwrap_or_else(|| task.chars().take(40).collect::<String>());
            let (model, api_key_resolved, disabled_tools_base) = {
                let cfg = config.read().unwrap();
                (
                    cfg.agent.model.clone(),
                    cfg.api_keys.anthropic_key(),
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
                        crate::system_prompt::build_worker_prompt(&cfg, &path_policies, &task)
                            .unwrap_or_else(|e| {
                                tracing::warn!("Failed to build worker prompt: {e}");
                                format!("You are a subagent. Complete this task: {task}")
                            })
                    };
                    let api_key = api_key_resolved.unwrap_or_default();
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
                        let result = run_agent_turn(
                            sub_id, model, sub_prompt, vec![], task.clone(),
                            path_policies, disabled_tools, api_key,
                            eb.clone(), sm, db2.clone(), pm, cfg2,
                            "subagent".to_string(),  // This is a subagent/worker session
                            tg_state,
                        ).await;
                        match result {
                            Ok(()) => {
                                let _ = db2.update_subagent_status(sub_id, SubagentStatus::Completed, Some("Task completed successfully."));
                                eb.send(AgentToGateway::AuditLog {
                                    level: "info".into(), category: "agent".into(),
                                    event: "subagent_complete".into(),
                                    summary: format!("[Subagent: {label} — completed]"),
                                    detail_json: None,
                                });
                                info!("Subagent completed: key={sub_key}");
                            }
                            Err(e) => {
                                let _ = db2.update_subagent_status(sub_id, SubagentStatus::Failed, Some(&e.to_string()));
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
        ProcessAction::SpawnSubagent { .. } | ProcessAction::ListSubagents | ProcessAction::CancelSubagent { .. } | ProcessAction::AskOrchestrator { .. } | ProcessAction::PauseSubagent { .. } | ProcessAction::ResumeSubagent { .. } | ProcessAction::InstructSubagent { .. } => {
            ProcessResult::Error { message: "Subagent actions must be handled by the gateway directly".to_string() }
        }
    }
}

async fn run_agent_turn(
    session_id: Uuid,
    model: String,
    system_prompt: String,
    history: Vec<Message>,
    user_content: String,
    path_policies: Vec<PathPolicy>,
    disabled_tools: Vec<String>,
    api_key: String,
    event_bus: EventBus,
    session_manager: Arc<SessionManager>,
    db: Arc<Database>,
    proc_mgr: process_manager::ProcessManager,
    gw_config: Arc<RwLock<BatConfig>>,
    session_kind: String,  // "main" or "subagent"
    telegram_state: Option<Arc<TelegramState>>,
) -> Result<()> {
    let sid = session_id.to_string();

    // 1. Create named pipe server
    let (server, pipe_name) = ipc::create_pipe_server(session_id)
        .context("Failed to create agent pipe")?;

    info!("Created pipe: {}", pipe_name);

    // 2. Spawn the agent child process
    let child = ipc::spawn_agent(&pipe_name, &api_key)
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
    pipe.send(&GatewayToAgent::Init {
        session_id: session_id.to_string(),
        model,
        system_prompt,
        history,
        path_policies,
        disabled_tools,
        session_kind,
    })
    .await
    .context("Failed to send Init to agent")?;

    // 5. Send UserMessage
    pipe.send(&GatewayToAgent::UserMessage {
        content: user_content,
    })
    .await
    .context("Failed to send UserMessage to agent")?;

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
    let output = child.wait_with_output().await;
    match output {
        Ok(out) => {
            let code = out.status.code().unwrap_or(-1);
            if code != 0 {
                let stderr = String::from_utf8_lossy(&out.stderr);
                error!("Agent process exited with code {}: {}", code, stderr.trim());
                audit(&db, &event_bus, AuditLevel::Error, AuditCategory::Agent, "agent_exit",
                    &format!("Agent exited with code {code}"), Some(&sid),
                    Some(&format!("{{\"stderr\":\"{}\"}}", stderr.trim().replace('"', "\\\""))));
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
