pub mod fs_read;
pub mod fs_write;
pub mod fs_list;
pub mod web_fetch;
pub mod shell_run;
pub mod exec_run;
pub mod exec_output;
pub mod exec_write;
pub mod exec_kill;
pub mod exec_list;
pub mod app_open;
pub mod system_info;
pub mod session_spawn;
pub mod session_status;
pub mod clipboard;
pub mod screenshot;
pub mod ask_orchestrator;
pub mod session_answer;

use anyhow::Result;
use bat_types::message::{ToolCall, ToolResult};
use bat_types::policy::PathPolicy;

use crate::gateway_bridge::GatewayBridge;

/// Tool executor trait — each tool implements this.
pub trait ToolExecutor: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    fn execute(&self, input: &serde_json::Value) -> Result<String>;
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: Vec<Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: vec![] }
    }

    /// Create a registry with only orchestrator (session management) tools.
    pub fn with_orchestrator_tools(bridge: GatewayBridge, disabled: &[String]) -> Self {
        let mut reg = Self::new();

        // Only include session management tools for orchestrator
        if !disabled.contains(&"session_spawn".to_string()) {
            reg.register(Box::new(session_spawn::SessionSpawn::new(bridge.clone())));
        }
        if !disabled.contains(&"session_status".to_string()) {
            reg.register(Box::new(session_status::SessionStatus::new(bridge.clone())));
        }
        if !disabled.contains(&"session_answer".to_string()) {
            reg.register(Box::new(session_answer::SessionAnswer::new(bridge)));
        }

        // TODO: Add session_pause, session_resume, session_instruct, session_cancel, session_answer
        // These will be implemented in Phase C

        reg
    }

    /// Create a registry with all default tools, skipping any in `disabled`.
    pub fn with_default_tools(policies: Vec<PathPolicy>, disabled: &[String], bridge: Option<GatewayBridge>) -> Self {
        let mut reg = Self::new();
        if !disabled.contains(&"fs_read".to_string()) {
            reg.register(Box::new(fs_read::FsRead::new(policies.clone())));
        }
        if !disabled.contains(&"fs_write".to_string()) {
            reg.register(Box::new(fs_write::FsWrite::new(policies.clone())));
        }
        if !disabled.contains(&"fs_list".to_string()) {
            reg.register(Box::new(fs_list::FsList::new(policies)));
        }
        if !disabled.contains(&"web_fetch".to_string()) {
            reg.register(Box::new(web_fetch::WebFetch::new()));
        }
        if !disabled.contains(&"shell_run".to_string()) {
            reg.register(Box::new(shell_run::ShellRun::new()));
        }
        if !disabled.contains(&"app_open".to_string()) {
            reg.register(Box::new(app_open::AppOpen::new()));
        }
        if !disabled.contains(&"system_info".to_string()) {
            reg.register(Box::new(system_info::SystemInfo::new()));
        }
        if !disabled.contains(&"clipboard".to_string()) {
            reg.register(Box::new(clipboard::Clipboard::new()));
        }
        if !disabled.contains(&"screenshot".to_string()) {
            reg.register(Box::new(screenshot::Screenshot::new()));
        }
        // Exec tools require a gateway bridge for IPC
        if let Some(bridge) = bridge {
            if !disabled.contains(&"exec_run".to_string()) {
                reg.register(Box::new(exec_run::ExecRun::new(bridge.clone())));
            }
            if !disabled.contains(&"exec_output".to_string()) {
                reg.register(Box::new(exec_output::ExecOutput::new(bridge.clone())));
            }
            if !disabled.contains(&"exec_write".to_string()) {
                reg.register(Box::new(exec_write::ExecWrite::new(bridge.clone())));
            }
            if !disabled.contains(&"exec_kill".to_string()) {
                reg.register(Box::new(exec_kill::ExecKill::new(bridge.clone())));
            }
            if !disabled.contains(&"exec_list".to_string()) {
                reg.register(Box::new(exec_list::ExecList::new(bridge.clone())));
            }
            if !disabled.contains(&"session_spawn".to_string()) {
                reg.register(Box::new(session_spawn::SessionSpawn::new(bridge.clone())));
            }
            if !disabled.contains(&"session_status".to_string()) {
                reg.register(Box::new(session_status::SessionStatus::new(bridge.clone())));
            }
            if !disabled.contains(&"ask_orchestrator".to_string()) {
                reg.register(Box::new(ask_orchestrator::AskOrchestrator::new(bridge)));
            }
        }
        reg
    }

    pub fn register(&mut self, tool: Box<dyn ToolExecutor>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn ToolExecutor> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
    }

    /// Execute a tool call, returning a ToolResult.
    pub fn execute(&self, call: &ToolCall) -> ToolResult {
        let result = self
            .get(&call.name)
            .map(|t| t.execute(&call.input))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Unknown tool: {}", call.name)));

        match result {
            Ok(output) => ToolResult {
                tool_call_id: call.id.clone(),
                content: output,
                is_error: false,
            },
            Err(e) => ToolResult {
                tool_call_id: call.id.clone(),
                content: e.to_string(),
                is_error: true,
            },
        }
    }

    /// Returns Anthropic-format tool definitions for the API.
    pub fn definitions(&self) -> Vec<serde_json::Value> {
        self.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.input_schema(),
                })
            })
            .collect()
    }
}
