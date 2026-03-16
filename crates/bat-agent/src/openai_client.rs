//! OpenAI-compatible chat client — used for Ollama and OpenAI API.
//!
//! Ollama exposes an OpenAI-compatible API at `/v1/chat/completions`.
//! This client translates between the internal Anthropic-style types
//! (used throughout bat-agent) and the OpenAI chat completions format.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use crate::llm::{ChatResponse, ContentBlock, Usage};

// ─── OpenAI request/response types ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoiceMessage {
    #[allow(dead_code)]
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
}

// ─── SSE streaming types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct StreamChunk {
    id: String,
    choices: Vec<StreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCall {
    index: usize,
    id: Option<String>,
    function: Option<StreamFunction>,
}

#[derive(Debug, Deserialize)]
struct StreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

// ─── Client ──────────────────────────────────────────────────────────────────

pub struct OpenAICompatibleClient {
    client: reqwest::Client,
    api_key: Option<String>,
    base_url: String,
}

impl OpenAICompatibleClient {
    /// Create a client for Ollama (no API key needed).
    pub fn ollama(endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: None,
            base_url: format!("{}/v1", endpoint.trim_end_matches('/')),
        }
    }

    /// Create a client for OpenAI.
    #[allow(dead_code)]
    pub fn openai(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Some(api_key),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Convert Anthropic-style messages + system prompt to OpenAI format.
    fn build_openai_messages(
        system: &str,
        messages: &[crate::llm::AnthropicMessage],
    ) -> Vec<OpenAIMessage> {
        let mut result = vec![OpenAIMessage {
            role: "system".to_string(),
            content: serde_json::Value::String(system.to_string()),
        }];
        for msg in messages {
            result.push(OpenAIMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }
        result
    }

    /// Convert OpenAI tool definitions from Anthropic format.
    /// Anthropic format: { name, description, input_schema }
    /// OpenAI format: { type: "function", function: { name, description, parameters } }
    fn convert_tools(anthropic_tools: &[serde_json::Value]) -> Vec<serde_json::Value> {
        anthropic_tools
            .iter()
            .filter_map(|tool| {
                let name = tool.get("name")?.as_str()?;
                let description = tool.get("description")?.as_str()?;
                let parameters = tool.get("input_schema").cloned()
                    .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));
                Some(serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": name,
                        "description": description,
                        "parameters": parameters,
                    }
                }))
            })
            .collect()
    }

    /// Non-streaming chat request. Returns in Anthropic-compatible format.
    pub async fn chat(&self, request: &crate::llm::ChatRequest) -> Result<ChatResponse> {
        let openai_messages = Self::build_openai_messages(&request.system, &request.messages);
        let openai_tools = Self::convert_tools(&request.tools);

        let openai_req = OpenAIRequest {
            model: request.model.clone(),
            messages: openai_messages,
            max_tokens: request.max_tokens,
            stream: false,
            tools: openai_tools,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let mut req_builder = self.client.post(&url)
            .header("content-type", "application/json");

        if let Some(ref key) = self.api_key {
            req_builder = req_builder.header("authorization", format!("Bearer {}", key));
        }

        let response = req_builder
            .json(&openai_req)
            .send()
            .await
            .context("Failed to send request to OpenAI-compatible API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI-compatible API error ({}): {}", status, body);
        }

        let oai_resp: OpenAIResponse = response.json().await
            .context("Failed to parse OpenAI-compatible API response")?;

        self.convert_response(oai_resp)
    }

    /// Streaming chat request. Text deltas sent via `text_tx`.
    pub async fn chat_streaming(
        &self,
        request: &crate::llm::ChatRequest,
        text_tx: Sender<String>,
    ) -> Result<(ChatResponse, String)> {
        let openai_messages = Self::build_openai_messages(&request.system, &request.messages);
        let openai_tools = Self::convert_tools(&request.tools);

        let openai_req = OpenAIRequest {
            model: request.model.clone(),
            messages: openai_messages,
            max_tokens: request.max_tokens,
            stream: true,
            tools: openai_tools,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let mut req_builder = self.client.post(&url)
            .header("content-type", "application/json");

        if let Some(ref key) = self.api_key {
            req_builder = req_builder.header("authorization", format!("Bearer {}", key));
        }

        let response = req_builder
            .json(&openai_req)
            .send()
            .await
            .context("Failed to send streaming request to OpenAI-compatible API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI-compatible API error ({}): {}", status, body);
        }

        let mut stream = response.bytes_stream();
        let mut sse_buffer = String::new();
        let mut message_id = String::new();
        let mut full_text = String::new();
        let mut finish_reason: Option<String> = None;
        let mut total_usage: Option<OpenAIUsage> = None;

        // Tool call accumulators: index -> (id, name, arguments)
        let mut tool_accums: Vec<(String, String, String)> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading streaming response")?;
            sse_buffer.push_str(&String::from_utf8_lossy(&chunk));

            loop {
                if let Some(pos) = sse_buffer.find("\n\n") {
                    let message = sse_buffer[..pos].to_string();
                    sse_buffer = sse_buffer[pos + 2..].to_string();

                    for line in message.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                                if message_id.is_empty() {
                                    message_id = chunk.id;
                                }
                                if let Some(usage) = chunk.usage {
                                    total_usage = Some(usage);
                                }
                                for choice in &chunk.choices {
                                    if let Some(ref text) = choice.delta.content {
                                        full_text.push_str(text);
                                        let _ = text_tx.try_send(text.clone());
                                    }
                                    if let Some(ref reason) = choice.finish_reason {
                                        finish_reason = Some(reason.clone());
                                    }
                                    if let Some(ref tool_calls) = choice.delta.tool_calls {
                                        for tc in tool_calls {
                                            while tool_accums.len() <= tc.index {
                                                tool_accums.push((String::new(), String::new(), String::new()));
                                            }
                                            if let Some(ref id) = tc.id {
                                                tool_accums[tc.index].0 = id.clone();
                                            }
                                            if let Some(ref f) = tc.function {
                                                if let Some(ref name) = f.name {
                                                    tool_accums[tc.index].1 = name.clone();
                                                }
                                                if let Some(ref args) = f.arguments {
                                                    tool_accums[tc.index].2.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // Build content blocks
        let mut content = Vec::new();
        if !full_text.is_empty() {
            content.push(ContentBlock::Text { text: full_text.clone() });
        }
        for (id, name, args) in tool_accums {
            if !name.is_empty() {
                let input = serde_json::from_str(&args)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                content.push(ContentBlock::ToolUse { id, name, input });
            }
        }

        // Map finish_reason to Anthropic-style stop_reason
        let stop_reason = match finish_reason.as_deref() {
            Some("tool_calls") => Some("tool_use".to_string()),
            Some("stop") => Some("end_turn".to_string()),
            Some("length") => Some("max_tokens".to_string()),
            other => other.map(|s| s.to_string()),
        };

        let usage = total_usage.map(|u| Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        }).unwrap_or(Usage { input_tokens: 0, output_tokens: 0 });

        let resp = ChatResponse {
            id: message_id,
            content,
            stop_reason,
            usage,
        };

        Ok((resp, full_text))
    }

    fn convert_response(&self, oai: OpenAIResponse) -> Result<ChatResponse> {
        let choice = oai.choices.into_iter().next()
            .context("No choices in OpenAI response")?;

        let mut content = Vec::new();

        if let Some(text) = choice.message.content {
            if !text.is_empty() {
                content.push(ContentBlock::Text { text });
            }
        }

        if let Some(tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                let input = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                content.push(ContentBlock::ToolUse {
                    id: tc.id,
                    name: tc.function.name,
                    input,
                });
            }
        }

        // Map finish_reason to Anthropic-style stop_reason
        let stop_reason = match choice.finish_reason.as_deref() {
            Some("tool_calls") => Some("tool_use".to_string()),
            Some("stop") => Some("end_turn".to_string()),
            Some("length") => Some("max_tokens".to_string()),
            other => other.map(|s| s.to_string()),
        };

        let usage = oai.usage.map(|u| Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        }).unwrap_or(Usage { input_tokens: 0, output_tokens: 0 });

        Ok(ChatResponse {
            id: oai.id,
            content,
            stop_reason,
            usage,
        })
    }
}
