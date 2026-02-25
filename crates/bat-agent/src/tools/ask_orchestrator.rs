use anyhow::Result;
use serde_json::{Value, json};
use crate::gateway_bridge::GatewayBridge;
use bat_types::ipc::{ProcessAction, ProcessResult};

pub struct AskOrchestrator {
    bridge: GatewayBridge,
}

impl AskOrchestrator {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for AskOrchestrator {
    fn name(&self) -> &str { "ask_orchestrator" }

    fn description(&self) -> &str {
        "Ask a question to your orchestrator. Use when you need clarification or guidance."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question you want to ask the orchestrator"
                },
                "context": {
                    "type": "string",
                    "description": "Context about what you're doing and why you need this information"
                },
                "blocking": {
                    "type": "boolean",
                    "description": "Whether to wait for an answer before continuing (default: true)",
                    "default": true
                }
            },
            "required": ["question", "context"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let question = input.get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'question' parameter"))?;

        let context = input.get("context")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'context' parameter"))?;

        let blocking = input.get("blocking")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let action = ProcessAction::AskOrchestrator {
            question: question.to_string(),
            context: context.to_string(),
            blocking,
        };

        match self.bridge.request(action) {
            ProcessResult::OrchestratorAnswer { answer } => {
                Ok(json!({
                    "status": "answered",
                    "answer": answer,
                    "message": "Received answer from orchestrator"
                }).to_string())
            }
            ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to ask orchestrator: {message}"))
            }
            other => {
                Err(anyhow::anyhow!("Unexpected response: {other:?}"))
            }
        }
    }
}