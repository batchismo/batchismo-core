use anyhow::Result;
use serde_json::{Value, json};
use uuid::Uuid;
use crate::gateway_bridge::GatewayBridge;

pub struct SessionAnswer {
    #[allow(dead_code)]
    bridge: GatewayBridge,
}

impl SessionAnswer {
    pub fn new(bridge: GatewayBridge) -> Self {
        Self { bridge }
    }
}

impl super::ToolExecutor for SessionAnswer {
    fn name(&self) -> &str { "session_answer" }

    fn description(&self) -> &str {
        "Answer a pending question from a sub-agent. Use the session_key to identify which sub-agent to answer."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "The session key of the sub-agent that asked the question"
                },
                "answer": {
                    "type": "string",
                    "description": "Your answer to the sub-agent's question"
                }
            },
            "required": ["session_key", "answer"]
        })
    }

    fn execute(&self, input: &Value) -> Result<String> {
        let session_key = input.get("session_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'session_key' parameter"))?;

        let answer = input.get("answer")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'answer' parameter"))?;

        // Generate a question ID for tracking (in the future we'll get this from the question)
        let question_id = Uuid::new_v4().to_string();

        // Send the answer request to the gateway
        let action = bat_types::ipc::ProcessAction::AnswerSubagent {
            session_key: session_key.to_string(),
            question_id,
            answer: answer.to_string(),
        };

        let result = self.bridge.request(action);
        match result {
            bat_types::ipc::ProcessResult::SubagentAnswered => {
                Ok(format!("Answer successfully sent to sub-agent {}: {}", session_key, answer))
            },
            bat_types::ipc::ProcessResult::Error { message } => {
                Err(anyhow::anyhow!("Failed to answer sub-agent: {}", message))
            },
            _ => {
                Err(anyhow::anyhow!("Unexpected response when answering sub-agent"))
            }
        }
    }
}