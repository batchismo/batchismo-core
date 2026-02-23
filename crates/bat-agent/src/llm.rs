use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value,
}

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    #[allow(dead_code)]
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

// ─── SSE streaming types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: SseMessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: SseContentBlockStart,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: SseDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { #[allow(dead_code)] index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: SseMessageDelta,
        usage: SseDeltaUsage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct SseMessageStart {
    id: String,
    usage: SseStartUsage,
}

#[derive(Debug, Deserialize)]
struct SseStartUsage {
    input_tokens: i64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseContentBlockStart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
struct SseMessageDelta {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseDeltaUsage {
    output_tokens: i64,
}

// Accumulator for building ContentBlocks from stream events
enum BlockAccum {
    Text { text: String },
    ToolUse { id: String, name: String, input_json: String },
}

// ─── Client ───────────────────────────────────────────────────────────────────

pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Non-streaming chat request.
    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let mut req = request.clone();
        req.stream = false;

        let url = format!("{}/messages", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error ({}): {}", status, body);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic API response")?;

        Ok(chat_response)
    }

    /// Streaming chat request. Text chunks are sent via `text_tx`.
    /// Returns the reconstructed ChatResponse plus the full accumulated text.
    pub async fn chat_streaming(
        &self,
        request: &ChatRequest,
        text_tx: Sender<String>,
    ) -> Result<(ChatResponse, String)> {
        let mut req = request.clone();
        req.stream = true;

        let url = format!("{}/messages", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req)
            .send()
            .await
            .context("Failed to send streaming request to Anthropic API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error ({}): {}", status, body);
        }

        // Parse SSE stream
        let mut stream = response.bytes_stream();
        let mut sse_buffer = String::new();

        // Accumulate state
        let mut message_id = String::new();
        let mut input_tokens = 0i64;
        let mut output_tokens = 0i64;
        let mut stop_reason: Option<String> = None;
        let mut blocks: Vec<BlockAccum> = Vec::new();
        let mut full_text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading streaming response")?;
            sse_buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE messages (separated by \n\n)
            loop {
                if let Some(pos) = sse_buffer.find("\n\n") {
                    let message = sse_buffer[..pos].to_string();
                    sse_buffer = sse_buffer[pos + 2..].to_string();

                    if let Some(event) = parse_sse_message(&message) {
                        match event {
                            SseEvent::MessageStart { message: msg_start } => {
                                message_id = msg_start.id;
                                input_tokens = msg_start.usage.input_tokens;
                            }
                            SseEvent::ContentBlockStart {
                                index,
                                content_block,
                            } => {
                                // Ensure blocks vec is large enough
                                while blocks.len() <= index {
                                    blocks.push(BlockAccum::Text { text: String::new() });
                                }
                                blocks[index] = match content_block {
                                    SseContentBlockStart::Text { text } => {
                                        BlockAccum::Text { text }
                                    }
                                    SseContentBlockStart::ToolUse { id, name } => {
                                        BlockAccum::ToolUse {
                                            id,
                                            name,
                                            input_json: String::new(),
                                        }
                                    }
                                };
                            }
                            SseEvent::ContentBlockDelta { index, delta } => {
                                if let Some(block) = blocks.get_mut(index) {
                                    match (block, delta) {
                                        (
                                            BlockAccum::Text { text },
                                            SseDelta::TextDelta { text: chunk_text },
                                        ) => {
                                            text.push_str(&chunk_text);
                                            full_text.push_str(&chunk_text);
                                            // Send text delta to caller (best-effort)
                                            let _ = text_tx.try_send(chunk_text);
                                        }
                                        (
                                            BlockAccum::ToolUse { input_json, .. },
                                            SseDelta::InputJsonDelta { partial_json },
                                        ) => {
                                            input_json.push_str(&partial_json);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            SseEvent::MessageDelta { delta, usage } => {
                                stop_reason = delta.stop_reason;
                                output_tokens = usage.output_tokens;
                            }
                            SseEvent::MessageStop => break,
                            _ => {}
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // Reconstruct ChatResponse from accumulated state
        let content: Vec<ContentBlock> = blocks
            .into_iter()
            .filter_map(|b| match b {
                BlockAccum::Text { text } if !text.is_empty() => {
                    Some(ContentBlock::Text { text })
                }
                BlockAccum::ToolUse { id, name, input_json } => {
                    let input = serde_json::from_str(&input_json)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    Some(ContentBlock::ToolUse { id, name, input })
                }
                _ => None,
            })
            .collect();

        let chat_response = ChatResponse {
            id: message_id,
            content,
            stop_reason,
            usage: Usage {
                input_tokens,
                output_tokens,
            },
        };

        Ok((chat_response, full_text))
    }
}

/// Parse one SSE message block (may contain "event: ..." and "data: ..." lines).
fn parse_sse_message(message: &str) -> Option<SseEvent> {
    let mut data_line: Option<&str> = None;

    for line in message.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            data_line = Some(data);
        }
        // We ignore "event:" lines — type is in the JSON "type" field
    }

    let data = data_line?;
    if data == "[DONE]" {
        return Some(SseEvent::MessageStop);
    }

    serde_json::from_str(data).ok()
}

// ─── ChatResponse helpers ─────────────────────────────────────────────────────

impl ChatResponse {
    /// Extract all text content from the response.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool use blocks.
    #[allow(dead_code)]
    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input))
                }
                _ => None,
            })
            .collect()
    }

    /// Check if the model wants to use tools.
    pub fn wants_tool_use(&self) -> bool {
        self.stop_reason.as_deref() == Some("tool_use")
    }
}
