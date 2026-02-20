pub mod config;
pub mod db;
pub mod events;
pub mod ipc;
pub mod session;
pub mod system_prompt;

pub use events::EventBus;

use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

use bat_types::{
    config::BatConfig,
    ipc::{AgentToGateway, GatewayToAgent},
    message::Message,
    policy::{AccessLevel, PathPolicy},
    session::SessionMeta,
};

use db::Database;
use session::SessionManager;

/// Metadata about a registered tool (for the Settings UI).
#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// The central gateway — owns the database, session state, and event bus.
/// The Tauri shell holds an `Arc<Gateway>` in `AppState`.
pub struct Gateway {
    session_manager: Arc<SessionManager>,
    db: Arc<Database>,
    config: Arc<RwLock<BatConfig>>,
    event_bus: EventBus,
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
        })
    }

    /// Subscribe to the event bus for streaming events from agents.
    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentToGateway> {
        self.event_bus.subscribe()
    }

    // ─── Commands exposed to Tauri ────────────────────────────────────────────

    /// Send a user message. Persists the message, spawns the agent in the
    /// background, and returns immediately. Events arrive via the event bus.
    pub async fn send_user_message(&self, content: &str) -> Result<()> {
        let session = self
            .session_manager
            .get_or_create_main()
            .context("Failed to get or create main session")?;

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
        let (model, disabled_tools, config_api_key) = {
            let cfg = self.config.read().unwrap();
            (
                cfg.agent.model.clone(),
                cfg.agent.disabled_tools.clone(),
                cfg.agent.api_key.clone(),
            )
        };

        let system_prompt = {
            let cfg = self.config.read().unwrap();
            system_prompt::build_system_prompt(&cfg, &path_policies)
                .context("Failed to build system prompt")?
        };

        // API key: env var takes priority, then fall back to config value
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .or(config_api_key)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No API key found. Set ANTHROPIC_API_KEY env var or add it in Settings."
                )
            })?;

        let event_bus = self.event_bus.clone();
        let session_manager = Arc::clone(&self.session_manager);
        let content_owned = content.to_string();

        // Spawn the agent turn in a background task
        tokio::spawn(async move {
            event_bus.send(AgentToGateway::TextDelta {
                content: String::new(), // Signal "thinking started"
            });

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
                session_manager,
            )
            .await
            {
                error!("Agent turn failed: {}", e);
                event_bus.send(AgentToGateway::Error {
                    message: format!("Agent error: {e}"),
                });
            }
        });

        Ok(())
    }

    /// Get the full message history for the main session.
    pub async fn get_main_history(&self) -> Result<Vec<Message>> {
        let session = self.session_manager.get_or_create_main()?;
        self.session_manager.get_history(session.id)
    }

    /// Get the main session metadata.
    pub async fn get_main_session(&self) -> Result<SessionMeta> {
        self.session_manager.get_or_create_main()
    }

    /// Get all configured path policies.
    pub async fn get_path_policies(&self) -> Result<Vec<PathPolicy>> {
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
            path: path.into(),
            access: access_level,
            recursive,
            description: None,
        };
        self.db.add_path_policy(&policy)
    }

    /// Delete a path policy by its path string.
    pub async fn delete_path_policy(&self, path: &str) -> Result<()> {
        self.db.delete_path_policy(path)
    }

    /// Get a clone of the current config.
    pub fn get_config(&self) -> BatConfig {
        self.config.read().unwrap().clone()
    }

    /// Update config in-memory and persist to disk.
    pub fn update_config(&self, new_config: BatConfig) -> Result<()> {
        config::save_config(&new_config)?;
        *self.config.write().unwrap() = new_config;
        Ok(())
    }

    /// Returns info about all known tools (name, description, enabled state).
    pub fn get_tools_info(&self) -> Vec<ToolInfo> {
        let disabled = self.config.read().unwrap().agent.disabled_tools.clone();
        vec![
            ToolInfo {
                name: "fs_read".to_string(),
                description: "Read the contents of a file on disk.".to_string(),
                enabled: !disabled.contains(&"fs_read".to_string()),
            },
            ToolInfo {
                name: "fs_write".to_string(),
                description: "Write or create files on disk.".to_string(),
                enabled: !disabled.contains(&"fs_write".to_string()),
            },
            ToolInfo {
                name: "fs_list".to_string(),
                description: "List the contents of a directory.".to_string(),
                enabled: !disabled.contains(&"fs_list".to_string()),
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
        system_prompt::build_system_prompt(&cfg, &path_policies)
    }
}

// ─── Agent turn runner ────────────────────────────────────────────────────────

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
) -> Result<()> {
    // 1. Create named pipe server
    let (server, pipe_name) = ipc::create_pipe_server(session_id)
        .context("Failed to create agent pipe")?;

    info!("Created pipe: {}", pipe_name);

    // 2. Spawn the agent child process
    let mut child = ipc::spawn_agent(&pipe_name, &api_key)
        .context("Failed to spawn bat-agent")?;

    info!("Spawned bat-agent (pid: {:?})", child.id());

    // 3. Wait for agent to connect
    let mut pipe = ipc::wait_for_agent(server)
        .await
        .context("Failed while waiting for agent connection")?;

    info!("Agent connected");

    // 4. Send Init
    pipe.send(&GatewayToAgent::Init {
        session_id: session_id.to_string(),
        model,
        system_prompt,
        history,
        path_policies,
        disabled_tools,
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
            } else {
                info!("Agent process exited cleanly");
            }
        }
        Err(e) => error!("Failed to wait for agent process: {}", e),
    }

    Ok(())
}
