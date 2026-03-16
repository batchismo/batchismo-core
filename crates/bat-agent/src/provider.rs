//! Unified LLM provider — routes requests to the correct backend client.

use anyhow::Result;
use tokio::sync::mpsc::Sender;

use crate::llm::{AnthropicClient, ChatRequest, ChatResponse};
use crate::openai_client::OpenAICompatibleClient;

/// A unified LLM client that wraps either Anthropic or OpenAI-compatible backends.
pub enum LlmClient {
    Anthropic(AnthropicClient),
    OpenAICompatible(OpenAICompatibleClient),
}

impl LlmClient {
    /// Non-streaming chat request.
    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        match self {
            LlmClient::Anthropic(c) => c.chat(request).await,
            LlmClient::OpenAICompatible(c) => c.chat(request).await,
        }
    }

    /// Streaming chat request.
    pub async fn chat_streaming(
        &self,
        request: &ChatRequest,
        text_tx: Sender<String>,
    ) -> Result<(ChatResponse, String)> {
        match self {
            LlmClient::Anthropic(c) => c.chat_streaming(request, text_tx).await,
            LlmClient::OpenAICompatible(c) => c.chat_streaming(request, text_tx).await,
        }
    }
}
