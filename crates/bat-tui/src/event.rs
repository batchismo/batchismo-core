use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::broadcast;
use tracing::warn;

use bat_types::ipc::AgentToGateway;

use bat_gateway::Gateway;

use crate::app::{App, InputMode, Screen, SettingsTab};

pub struct EventHandler {
    rx: tokio::sync::Mutex<broadcast::Receiver<AgentToGateway>>,
}

impl EventHandler {
    pub fn new(rx: broadcast::Receiver<AgentToGateway>) -> Self {
        Self {
            rx: tokio::sync::Mutex::new(rx),
        }
    }

    /// Poll for terminal input events and gateway events.
    /// This is called once per frame from the main loop.
    pub async fn handle(&self, app: &mut App) -> Result<()> {
        // Drain all available gateway events (non-blocking)
        {
            let mut rx = self.rx.lock().await;
            loop {
                match rx.try_recv() {
                    Ok(event) => app.handle_gateway_event(event),
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!("Event bus lagged by {n} events");
                    }
                    Err(broadcast::error::TryRecvError::Closed) => break,
                }
            }
        }

        // Poll for terminal input with a short timeout so we can
        // keep draining gateway events for streaming
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events — ignore release/repeat
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key).await?;
                }
            }
        }

        Ok(())
    }
}

async fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    // Global keys
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    // Help overlay toggle
    if key.code == KeyCode::Char('?') && app.input_mode == InputMode::Normal && app.screen == Screen::Chat && app.input.is_empty() {
        app.show_help = !app.show_help;
        return Ok(());
    }

    if app.show_help {
        // Any key dismisses help
        app.show_help = false;
        return Ok(());
    }

    // Editing mode in settings
    if app.input_mode == InputMode::Editing {
        return handle_editing_key(app, key).await;
    }

    match app.screen {
        Screen::Onboarding => handle_onboarding_key(app, key).await,
        Screen::Chat => handle_chat_key(app, key).await,
        Screen::Settings => handle_settings_key(app, key).await,
        Screen::Logs => handle_logs_key(app, key).await,
        Screen::Memory => handle_memory_key(app, key).await,
        Screen::Activity => handle_activity_key(app, key).await,
        Screen::Usage => handle_usage_key(app, key).await,
    }
}

async fn handle_chat_key(app: &mut App, key: KeyEvent) -> Result<()> {
    // Session switcher overlay
    if app.show_session_switcher {
        return handle_session_switcher_key(app, key).await;
    }

    match key.code {
        KeyCode::Tab => {
            app.screen = Screen::Settings;
            app.settings_cursor = 0;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.refresh_sessions();
            app.show_session_switcher = true;
            app.session_creating = false;
            // Position cursor on active session
            let active = app.gateway.active_session_key();
            app.session_cursor = app.session_list.iter().position(|s| s.key == active).unwrap_or(0);
        }
        KeyCode::Enter => {
            if !app.input.is_empty() {
                if let Err(e) = app.send_message().await {
                    warn!("Failed to send message: {e}");
                }
            }
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Up => {
            app.scroll_offset = app.scroll_offset.saturating_add(1);
        }
        KeyCode::Down => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        KeyCode::Esc => {
            if !app.input.is_empty() {
                app.input.clear();
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_session_switcher_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.session_creating {
        match key.code {
            KeyCode::Enter => {
                let name = app.session_new_name.trim().to_string();
                if !name.is_empty() {
                    match app.gateway.create_named_session(&name) {
                        Ok(session) => {
                            let _ = app.gateway.switch_session(&session.key);
                            // Reload history for new session
                            app.messages.clear();
                            app.streaming_text.clear();
                        }
                        Err(e) => warn!("Failed to create session: {e}"),
                    }
                }
                app.session_creating = false;
                app.session_new_name.clear();
                app.refresh_sessions();
            }
            KeyCode::Esc => {
                app.session_creating = false;
                app.session_new_name.clear();
            }
            KeyCode::Char(c) => app.session_new_name.push(c),
            KeyCode::Backspace => { app.session_new_name.pop(); }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Esc => {
            app.show_session_switcher = false;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.show_session_switcher = false;
        }
        KeyCode::Up => {
            app.session_cursor = app.session_cursor.saturating_sub(1);
        }
        KeyCode::Down => {
            if !app.session_list.is_empty() {
                app.session_cursor = (app.session_cursor + 1).min(app.session_list.len() - 1);
            }
        }
        KeyCode::Enter => {
            if let Some(session) = app.session_list.get(app.session_cursor) {
                let key = session.key.clone();
                match app.gateway.switch_session(&key) {
                    Ok(_) => {
                        // Reload history
                        if let Ok(hist) = app.gateway.get_main_history().await {
                            app.messages = hist;
                        }
                        app.streaming_text.clear();
                        app.scroll_offset = 0;
                    }
                    Err(e) => warn!("Failed to switch session: {e}"),
                }
                app.show_session_switcher = false;
            }
        }
        KeyCode::Char('n') => {
            app.session_creating = true;
            app.session_new_name.clear();
        }
        KeyCode::Char('d') => {
            if let Some(session) = app.session_list.get(app.session_cursor) {
                if session.key != "main" {
                    let key = session.key.clone();
                    let was_active = app.gateway.active_session_key() == key;
                    let _ = app.gateway.delete_session(&key);
                    app.refresh_sessions();
                    // Only reload history if we deleted the active session
                    if was_active {
                        if let Ok(hist) = app.gateway.get_main_history().await {
                            app.messages = hist;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_settings_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            app.settings_tab = app.settings_tab.prev();
            app.settings_cursor = 0;
        }
        KeyCode::Tab => {
            // If on last settings tab, go to Logs screen
            if app.settings_tab == SettingsTab::About {
                app.screen = Screen::Logs;
            } else {
                app.settings_tab = app.settings_tab.next();
                app.settings_cursor = 0;
            }
        }
        KeyCode::Esc => {
            app.screen = Screen::Chat;
        }
        KeyCode::Up => match app.settings_tab {
            SettingsTab::AgentConfig => {
                app.settings_cursor = app.settings_cursor.saturating_sub(1);
            }
            SettingsTab::PathPolicies => {
                app.path_cursor = app.path_cursor.saturating_sub(1);
            }
            SettingsTab::Tools => {
                app.tools_cursor = app.tools_cursor.saturating_sub(1);
            }
            _ => {}
        },
        KeyCode::Down => match app.settings_tab {
            SettingsTab::AgentConfig => {
                app.settings_cursor = (app.settings_cursor + 1).min(3); // 4 fields: name, model, thinking, api key
            }
            SettingsTab::PathPolicies => {
                let max = app.path_policies.len().saturating_sub(1);
                app.path_cursor = (app.path_cursor + 1).min(max);
            }
            SettingsTab::Tools => {
                let tools = app.gateway.get_tools_info();
                let max = tools.len().saturating_sub(1);
                app.tools_cursor = (app.tools_cursor + 1).min(max);
            }
            _ => {}
        },
        KeyCode::Enter => match app.settings_tab {
            SettingsTab::AgentConfig => {
                // Enter editing mode for the selected field
                let cfg = app.gateway.get_config();
                app.edit_buffer = match app.settings_cursor {
                    0 => cfg.agent.name.clone(),
                    1 => cfg.agent.model.clone(),
                    2 => cfg.agent.thinking_level.clone(),
                    3 => cfg.agent.api_key.clone().unwrap_or_default(),
                    _ => String::new(),
                };
                app.input_mode = InputMode::Editing;
            }
            _ => {}
        },
        KeyCode::Char(' ') => match app.settings_tab {
            SettingsTab::AgentConfig if app.settings_cursor == 3 => {
                app.show_api_key = !app.show_api_key;
            }
            SettingsTab::PathPolicies => {
                // Cycle access level for selected policy
                if let Some(policy) = app.path_policies.get(app.path_cursor) {
                    use bat_types::policy::AccessLevel;
                    let new_access = match policy.access {
                        AccessLevel::ReadOnly => "read-write",
                        AccessLevel::ReadWrite => "write-only",
                        AccessLevel::WriteOnly => "read-only",
                    };
                    let path_str = policy.path.to_string_lossy().to_string();
                    let recursive = policy.recursive;
                    // Delete and re-add with new access
                    if let Some(id) = policy.id {
                        let _ = app.gateway.delete_path_policy(id).await;
                    }
                    let _ = app.gateway.add_path_policy(&path_str, new_access, recursive).await;
                    app.refresh_path_policies().await;
                }
            }
            SettingsTab::Tools => {
                let tools = app.gateway.get_tools_info();
                if let Some(tool) = tools.get(app.tools_cursor) {
                    let _ = app.gateway.toggle_tool(&tool.name, !tool.enabled);
                }
            }
            _ => {}
        },
        KeyCode::Char('d') => {
            if app.settings_tab == SettingsTab::PathPolicies {
                if let Some(policy) = app.path_policies.get(app.path_cursor) {
                    if let Some(id) = policy.id {
                        let _ = app.gateway.delete_path_policy(id).await;
                    }
                    app.refresh_path_policies().await;
                    if app.path_cursor > 0 {
                        app.path_cursor -= 1;
                    }
                }
            }
        }
        KeyCode::Char('a') => {
            if app.settings_tab == SettingsTab::PathPolicies {
                // Enter editing mode to type a new path
                app.edit_buffer.clear();
                app.input_mode = InputMode::Editing;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_onboarding_key(app: &mut App, key: KeyEvent) -> Result<()> {
    // If editing a text field, capture keystrokes
    if app.onboarding_editing {
        match key.code {
            KeyCode::Enter => {
                app.onboarding_editing = false;
                // On step 3 (access), Enter adds the typed path
                if app.onboarding_step == 3 && !app.edit_buffer.is_empty() {
                    app.onboarding_folders.push((
                        app.edit_buffer.clone(),
                        "read-write".to_string(),
                        true,
                    ));
                    app.edit_buffer.clear();
                }
            }
            KeyCode::Esc => {
                app.onboarding_editing = false;
                app.edit_buffer.clear();
            }
            KeyCode::Char(c) => app.edit_buffer.push(c),
            KeyCode::Backspace => { app.edit_buffer.pop(); }
            _ => {}
        }
        return Ok(());
    }

    match app.onboarding_step {
        0 => {
            // Welcome — Enter to proceed
            if key.code == KeyCode::Enter {
                app.onboarding_step = 1;
                app.onboarding_editing = true;
                app.edit_buffer = app.onboarding_api_key.clone();
            }
        }
        1 => {
            // API Key
            match key.code {
                KeyCode::Enter if !app.onboarding_validated => {
                    // Start editing to enter/modify key
                    app.onboarding_editing = true;
                    app.edit_buffer = app.onboarding_api_key.clone();
                }
                KeyCode::Enter if app.onboarding_validated => {
                    app.onboarding_step = 2;
                    app.onboarding_editing = true;
                    app.edit_buffer = app.onboarding_name.clone();
                }
                KeyCode::Char('v') if !app.onboarding_api_key.is_empty() => {
                    // Validate
                    app.onboarding_error.clear();
                    match Gateway::validate_api_key(&app.onboarding_api_key).await {
                        Ok(()) => app.onboarding_validated = true,
                        Err(e) => app.onboarding_error = e.to_string(),
                    }
                }
                KeyCode::Char('e') => {
                    app.onboarding_editing = true;
                    app.edit_buffer = app.onboarding_api_key.clone();
                }
                KeyCode::Esc => app.onboarding_step = 0,
                _ => {}
            }
            // Save buffer back when done editing
            if !app.onboarding_editing && !app.edit_buffer.is_empty() {
                app.onboarding_api_key = app.edit_buffer.clone();
                app.edit_buffer.clear();
                app.onboarding_validated = false;
            }
        }
        2 => {
            // Name
            match key.code {
                KeyCode::Enter if !app.onboarding_name.is_empty() => {
                    app.onboarding_step = 3;
                }
                KeyCode::Enter => {
                    app.onboarding_editing = true;
                    app.edit_buffer = app.onboarding_name.clone();
                }
                KeyCode::Char('e') => {
                    app.onboarding_editing = true;
                    app.edit_buffer = app.onboarding_name.clone();
                }
                KeyCode::Esc => {
                    app.onboarding_step = 1;
                }
                _ => {}
            }
            if !app.onboarding_editing && !app.edit_buffer.is_empty() {
                app.onboarding_name = app.edit_buffer.clone();
                app.edit_buffer.clear();
            }
        }
        3 => {
            // Access — add folders
            match key.code {
                KeyCode::Char('a') => {
                    app.onboarding_editing = true;
                    app.edit_buffer.clear();
                }
                KeyCode::Char('d') => {
                    // Delete last folder
                    app.onboarding_folders.pop();
                }
                KeyCode::Enter if !app.onboarding_folders.is_empty() => {
                    app.onboarding_step = 4;
                }
                KeyCode::Esc => app.onboarding_step = 2,
                _ => {}
            }
        }
        4 => {
            // Ready — finish
            match key.code {
                KeyCode::Enter => {
                    app.onboarding_error.clear();
                    match app.gateway.complete_onboarding(
                        app.onboarding_name.clone(),
                        app.onboarding_api_key.clone(),
                        None, // OpenAI key can be added later in Settings
                        app.onboarding_folders.clone(),
                    ).await {
                        Ok(()) => {
                            app.screen = Screen::Chat;
                            // Refresh policies
                            app.path_policies = app.gateway.get_path_policies_sync().unwrap_or_default();
                        }
                        Err(e) => app.onboarding_error = e.to_string(),
                    }
                }
                KeyCode::Esc => app.onboarding_step = 3,
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_logs_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Tab => {
            app.screen = Screen::Memory;
            app.refresh_memory().await;
            if !app.memory_files.is_empty() {
                app.load_selected_memory_file().await;
            }
        }
        KeyCode::Esc => {
            app.screen = Screen::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.logs_scroll = app.logs_scroll.saturating_add(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.logs_scroll = app.logs_scroll.saturating_sub(1);
        }
        KeyCode::Char('G') => {
            app.logs_scroll = 0; // Jump to bottom (newest)
        }
        KeyCode::Char('g') => {
            // Jump to top (oldest)
            app.logs_scroll = app.audit_entries.len().saturating_sub(1);
        }
        _ => {}
    }
    Ok(())
}

async fn handle_memory_key(app: &mut App, key: KeyEvent) -> Result<()> {
    // If editing a memory file, handle editing keys
    if app.memory_editing {
        match key.code {
            KeyCode::Esc => {
                app.memory_editing = false;
                app.memory_edit_content = app.memory_content.clone();
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Save
                if let Some(file) = app.memory_files.get(app.memory_cursor) {
                    let name = file.name.clone();
                    let content = app.memory_edit_content.clone();
                    if let Err(e) = app.gateway.write_memory_file(&name, &content) {
                        warn!("Failed to save memory file: {e}");
                    } else {
                        app.memory_content = content;
                        app.memory_editing = false;
                        app.refresh_memory().await;
                    }
                }
            }
            KeyCode::Char(c) => {
                app.memory_edit_content.push(c);
            }
            KeyCode::Enter => {
                app.memory_edit_content.push('\n');
            }
            KeyCode::Backspace => {
                app.memory_edit_content.pop();
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Tab => {
            app.screen = Screen::Activity;
            app.refresh_subagents().await;
        }
        KeyCode::Esc => {
            app.screen = Screen::Chat;
        }
        KeyCode::Up => {
            if !app.memory_files.is_empty() {
                app.memory_cursor = app.memory_cursor.saturating_sub(1);
                app.load_selected_memory_file().await;
            }
        }
        KeyCode::Down => {
            if !app.memory_files.is_empty() {
                let max = app.memory_files.len().saturating_sub(1);
                if app.memory_cursor < max {
                    app.memory_cursor += 1;
                    app.load_selected_memory_file().await;
                }
            }
        }
        KeyCode::Char('e') => {
            if !app.memory_files.is_empty() {
                app.memory_editing = true;
                app.memory_edit_content = app.memory_content.clone();
            }
        }
        KeyCode::Char('c') => {
            if !app.memory_consolidating {
                app.memory_consolidating = true;
                app.memory_consolidation_result.clear();
                let gw = app.gateway.clone();
                match gw.trigger_consolidation().await {
                    Ok(result) => {
                        let summary = if result.files_updated.is_empty() {
                            "No updates needed".to_string()
                        } else {
                            format!("Updated: {}", result.files_updated.join(", "))
                        };
                        app.memory_consolidation_result = summary;
                        app.refresh_memory().await;
                        app.load_selected_memory_file().await;
                    }
                    Err(e) => {
                        app.memory_consolidation_result = format!("Error: {e}");
                    }
                }
                app.memory_consolidating = false;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_editing_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let value = app.edit_buffer.clone();
            app.input_mode = InputMode::Normal;

            match app.settings_tab {
                SettingsTab::AgentConfig => {
                    let mut cfg = app.gateway.get_config();
                    match app.settings_cursor {
                        0 => cfg.agent.name = value,
                        1 => cfg.agent.model = value,
                        2 => cfg.agent.thinking_level = value,
                        3 => {
                            cfg.agent.api_key = if value.is_empty() {
                                None
                            } else {
                                Some(value)
                            };
                        }
                        _ => {}
                    }
                    let _ = app.gateway.update_config(cfg);
                }
                SettingsTab::PathPolicies => {
                    // Adding a new path policy
                    if !value.is_empty() {
                        let _ = app
                            .gateway
                            .add_path_policy(&value, "read-write", true)
                            .await;
                        app.refresh_path_policies().await;
                    }
                }
                _ => {}
            }
            app.edit_buffer.clear();
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.edit_buffer.clear();
        }
        KeyCode::Char(c) => {
            app.edit_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_usage_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Tab => {
            app.screen = Screen::Chat;
        }
        KeyCode::Esc => {
            app.screen = Screen::Chat;
        }
        KeyCode::Char('r') => {
            app.refresh_usage();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_activity_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Tab => {
            app.screen = Screen::Usage;
            app.refresh_usage();
        }
        KeyCode::Esc => {
            app.screen = Screen::Chat;
        }
        KeyCode::Char('r') => {
            app.refresh_subagents().await;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.activity_cursor = app.activity_cursor.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.subagents.is_empty() {
                app.activity_cursor = (app.activity_cursor + 1).min(app.subagents.len() - 1);
            }
        }
        KeyCode::Enter => {
            app.activity_expanded = !app.activity_expanded;
        }
        _ => {}
    }
    Ok(())
}
