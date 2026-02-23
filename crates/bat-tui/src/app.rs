use std::sync::Arc;

use bat_gateway::Gateway;
use bat_types::ipc::AgentToGateway;
use bat_types::memory::{MemoryFileInfo, ObservationSummary};
use bat_types::message::Message;
use bat_types::policy::PathPolicy;

/// Which top-level screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Onboarding,
    Chat,
    Settings,
    Logs,
    Memory,
    Activity,
}

/// Settings sub-pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    AgentConfig,
    PathPolicies,
    Tools,
    About,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 4] = [
        SettingsTab::AgentConfig,
        SettingsTab::PathPolicies,
        SettingsTab::Tools,
        SettingsTab::About,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::AgentConfig => "Agent Config",
            Self::PathPolicies => "Path Policies",
            Self::Tools => "Tools",
            Self::About => "About",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::AgentConfig => Self::PathPolicies,
            Self::PathPolicies => Self::Tools,
            Self::Tools => Self::About,
            Self::About => Self::AgentConfig,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::AgentConfig => Self::About,
            Self::PathPolicies => Self::AgentConfig,
            Self::Tools => Self::PathPolicies,
            Self::About => Self::Tools,
        }
    }
}

/// Input mode for text fields in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

/// Main application state.
pub struct App {
    pub gateway: Arc<Gateway>,
    pub should_quit: bool,

    // Screen state
    pub screen: Screen,
    pub settings_tab: SettingsTab,
    pub show_help: bool,

    // Chat state
    pub messages: Vec<Message>,
    pub input: String,
    pub scroll_offset: usize,
    pub streaming_text: String,
    pub is_streaming: bool,
    pub tool_calls_expanded: Vec<bool>,

    // Settings state
    pub settings_cursor: usize,
    pub input_mode: InputMode,
    pub show_api_key: bool,
    pub path_policies: Vec<PathPolicy>,
    pub path_cursor: usize,
    pub tools_cursor: usize,

    // Editing buffers
    pub edit_buffer: String,

    // Audit log
    pub audit_entries: Vec<String>,
    pub logs_scroll: usize,

    // Memory
    pub memory_files: Vec<MemoryFileInfo>,
    pub memory_cursor: usize,
    pub memory_content: String,
    pub memory_edit_content: String,
    pub memory_editing: bool,
    pub memory_summary: Option<ObservationSummary>,
    pub memory_consolidating: bool,
    pub memory_consolidation_result: String,

    // Activity (subagents)
    pub subagents: Vec<bat_types::session::SubagentInfo>,
    pub activity_cursor: usize,
    pub activity_expanded: bool,

    // Onboarding
    pub onboarding_step: u8,          // 0=welcome, 1=apikey, 2=name, 3=access, 4=ready
    pub onboarding_api_key: String,
    pub onboarding_name: String,
    pub onboarding_folders: Vec<(String, String, bool)>, // (path, access, recursive)
    pub onboarding_error: String,
    pub onboarding_validated: bool,
    pub onboarding_editing: bool,     // true when typing in a field
}

impl App {
    pub fn new(gateway: Arc<Gateway>, history: Vec<Message>) -> Self {
        let path_policies = gateway.get_path_policies_sync()
            .unwrap_or_default();

        let needs_onboarding = !gateway.is_onboarding_complete();

        Self {
            gateway,
            should_quit: false,

            screen: if needs_onboarding { Screen::Onboarding } else { Screen::Chat },
            settings_tab: SettingsTab::AgentConfig,
            show_help: false,

            messages: history,
            input: String::new(),
            scroll_offset: 0,
            streaming_text: String::new(),
            is_streaming: false,
            tool_calls_expanded: Vec::new(),

            settings_cursor: 0,
            input_mode: InputMode::Normal,
            show_api_key: false,
            path_policies,
            path_cursor: 0,
            tools_cursor: 0,

            edit_buffer: String::new(),

            audit_entries: Vec::new(),
            logs_scroll: 0,

            memory_files: Vec::new(),
            memory_cursor: 0,
            memory_content: String::new(),
            memory_edit_content: String::new(),
            memory_editing: false,
            memory_summary: None,
            memory_consolidating: false,
            memory_consolidation_result: String::new(),

            subagents: Vec::new(),
            activity_cursor: 0,
            activity_expanded: false,

            onboarding_step: 0,
            onboarding_api_key: String::new(),
            onboarding_name: String::new(),
            onboarding_folders: Vec::new(),
            onboarding_error: String::new(),
            onboarding_validated: false,
            onboarding_editing: false,
        }
    }

    /// Handle an event from the gateway event bus.
    pub fn handle_gateway_event(&mut self, event: AgentToGateway) {
        match event {
            AgentToGateway::TextDelta { content } => {
                if !self.is_streaming {
                    self.is_streaming = true;
                    self.streaming_text.clear();
                }
                self.streaming_text.push_str(&content);
            }
            AgentToGateway::ToolCallStart { tool_call } => {
                // Append a visual marker into streaming text
                self.streaming_text.push_str(&format!(
                    "\n🔧 {} ({})\n",
                    tool_call.name, tool_call.id
                ));
                self.tool_calls_expanded.push(false);
            }
            AgentToGateway::ToolCallResult { result } => {
                let status = if result.is_error { "❌" } else { "✅" };
                self.streaming_text.push_str(&format!(
                    "  {} {}\n",
                    status,
                    truncate_str(&result.content, 120),
                ));
            }
            AgentToGateway::TurnComplete { message } => {
                self.is_streaming = false;
                self.streaming_text.clear();
                self.tool_calls_expanded.clear();
                self.messages.push(message);
                // Auto-scroll to bottom
                self.scroll_offset = 0;
            }
            AgentToGateway::Error { message } => {
                self.is_streaming = false;
                self.streaming_text.clear();
                // Show error as a system message
                let err_msg = Message::system(
                    self.messages
                        .first()
                        .map(|m| m.session_id)
                        .unwrap_or_default(),
                    format!("⚠️ {message}"),
                );
                self.messages.push(err_msg);
            }
            AgentToGateway::AuditLog { level, category, summary, .. } => {
                // Store audit events for the logs screen
                self.audit_entries.push(format!(
                    "[{}] [{}] {}",
                    level.to_uppercase(),
                    category.to_uppercase(),
                    summary,
                ));
            }
            AgentToGateway::ProcessRequest { .. } => {
                // Process requests are handled by the gateway, not the TUI
            }
        }
    }

    /// Send the current input as a user message.
    pub async fn send_message(&mut self) -> anyhow::Result<()> {
        let content = self.input.trim().to_string();
        if content.is_empty() {
            return Ok(());
        }
        self.input.clear();

        // Add user message to display immediately
        let session = self.gateway.get_main_session().await?;
        let user_msg = Message::user(session.id, &content);
        self.messages.push(user_msg);
        self.scroll_offset = 0;

        // Send to gateway (spawns agent in background)
        self.gateway.send_user_message(&content).await?;
        Ok(())
    }

    /// Refresh memory files list and summary from the gateway.
    pub async fn refresh_memory(&mut self) {
        if let Ok(files) = self.gateway.list_memory_files() {
            self.memory_files = files;
        }
        if let Ok(summary) = self.gateway.get_observation_summary() {
            self.memory_summary = Some(summary);
        }
    }

    /// Load the content of the currently selected memory file.
    pub async fn load_selected_memory_file(&mut self) {
        if let Some(file) = self.memory_files.get(self.memory_cursor) {
            match self.gateway.read_memory_file(&file.name) {
                Ok(content) => {
                    self.memory_content = content.clone();
                    self.memory_edit_content = content;
                }
                Err(e) => {
                    self.memory_content = format!("Error: {e}");
                    self.memory_edit_content.clear();
                }
            }
        }
    }

    /// Refresh subagent list from the gateway.
    pub async fn refresh_subagents(&mut self) {
        if let Ok(agents) = self.gateway.get_subagents().await {
            self.subagents = agents;
            if self.activity_cursor >= self.subagents.len() && !self.subagents.is_empty() {
                self.activity_cursor = self.subagents.len() - 1;
            }
        }
    }

    /// Refresh path policies from the database.
    pub async fn refresh_path_policies(&mut self) {
        if let Ok(policies) = self.gateway.get_path_policies().await {
            self.path_policies = policies;
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
