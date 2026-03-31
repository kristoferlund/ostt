//! ElevenLabs Scribe API implementation.
//!
//! Handles transcription requests to ElevenLabs' speech-to-text API using
//! multipart form data. Supports the Scribe v2 and Scribe v1 models.

use std::path::Path;
use serde::Deserialize;

use super::TranscriptionConfig;

/// ElevenLabs speech-to-text response structure
#[derive(Debug, Deserialize)]
struct ElevenLabsResponse {
    /// The transcribed text
    text: String,
}

/// Transcribes an audio file using ElevenLabs' Scribe API.
///
/// Sends multipart form data with `xi-api-key` header authentication.
/// Keywords are passed as `keyterms` to improve transcription accuracy for
/// domain-specific terms.
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data = std::fs::read(audio_path).map_err(|e| {
        anyhow::anyhow!("Failed to read audio file: {e}")
    })?;

    let file_name = audio_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let file_part = reqwest::multipart::Part::bytes(audio_data)
        .file_name(file_name)
        .mime_str("audio/mpeg")
        .map_err(|e| anyhow::anyhow!("Failed to create file part for upload: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .text("model_id", config.model.api_model_name().to_string())
        .part("file", file_part);

    // Add optional language code from provider config
    let elevenlabs_config = &config.providers.elevenlabs;
    if let Some(ref lang) = elevenlabs_config.language_code {
        if !lang.is_empty() {
            form = form.text("language_code", lang.clone());
        }
    }

    // Add keyterms (ElevenLabs supports up to 1000 keyterms for boosting accuracy)
    // Each keyterm is passed as a separate form field
    for keyword in &config.keywords {
        form = form.text("keyterms", keyword.clone());
    }

    let client = reqwest::Client::new();
    let url = config.model.endpoint();

    tracing::debug!(
        "ElevenLabs API Call:\n  URL: {}\n  Method: POST\n  Model: {}\n  Keyterms: {:?}",
        url,
        config.model.api_model_name(),
        config.keywords,
    );

    let response = match client
        .post(url)
        .header("xi-api-key", &config.api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to ElevenLabs API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to ElevenLabs timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build ElevenLabs API request: {e}. This may be a configuration error.")
            } else {
                format!("ElevenLabs network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "ElevenLabs API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use ElevenLabs' API. Check your API key and account status.".to_string(),
            422 => format!("ElevenLabs API validation error: {error_body}"),
            429 => "Too many requests to ElevenLabs. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "ElevenLabs API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("ElevenLabs API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let elevenlabs_response: ElevenLabsResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse ElevenLabs response: {e}"))?;

    Ok(elevenlabs_response.text.trim().to_string())
}
