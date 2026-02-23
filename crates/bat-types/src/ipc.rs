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
    /// Response to a process management request from the agent.
    ProcessResponse {
        request_id: String,
        result: ProcessResult,
    },
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
    /// Request process management from the gateway.
    ProcessRequest {
        request_id: String,
        action: ProcessAction,
    },
}

/// Process management actions the agent can request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ProcessAction {
    /// Start a new process. If background=true, returns immediately with session_id.
    Start {
        command: String,
        #[serde(default)]
        workdir: Option<String>,
        #[serde(default)]
        background: bool,
    },
    /// Get output from a managed process.
    GetOutput {
        session_id: String,
    },
    /// Write to stdin of a running process.
    WriteStdin {
        session_id: String,
        data: String,
    },
    /// Kill a running process.
    Kill {
        session_id: String,
    },
    /// List all managed processes.
    List,
    /// Spawn a subagent.
    SpawnSubagent {
        task: String,
        label: Option<String>,
    },
    /// Get status of subagents.
    ListSubagents,
    /// Cancel a running subagent.
    CancelSubagent {
        session_key: String,
    },
}

/// Result of a process management request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ProcessResult {
    Started {
        session_id: String,
    },
    Output {
        session_id: String,
        stdout: String,
        stderr: String,
        is_running: bool,
        exit_code: Option<i32>,
    },
    Written,
    Killed,
    ProcessList {
        processes: Vec<ProcessInfo>,
    },
    Error {
        message: String,
    },
    SubagentSpawned {
        session_key: String,
        session_id: String,
    },
    SubagentList {
        subagents: Vec<crate::session::SubagentInfo>,
    },
    SubagentCancelled,
}

/// Info about a managed process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub session_id: String,
    pub command: String,
    pub is_running: bool,
    pub exit_code: Option<i32>,
    pub started_at: String,
}
