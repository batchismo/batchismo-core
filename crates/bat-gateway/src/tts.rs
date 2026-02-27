//! Text-to-Speech — supports OpenAI TTS and ElevenLabs.

use anyhow::{Context, Result};
use tracing::{info, warn};

use bat_types::config::VoiceConfig;

/// Generated speech audio.
pub struct SpeechAudio {
    /// Raw audio bytes (OGG opus for Telegram, MP3 otherwise).
    pub data: Vec<u8>,
    /// MIME type of the audio.
    pub mime_type: String,
}

/// Synthesize speech from text using the configured provider.
/// Applies a 30-second timeout to prevent hanging TTS calls from blocking replies.
pub async fn synthesize(text: &str, config: &VoiceConfig, api_key: &str) -> Result<SpeechAudio> {
    let timeout = std::time::Duration::from_secs(30);
    let result = match config.tts_provider.as_str() {
        "elevenlabs" => {
            tokio::time::timeout(timeout, synthesize_elevenlabs(text, config, api_key)).await
        }
        _ => {
            tokio::time::timeout(timeout, synthesize_openai(text, config, api_key)).await
        }
    };
    result.map_err(|_| anyhow::anyhow!("TTS timed out after 30 seconds"))?
}

/// OpenAI TTS API.
async fn synthesize_openai(text: &str, config: &VoiceConfig, api_key: &str) -> Result<SpeechAudio> {
    let client = reqwest::Client::new();
    let model = &config.openai_tts_model;
    let voice = &config.openai_voice;

    info!("TTS: OpenAI {model}/{voice}, {} chars", text.len());

    let resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "model": model,
            "input": text,
            "voice": voice,
            "response_format": "opus",
        }))
        .send()
        .await
        .context("Failed to call OpenAI TTS API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI TTS error ({status}): {body}");
    }

    let data = resp.bytes().await?.to_vec();
    info!("TTS: OpenAI returned {} bytes", data.len());

    Ok(SpeechAudio {
        data,
        mime_type: "audio/ogg".to_string(),
    })
}

/// ElevenLabs TTS API. The api_key parameter is the ElevenLabs key.
async fn synthesize_elevenlabs(text: &str, config: &VoiceConfig, api_key: &str) -> Result<SpeechAudio> {
    let voice_id = config.elevenlabs_voice_id.as_deref()
        .ok_or_else(|| anyhow::anyhow!("ElevenLabs voice ID not configured"))?;

    let client = reqwest::Client::new();

    info!("TTS: ElevenLabs voice={voice_id}, {} chars", text.len());

    let resp = client
        .post(format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}"))
        .header("xi-api-key", api_key)
        .header("Accept", "audio/mpeg")
        .json(&serde_json::json!({
            "text": text,
            "model_id": "eleven_turbo_v2_5",
            "output_format": "mp3_44100_128",
        }))
        .send()
        .await
        .context("Failed to call ElevenLabs TTS API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("ElevenLabs TTS error ({status}): {body}");
    }

    let mp3_data = resp.bytes().await?.to_vec();
    info!("TTS: ElevenLabs returned {} bytes (MP3)", mp3_data.len());

    // For Telegram voice, we need OGG opus. Convert MP3→OGG if ffmpeg available,
    // otherwise fall back to sending as audio file (not voice note).
    match convert_mp3_to_ogg(&mp3_data).await {
        Ok(ogg_data) => Ok(SpeechAudio {
            data: ogg_data,
            mime_type: "audio/ogg".to_string(),
        }),
        Err(e) => {
            warn!("MP3→OGG conversion failed ({e}), sending as MP3");
            Ok(SpeechAudio {
                data: mp3_data,
                mime_type: "audio/mpeg".to_string(),
            })
        }
    }
}

/// Convert MP3 bytes to OGG opus using ffmpeg (if available).
async fn convert_mp3_to_ogg(mp3_data: &[u8]) -> Result<Vec<u8>> {
    use tokio::process::Command;
    use tokio::io::AsyncWriteExt;

    let mut child = Command::new("ffmpeg")
        .args(["-i", "pipe:0", "-c:a", "libopus", "-f", "ogg", "pipe:1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("ffmpeg not found — install ffmpeg for ElevenLabs voice on Telegram")?;

    let mut stdin = child.stdin.take().unwrap();
    let data = mp3_data.to_vec();
    tokio::spawn(async move {
        let _ = stdin.write_all(&data).await;
        drop(stdin);
    });

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!("ffmpeg exited with code {:?}", output.status.code());
    }
    Ok(output.stdout)
}
