//! Mistral Voxtral API implementation.
//!
//! Handles transcription requests to Mistral's Voxtral transcription API using
//! multipart form data. Supports the Voxtral Mini Transcribe model.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;

/// Mistral API response wrapper
#[derive(Debug, Deserialize)]
struct MistralResponse {
    text: String,
}

/// Transcribes an audio file using Mistral's Voxtral API.
///
/// Uses multipart form data with bearer token authentication.
/// Keywords are passed as the `context_bias` parameter to improve transcription
/// accuracy for domain-specific terms.
pub async fn transcribe(config: &TranscriptionConfig, audio_path: &Path) -> anyhow::Result<String> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| anyhow::anyhow!("Failed to read audio file: {e}"))?;

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
        .part("file", file_part)
        .text("model", config.model.api_model_name().to_string());

    // Debug log: Log the API call details (without the audio data)
    let debug_params = [format!("model={}", config.model.api_model_name())];

    // Add keywords as context_bias for better transcription accuracy
    // Mistral supports up to 100 words/phrases for context biasing
    if !config.keywords.is_empty() {
        for keyword in &config.keywords {
            form = form.text("context_bias", keyword.clone());
        }
        tracing::debug!("Keywords used as context_bias for Mistral model: {:?}", config.keywords);
    }

    // Add optional language parameter from provider config
    let mistral_config = &config.providers.mistral;
    if let Some(ref lang) = mistral_config.language {
        if !lang.is_empty() {
            form = form.text("language", lang.clone());
            tracing::debug!("Language set for Mistral model: {}", lang);
        }
    }

    let endpoint = config.model.endpoint();

    let client = reqwest::Client::new();

    tracing::debug!(
        "Mistral API Call:\n  URL: {}\n  Method: POST\n  Headers:\n    Authorization: Bearer <redacted>\n    Content-Type: multipart/form-data\n  Body parameters: {}",
        endpoint,
        debug_params.join("\n    ")
    );

    let response = match client
        .post(endpoint)
        .bearer_auth(&config.api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to Mistral API server. Check your internet connection."
                    .to_string()
            } else if e.is_timeout() {
                "Request to Mistral timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build Mistral API request: {e}. This may be a configuration error.")
            } else {
                format!("Mistral network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "Mistral API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use Mistral's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to Mistral. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "Mistral API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("Mistral API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let mistral_response: MistralResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Mistral response: {e}"))?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "Mistral API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        mistral_response.text.len(),
        mistral_response
    );

    Ok(mistral_response.text.trim().to_string())
}
