//! Telegram Bot API channel adapter.
//!
//! Runs long polling in a background task, routes messages to/from the gateway.

use anyhow::Result;
use serde::Deserialize;
use tracing::{info, warn, error, debug};
use tokio::sync::mpsc;

/// Configuration for the Telegram channel.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    /// User IDs allowed to interact with the bot.
    pub allow_from: Vec<i64>,
    /// Whether STT is enabled (transcribe voice messages).
    pub stt_enabled: bool,
    /// API key for Whisper (OpenAI).
    pub stt_api_key: String,
}

/// Inbound message from Telegram.
#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub chat_id: i64,
    pub user_id: i64,
    pub text: String,
    pub message_id: i64,
    /// Voice message transcription (if voice was sent).
    pub voice_text: Option<String>,
}

/// Outbound message to Telegram.
#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub chat_id: i64,
    pub text: String,
    pub reply_to: Option<i64>,
    /// If set, send as voice (OGG opus bytes).
    pub voice_data: Option<Vec<u8>>,
}

// ── Telegram API types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TgResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TgUpdate {
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Debug, Deserialize)]
struct TgMessage {
    message_id: i64,
    from: Option<TgUser>,
    chat: TgChat,
    text: Option<String>,
    voice: Option<TgVoice>,
}

#[derive(Debug, Deserialize)]
struct TgUser {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TgChat {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TgVoice {
    file_id: String,
    _duration: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TgFile {
    file_path: Option<String>,
}

/// The running Telegram adapter.
#[allow(dead_code)]
pub struct TelegramAdapter {
    config: TelegramConfig,
    client: reqwest::Client,
    /// Send outbound messages to Telegram.
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
}

impl TelegramAdapter {
    /// Start the Telegram adapter. Returns channels for inbound/outbound messages.
    pub fn start(
        config: TelegramConfig,
    ) -> (mpsc::UnboundedReceiver<InboundMessage>, mpsc::UnboundedSender<OutboundMessage>) {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<OutboundMessage>();

        let client = reqwest::Client::new();
        let bot_token = config.bot_token.clone();
        let allow_from = config.allow_from.clone();
        let stt_enabled = config.stt_enabled;
        let stt_api_key = config.stt_api_key.clone();

        // Long polling task
        let poll_client = client.clone();
        let poll_token = bot_token.clone();
        tokio::spawn(async move {
            let mut offset: i64 = 0;
            info!("Telegram adapter started (long polling)");

            loop {
                match get_updates(&poll_client, &poll_token, offset, 30).await {
                    Ok(updates) => {
                        for update in updates {
                            if let Some(update_id) = Some(update.update_id) {
                                offset = update_id + 1;
                            }
                            if let Some(msg) = update.message {
                                let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(0);

                                // Security: check allow_from
                                if !allow_from.is_empty() && !allow_from.contains(&user_id) {
                                    debug!("Telegram: ignoring message from unauthorized user {user_id}");
                                    continue;
                                }

                                let text = msg.text.unwrap_or_default();

                                // Handle voice messages
                                let voice_text = if let Some(ref voice) = msg.voice {
                                    if stt_enabled && !stt_api_key.is_empty() {
                                        match crate::stt::download_telegram_voice(&poll_token, &voice.file_id).await {
                                            Ok(audio_data) => {
                                                match crate::stt::transcribe(&audio_data, &stt_api_key).await {
                                                    Ok(transcription) => {
                                                        info!("Voice transcribed: \"{}\"", &transcription[..transcription.len().min(50)]);
                                                        Some(transcription)
                                                    }
                                                    Err(e) => {
                                                        warn!("Voice transcription failed: {e}");
                                                        None
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Voice download failed: {e}");
                                                None
                                            }
                                        }
                                    } else {
                                        debug!("Voice message received but STT disabled");
                                        None
                                    }
                                } else {
                                    None
                                };

                                if text.is_empty() && voice_text.is_none() {
                                    continue; // Skip non-text, non-voice messages
                                }

                                let inbound = InboundMessage {
                                    chat_id: msg.chat.id,
                                    user_id,
                                    text,
                                    message_id: msg.message_id,
                                    voice_text,
                                };
                                if inbound_tx.send(inbound).is_err() {
                                    error!("Telegram inbound channel closed");
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Telegram polling error: {e}");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        // Outbound send task
        let send_client = client.clone();
        let send_token = bot_token.clone();
        tokio::spawn(async move {
            info!("Telegram outbound send task started");
            while let Some(msg) = outbound_rx.recv().await {
                info!("Telegram outbound: received message for chat_id={}, has_voice={}, text_len={}", msg.chat_id, msg.voice_data.is_some(), msg.text.len());
                if let Some(voice_data) = msg.voice_data {
                    info!("Telegram outbound: sending voice ({} bytes)...", voice_data.len());
                    match send_voice(&send_client, &send_token, msg.chat_id, &voice_data, msg.reply_to).await {
                        Ok(_) => info!("Telegram outbound: voice sent successfully"),
                        Err(e) => {
                            error!("Telegram send voice error: {e}");
                            // Fallback: try sending text only
                            info!("Telegram outbound: falling back to text-only...");
                            if let Err(e2) = send_message(&send_client, &send_token, msg.chat_id, &msg.text, msg.reply_to).await {
                                error!("Telegram send text fallback also failed: {e2}");
                            }
                        }
                    }
                } else {
                    info!("Telegram outbound: sending text...");
                    if let Err(e) = send_message(&send_client, &send_token, msg.chat_id, &msg.text, msg.reply_to).await {
                        error!("Telegram send error: {e}");
                    } else {
                        info!("Telegram outbound: text sent successfully");
                    }
                }
            }
            warn!("Telegram outbound send task exited — channel closed");
        });

        (inbound_rx, outbound_tx)
    }
}

// ── API calls ───────────────────────────────────────────────────────────

async fn get_updates(
    client: &reqwest::Client,
    token: &str,
    offset: i64,
    timeout: u64,
) -> Result<Vec<TgUpdate>> {
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");
    let resp: TgResponse<Vec<TgUpdate>> = client
        .get(&url)
        .query(&[
            ("offset", offset.to_string()),
            ("timeout", timeout.to_string()),
            ("allowed_updates", r#"["message"]"#.to_string()),
        ])
        .timeout(std::time::Duration::from_secs(timeout + 10))
        .send()
        .await?
        .json()
        .await?;

    if !resp.ok {
        return Err(anyhow::anyhow!("Telegram API error: {}", resp.description.unwrap_or_default()));
    }
    Ok(resp.result.unwrap_or_default())
}

async fn send_message(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    text: &str,
    reply_to: Option<i64>,
) -> Result<()> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");

    // Telegram has a 4096 char limit. Split if needed.
    let chunks = split_message(text, 4096);
    for chunk in chunks {
        let mut params = serde_json::json!({
            "chat_id": chat_id,
            "text": chunk,
            "parse_mode": "Markdown",
        });
        if let Some(reply_id) = reply_to {
            params["reply_to_message_id"] = serde_json::json!(reply_id);
        }

        let resp: TgResponse<serde_json::Value> = client
            .post(&url)
            .json(&params)
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            // Retry without Markdown if parse fails
            let mut params = serde_json::json!({
                "chat_id": chat_id,
                "text": chunk,
            });
            if let Some(reply_id) = reply_to {
                params["reply_to_message_id"] = serde_json::json!(reply_id);
            }
            client.post(&url).json(&params).send().await?;
        }
    }
    Ok(())
}

async fn send_voice(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    voice_data: &[u8],
    reply_to: Option<i64>,
) -> Result<()> {
    let url = format!("https://api.telegram.org/bot{token}/sendVoice");

    let part = reqwest::multipart::Part::bytes(voice_data.to_vec())
        .file_name("voice.ogg")
        .mime_str("audio/ogg")?;

    let mut form = reqwest::multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .part("voice", part);

    if let Some(reply_id) = reply_to {
        form = form.text("reply_to_message_id", reply_id.to_string());
    }

    let resp = client.post(&url).multipart(form).send().await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !body.contains("\"ok\":true") && !body.contains("\"ok\": true") {
        error!("Telegram sendVoice failed: HTTP {status}, body: {body}");
        return Err(anyhow::anyhow!("sendVoice failed: {body}"));
    }
    info!("Telegram sendVoice response: HTTP {status}, body: {}", &body[..body.len().min(200)]);
    Ok(())
}

/// Send a chat action (e.g. "typing") to a Telegram chat.
/// Failures are logged as warnings and never propagated.
pub async fn send_chat_action(client: &reqwest::Client, token: &str, chat_id: i64, action: &str) {
    let url = format!("https://api.telegram.org/bot{token}/sendChatAction");
    let params = serde_json::json!({
        "chat_id": chat_id,
        "action": action,
    });
    match client.post(&url).json(&params).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("sendChatAction failed: HTTP {}", resp.status());
            }
        }
        Err(e) => {
            warn!("sendChatAction error: {e}");
        }
    }
}

/// Spawn a background task that sends "typing" every 4 seconds until the
/// returned [`tokio::sync::oneshot::Sender`] is dropped or signalled.
pub fn spawn_typing_loop(
    client: reqwest::Client,
    token: String,
    chat_id: i64,
) -> tokio::sync::oneshot::Sender<()> {
    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        // Send immediately, then every 4 seconds
        send_chat_action(&client, &token, chat_id, "typing").await;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(4)) => {
                    send_chat_action(&client, &token, chat_id, "typing").await;
                }
                _ = &mut cancel_rx => {
                    break;
                }
            }
        }
    });
    cancel_tx
}

fn split_message(text: &str, max_len: usize) -> Vec<&str> {
    if text.len() <= max_len {
        return vec![text];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = (start + max_len).min(text.len());
        // Try to split at a newline
        let split_at = if end < text.len() {
            text[start..end].rfind('\n').map(|i| start + i + 1).unwrap_or(end)
        } else {
            end
        };
        chunks.push(&text[start..split_at]);
        start = split_at;
    }
    chunks
}
