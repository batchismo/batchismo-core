//! Speech-to-Text — OpenAI Whisper API.

use anyhow::{Context, Result};
use tracing::info;

/// Transcribe audio bytes using OpenAI Whisper.
pub async fn transcribe(audio_data: &[u8], api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    info!("STT: Whisper transcribing {} bytes", audio_data.len());

    let part = reqwest::multipart::Part::bytes(audio_data.to_vec())
        .file_name("audio.ogg")
        .mime_str("audio/ogg")?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .text("response_format", "text")
        .part("file", part);

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .context("Failed to call Whisper API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Whisper API error ({status}): {body}");
    }

    let text = resp.text().await?.trim().to_string();
    info!("STT: Whisper transcribed: \"{}\"", &text[..text.len().min(80)]);
    Ok(text)
}

/// Download a Telegram voice file and return the bytes.
pub async fn download_telegram_voice(bot_token: &str, file_id: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();

    // Step 1: getFile to get file_path
    let url = format!("https://api.telegram.org/bot{bot_token}/getFile");
    let resp: serde_json::Value = client
        .get(&url)
        .query(&[("file_id", file_id)])
        .send()
        .await?
        .json()
        .await?;

    let file_path = resp["result"]["file_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No file_path in getFile response"))?;

    // Step 2: Download the file
    let download_url = format!("https://api.telegram.org/file/bot{bot_token}/{file_path}");
    let data = client.get(&download_url).send().await?.bytes().await?.to_vec();
    info!("STT: Downloaded Telegram voice file ({} bytes)", data.len());
    Ok(data)
}
