use anyhow::Result;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};
use uuid::Uuid;

use bat_types::message::{Message, ToolCall, ToolResult};
use crate::llm::{AnthropicClient, AnthropicMessage, ChatRequest, ContentBlock};
use crate::tools::ToolRegistry;

const MAX_TOOL_ITERATIONS: usize = 10;

/// Result of a single conversation turn.
pub struct TurnResult {
    pub response_text: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
}

/// Run one conversation turn with real-time streaming for the text response.
///
/// The first LLM call streams text deltas through `text_tx`. Subsequent
/// calls (after tool execution) are non-streaming. Returns once the full
/// turn completes (stop_reason == "end_turn" or max iterations reached).
pub async fn run_turn_streaming(
    client: &AnthropicClient,
    registry: &ToolRegistry,
    model: &str,
    system_prompt: &str,
    history: &[Message],
    user_content: &str,
    _session_id: Uuid,
    text_tx: Sender<String>,
) -> Result<TurnResult> {
    let mut messages = history_to_anthropic(history);
    messages.push(AnthropicMessage {
        role: "user".to_string(),
        content: serde_json::Value::String(user_content.to_string()),
    });

    let tool_defs = registry.definitions();
    let mut all_tool_calls: Vec<ToolCall> = Vec::new();
    let mut all_tool_results: Vec<ToolResult> = Vec::new();
    let mut total_input = 0i64;
    let mut total_output = 0i64;

    // First iteration uses streaming to deliver text deltas in real time.
    // Subsequent iterations (post-tool-use) use non-streaming.
    let mut first_call = true;

    for iteration in 0..MAX_TOOL_ITERATIONS {
        info!("LLM call iteration {}", iteration + 1);

        let request = ChatRequest {
            model: model.to_string(),
            max_tokens: 8192,
            system: system_prompt.to_string(),
            messages: messages.clone(),
            tools: tool_defs.clone(),
            stream: false, // overridden below
        };

        let (response, response_text) = if first_call {
            first_call = false;
            let (resp, text) = client.chat_streaming(&request, text_tx.clone()).await?;
            (resp, text)
        } else {
            let resp = client.chat(&request).await?;
            let text = resp.text();
            // Send the post-tool-use text (usually empty) via channel too
            if !text.is_empty() {
                let _ = text_tx.send(text.clone()).await;
            }
            (resp, text)
        };

        total_input += response.usage.input_tokens;
        total_output += response.usage.output_tokens;

        if !response.wants_tool_use() {
            info!("Turn complete after {} iteration(s)", iteration + 1);
            return Ok(TurnResult {
                response_text,
                tool_calls: all_tool_calls,
                tool_results: all_tool_results,
                total_input_tokens: total_input,
                total_output_tokens: total_output,
            });
        }

        // Tool use — build assistant message and execute tools
        let assistant_content = build_assistant_content(&response.content);
        messages.push(AnthropicMessage {
            role: "assistant".to_string(),
            content: serde_json::Value::Array(assistant_content),
        });

        let tool_result_blocks =
            execute_tools(&response.content, registry, &mut all_tool_calls, &mut all_tool_results);
        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: serde_json::Value::Array(tool_result_blocks),
        });
    }

    error!("Max tool iterations ({}) reached", MAX_TOOL_ITERATIONS);
    Ok(TurnResult {
        response_text: "[Error: Maximum tool call iterations reached]".to_string(),
        tool_calls: all_tool_calls,
        tool_results: all_tool_results,
        total_input_tokens: total_input,
        total_output_tokens: total_output,
    })
}

/// Non-streaming turn — convenience wrapper used in tests.
#[allow(dead_code)]
pub async fn run_turn(
    client: &AnthropicClient,
    registry: &ToolRegistry,
    model: &str,
    system_prompt: &str,
    history: &[Message],
    user_content: &str,
    session_id: Uuid,
) -> Result<TurnResult> {
    let (tx, _rx) = tokio::sync::mpsc::channel(128);
    run_turn_streaming(
        client, registry, model, system_prompt, history, user_content, session_id, tx,
    )
    .await
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn build_assistant_content(content: &[ContentBlock]) -> Vec<serde_json::Value> {
    content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => serde_json::json!({
                "type": "text",
                "text": text,
            }),
            ContentBlock::ToolUse { id, name, input } => serde_json::json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input,
            }),
        })
        .collect()
}

fn execute_tools(
    content: &[ContentBlock],
    registry: &ToolRegistry,
    all_calls: &mut Vec<ToolCall>,
    all_results: &mut Vec<ToolResult>,
) -> Vec<serde_json::Value> {
    let mut blocks = Vec::new();
    for (id, name, input) in tool_uses(content) {
        info!("Executing tool: {}", name);
        let call = ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            input: input.clone(),
        };
        let result = registry.execute(&call);

        if result.is_error {
            warn!("Tool {} returned error: {}", name, result.content);
        }

        blocks.push(serde_json::json!({
            "type": "tool_result",
            "tool_use_id": id,
            "content": result.content,
            "is_error": result.is_error,
        }));

        all_calls.push(call);
        all_results.push(result);
    }
    blocks
}

fn tool_uses(content: &[ContentBlock]) -> Vec<(&str, &str, &serde_json::Value)> {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some((id.as_str(), name.as_str(), input)),
            _ => None,
        })
        .collect()
}

fn history_to_anthropic(history: &[Message]) -> Vec<AnthropicMessage> {
    history
        .iter()
        .filter(|m| m.role != bat_types::message::Role::System)
        .map(|m| {
            let role = match m.role {
                bat_types::message::Role::User => "user",
                bat_types::message::Role::Assistant => "assistant",
                bat_types::message::Role::System => unreachable!(),
            };
            AnthropicMessage {
                role: role.to_string(),
                content: serde_json::Value::String(m.content.clone()),
            }
        })
        .collect()
}
