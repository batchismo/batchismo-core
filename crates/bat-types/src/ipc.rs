use serde::{Deserialize, Serialize};

use crate::message::{Message, ToolCall, ToolResult};
use crate::policy::PathPolicy;

/// Gateway → Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GatewayToAgent {
    Init {
        session_id: String,
        model: String,
        system_prompt: String,
        history: Vec<Message>,
        path_policies: Vec<PathPolicy>,
        #[serde(default)]
        disabled_tools: Vec<String>,
    },
    UserMessage {
        content: String,
    },
    Cancel,
}

/// Agent → Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentToGateway {
    TextDelta {
        content: String,
    },
    ToolCallStart {
        tool_call: ToolCall,
    },
    ToolCallResult {
        result: ToolResult,
    },
    TurnComplete {
        message: Message,
    },
    Error {
        message: String,
    },
    AuditLog {
        level: String,
        category: String,
        event: String,
        summary: String,
        detail_json: Option<String>,
    },
}
