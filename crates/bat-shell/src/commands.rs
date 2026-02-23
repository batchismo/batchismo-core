/// Tauri IPC commands — called from the frontend via `invoke()`.

use tauri::State;

use bat_gateway::ToolInfo;
use bat_types::{
    audit::{AuditEntry, AuditFilter, AuditStats},
    config::BatConfig,
    memory::{MemoryFileInfo, Observation, ObservationFilter, ObservationSummary},
    message::Message,
    policy::PathPolicy,
    session::SessionMeta,
};

use crate::AppState;

/// Send a user message to the main session's agent.
/// Returns immediately; streaming events arrive via the "bat-event" Tauri event.
#[tauri::command]
pub async fn send_message(
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .gateway
        .send_user_message(&content)
        .await
        .map_err(|e| e.to_string())
}

/// Get the full message history for the main session.
#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
) -> Result<Vec<Message>, String> {
    state
        .gateway
        .get_main_history()
        .await
        .map_err(|e| e.to_string())
}

/// Get the main session metadata (id, model, token counts, status).
#[tauri::command]
pub async fn get_session(
    state: State<'_, AppState>,
) -> Result<SessionMeta, String> {
    state
        .gateway
        .get_main_session()
        .await
        .map_err(|e| e.to_string())
}

/// List all user sessions.
#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<SessionMeta>, String> {
    state.gateway.list_sessions().map_err(|e| e.to_string())
}

/// Create a new named session.
#[tauri::command]
pub fn create_session(
    name: String,
    state: State<'_, AppState>,
) -> Result<SessionMeta, String> {
    state.gateway.create_named_session(&name).map_err(|e| e.to_string())
}

/// Switch active session.
#[tauri::command]
pub fn switch_session(
    key: String,
    state: State<'_, AppState>,
) -> Result<SessionMeta, String> {
    state.gateway.switch_session(&key).map_err(|e| e.to_string())
}

/// Delete a session.
#[tauri::command]
pub fn delete_session_by_key(
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.gateway.delete_session(&key).map_err(|e| e.to_string())
}

/// Rename a session.
#[tauri::command]
pub fn rename_session(
    old_key: String,
    new_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.gateway.rename_session(&old_key, &new_key).map_err(|e| e.to_string())
}

/// Get the active session key.
#[tauri::command]
pub fn get_active_session_key(
    state: State<'_, AppState>,
) -> String {
    state.gateway.active_session_key()
}

/// Get token usage statistics.
#[tauri::command]
pub fn get_usage_stats(
    state: State<'_, AppState>,
) -> Result<bat_types::usage::UsageStats, String> {
    state.gateway.get_usage_stats().map_err(|e| e.to_string())
}

/// Get all subagent sessions.
#[tauri::command]
pub async fn get_subagents(
    state: State<'_, AppState>,
) -> Result<Vec<bat_types::session::SubagentInfo>, String> {
    state
        .gateway
        .get_subagents()
        .await
        .map_err(|e| e.to_string())
}

/// Get all configured path policies.
#[tauri::command]
pub async fn get_path_policies(
    state: State<'_, AppState>,
) -> Result<Vec<PathPolicy>, String> {
    state
        .gateway
        .get_path_policies()
        .await
        .map_err(|e| e.to_string())
}

/// Add a new path policy.
#[tauri::command]
pub async fn add_path_policy(
    path: String,
    access: String,
    recursive: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .gateway
        .add_path_policy(&path, &access, recursive)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a path policy by its path string.
#[tauri::command]
pub async fn delete_path_policy(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .gateway
        .delete_path_policy(id)
        .await
        .map_err(|e| e.to_string())
}

/// Get info about all registered tools (name, description, enabled state).
#[tauri::command]
pub fn get_tools(state: State<'_, AppState>) -> Vec<ToolInfo> {
    state.gateway.get_tools_info()
}

/// Toggle a tool on or off.
#[tauri::command]
pub fn toggle_tool(
    name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .gateway
        .toggle_tool(&name, enabled)
        .map_err(|e| e.to_string())
}

/// Get the current agent configuration.
#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> BatConfig {
    state.gateway.get_config()
}

/// Update the agent configuration (persisted to disk).
#[tauri::command]
pub fn update_config(
    config: BatConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .gateway
        .update_config(config)
        .map_err(|e| e.to_string())
}

/// Build and return the current system prompt text (for preview in Settings).
#[tauri::command]
pub fn get_system_prompt(state: State<'_, AppState>) -> Result<String, String> {
    state
        .gateway
        .get_system_prompt()
        .map_err(|e| e.to_string())
}

// ─── Onboarding ────────────────────────────────────────────────────────

/// Check if onboarding has been completed.
#[tauri::command]
pub fn is_onboarding_complete(state: State<'_, AppState>) -> bool {
    state.gateway.is_onboarding_complete()
}

/// Validate an Anthropic API key.
#[tauri::command]
pub async fn validate_api_key(key: String) -> Result<bool, String> {
    bat_gateway::Gateway::validate_api_key(&key)
        .await
        .map(|_| true)
        .map_err(|e| e.to_string())
}

/// Complete the onboarding wizard.
#[tauri::command]
pub async fn complete_onboarding(
    name: String,
    api_key: String,
    folders: Vec<(String, String, bool)>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .gateway
        .complete_onboarding(name, api_key, folders)
        .await
        .map_err(|e| e.to_string())
}

// ─── Memory / Observations ─────────────────────────────────────────────

/// List workspace memory files.
#[tauri::command]
pub fn get_memory_files(state: State<'_, AppState>) -> Result<Vec<MemoryFileInfo>, String> {
    state.gateway.list_memory_files().map_err(|e| e.to_string())
}

/// Read a specific memory file.
#[tauri::command]
pub fn get_memory_file(name: String, state: State<'_, AppState>) -> Result<String, String> {
    state.gateway.read_memory_file(&name).map_err(|e| e.to_string())
}

/// Write/update a memory file.
#[tauri::command]
pub fn update_memory_file(name: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    state.gateway.write_memory_file(&name, &content).map_err(|e| e.to_string())
}

/// Query observations.
#[tauri::command]
pub fn get_observations(filter: ObservationFilter, state: State<'_, AppState>) -> Result<Vec<Observation>, String> {
    state.gateway.get_observations(&filter).map_err(|e| e.to_string())
}

/// Get observation summary.
#[tauri::command]
pub fn get_observation_summary(state: State<'_, AppState>) -> Result<ObservationSummary, String> {
    state.gateway.get_observation_summary().map_err(|e| e.to_string())
}

/// Trigger memory consolidation.
#[tauri::command]
pub async fn trigger_consolidation(state: State<'_, AppState>) -> Result<String, String> {
    let result = state
        .gateway
        .trigger_consolidation()
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "{} observations processed, {} files updated",
        result.observations_processed,
        result.files_updated.join(", ")
    ))
}

// ─── Audit ─────────────────────────────────────────────────────────────

/// Query audit log entries with optional filters.
#[tauri::command]
pub fn get_audit_logs(filter: AuditFilter, state: State<'_, AppState>) -> Result<Vec<AuditEntry>, String> {
    state
        .gateway
        .query_audit_log(&filter)
        .map_err(|e| e.to_string())
}

/// Get audit log summary statistics.
#[tauri::command]
pub fn get_audit_stats(state: State<'_, AppState>) -> Result<AuditStats, String> {
    state
        .gateway
        .get_audit_stats()
        .map_err(|e| e.to_string())
}
