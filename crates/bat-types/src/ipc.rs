use serde::{Deserialize, Serialize};

use crate::message::{ImageAttachment, Message, ToolCall, ToolResult};
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
        #[serde(default)]
        images: Vec<ImageAttachment>,
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
    /// Answer a question from a sub-agent.
    AnswerSubagent {
        session_key: String,
        question_id: String,
        answer: String,
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
    /// Answer sent to sub-agent.
    SubagentAnswered,
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_question_message_serialization() {
        let question_msg = AgentToGateway::Question {
            question_id: "test-q-123".to_string(),
            question: "What should I do next?".to_string(),
            context: "I'm processing files and need guidance".to_string(),
            blocking: true,
        };

        // Test serialization
        let json = serde_json::to_string(&question_msg).unwrap();
        assert!(json.contains("What should I do next?"));
        assert!(json.contains("blocking"));

        // Test deserialization
        let deserialized: AgentToGateway = serde_json::from_str(&json).unwrap();
        match deserialized {
            AgentToGateway::Question { question_id, question, context, blocking } => {
                assert_eq!(question_id, "test-q-123");
                assert_eq!(question, "What should I do next?");
                assert_eq!(context, "I'm processing files and need guidance");
                assert_eq!(blocking, true);
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_answer_message_serialization() {
        let answer_msg = GatewayToAgent::Answer {
            question_id: "test-q-123".to_string(),
            answer: "Continue with the current approach".to_string(),
        };

        // Test serialization
        let json = serde_json::to_string(&answer_msg).unwrap();
        assert!(json.contains("Continue with the current approach"));

        // Test deserialization
        let deserialized: GatewayToAgent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayToAgent::Answer { question_id, answer } => {
                assert_eq!(question_id, "test-q-123");
                assert_eq!(answer, "Continue with the current approach");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_progress_message_serialization() {
        let progress_msg = AgentToGateway::Progress {
            summary: "Processed 7 of 14 files".to_string(),
            percent: Some(50.0),
        };

        // Test serialization
        let json = serde_json::to_string(&progress_msg).unwrap();
        assert!(json.contains("Processed 7 of 14 files"));
        assert!(json.contains("50"));

        // Test deserialization
        let deserialized: AgentToGateway = serde_json::from_str(&json).unwrap();
        match deserialized {
            AgentToGateway::Progress { summary, percent } => {
                assert_eq!(summary, "Processed 7 of 14 files");
                assert_eq!(percent, Some(50.0));
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_instruction_message_serialization() {
        let instruction_msg = GatewayToAgent::Instruction {
            instruction_id: "inst-456".to_string(),
            content: "Change approach to use JSON format instead".to_string(),
        };

        // Test serialization
        let json = serde_json::to_string(&instruction_msg).unwrap();
        assert!(json.contains("Change approach to use JSON format"));

        // Test deserialization
        let deserialized: GatewayToAgent = serde_json::from_str(&json).unwrap();
        match deserialized {
            GatewayToAgent::Instruction { instruction_id, content } => {
                assert_eq!(instruction_id, "inst-456");
                assert_eq!(content, "Change approach to use JSON format instead");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_lifecycle_process_actions() {
        // Test PauseSubagent
        let pause_action = ProcessAction::PauseSubagent {
            session_key: "subagent-123".to_string(),
        };
        let json = serde_json::to_string(&pause_action).unwrap();
        assert!(json.contains("PauseSubagent"));
        assert!(json.contains("subagent-123"));

        // Test ResumeSubagent
        let resume_action = ProcessAction::ResumeSubagent {
            session_key: "subagent-123".to_string(),
            instructions: Some("Continue with new parameters".to_string()),
        };
        let json = serde_json::to_string(&resume_action).unwrap();
        assert!(json.contains("ResumeSubagent"));
        assert!(json.contains("Continue with new parameters"));

        // Test deserialization roundtrip
        let deserialized: ProcessAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            ProcessAction::ResumeSubagent { session_key, instructions } => {
                assert_eq!(session_key, "subagent-123");
                assert_eq!(instructions, Some("Continue with new parameters".to_string()));
            }
            _ => panic!("Wrong action type after deserialization"),
        }
    }
}
