pub mod fs_read;
pub mod fs_write;
pub mod fs_list;

use anyhow::Result;
use bat_types::message::{ToolCall, ToolResult};
use bat_types::policy::PathPolicy;

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

    /// Create a registry with the default filesystem tools, skipping any in `disabled`.
    pub fn with_fs_tools(policies: Vec<PathPolicy>, disabled: &[String]) -> Self {
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
