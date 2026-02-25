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
        #[serde(default)]
        session_kind: String,  // "main" or "subagent" - used to decide tool registry
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
    /// Answer to a sub-agent's question.
    Answer {
        question_id: String,
        answer: String,
    },
    /// Instruction sent to a running sub-agent.
    Instruction {
        instruction_id: String,
        content: String,
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
    /// Question from a sub-agent to its orchestrator.
    Question {
        question_id: String,
        question: String,
        context: String,
        blocking: bool,  // if true, agent waits for answer before continuing
    },
    /// Progress update from a sub-agent.
    Progress {
        summary: String,
        percent: Option<f32>,
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
    /// Ask a question to the orchestrator (sub-agent only).
    AskOrchestrator {
        question: String,
        context: String,
        blocking: bool,
    },
    /// Pause a running sub-agent.
    PauseSubagent {
        session_key: String,
    },
    /// Resume a paused sub-agent with optional new instructions.
    ResumeSubagent {
        session_key: String,
        instructions: Option<String>,
    },
    /// Send new instructions to a running sub-agent.
    InstructSubagent {
        session_key: String,
        instruction: String,
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
    /// Answer from the orchestrator.
    OrchestratorAnswer {
        answer: String,
    },
    /// Sub-agent paused.
    SubagentPaused,
    /// Sub-agent resumed.
    SubagentResumed,
    /// Instruction sent to sub-agent.
    SubagentInstructed,
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
