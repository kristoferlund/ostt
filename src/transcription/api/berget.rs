//! Berget API implementation.
//!
//! Handles transcription requests to Berget's OpenAI-compatible Whisper API using multipart form data.

use std::path::Path;
use serde::Deserialize;

use super::TranscriptionConfig;

/// Berget API response wrapper
#[derive(Debug, Deserialize)]
struct BergetResponse {
    text: String,
}

/// Berget API error response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BergetErrorResponse {
    code: String,
    error: String,
    #[serde(default)]
    details: Option<String>,
}

/// Transcribes an audio file using Berget's Whisper API.
///
/// Uses multipart form data with bearer token authentication.
/// Berget provides an OpenAI-compatible API endpoint.
///
/// Keywords are passed as the `prompt` parameter to guide transcription context.
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data = std::fs::read(audio_path).map_err(|e| {
        anyhow::anyhow!("Failed to read audio file: {e}")
    })?;

    let client = reqwest::Client::new();

    let file_name = audio_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let file_part = reqwest::multipart::Part::bytes(audio_data)
        .file_name(file_name.clone())
        .mime_str("audio/mpeg")
        .map_err(|e| anyhow::anyhow!("Failed to create file part for upload: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", config.model.api_model_name().to_string());

    // Debug log: Log the API call details (without the audio data)
    let mut debug_params = vec![
        format!("model={}", config.model.api_model_name()),
    ];

    // Add keywords as prompt for better transcription context
    if !config.keywords.is_empty() {
        let prompt = config.keywords.join(", ");
        form = form.text("prompt", prompt.clone());
        debug_params.push(format!("prompt={prompt}"));
        tracing::debug!("Keywords used as prompt for Berget model: {:?}", config.keywords);
    }

    let endpoint = config.model.endpoint();

    tracing::debug!(
        "Berget API Call:\n  URL: {}\n  Method: POST\n  Headers:\n    Authorization: Bearer <redacted>\n    Content-Type: multipart/form-data\n  Body parameters: {}",
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
                "Failed to connect to Berget API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to Berget timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build Berget API request: {e}. This may be a configuration error.")
            } else {
                format!("Berget network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        
        // Parse the JSON error response - all errors follow the same structure
        let error_message = response
            .json::<BergetErrorResponse>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("HTTP {status}"));

        return Err(anyhow::anyhow!("Berget API error: {error_message}"));
    }

    let berget_response: BergetResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Berget response: {e}"))?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "Berget API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        berget_response.text.len(),
        berget_response
    );

    Ok(berget_response.text.trim().to_string())
}
