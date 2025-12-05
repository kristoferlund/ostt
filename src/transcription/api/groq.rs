//! Groq API implementation.
//!
//! Handles transcription requests to Groq's OpenAI-compatible Whisper API using multipart form data.

use std::path::Path;
use serde::Deserialize;

use super::TranscriptionConfig;

/// Groq API response wrapper
#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
}

/// Transcribes an audio file using Groq's Whisper API.
///
/// Uses multipart form data with bearer token authentication.
/// Groq provides an OpenAI-compatible API endpoint.
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
        tracing::debug!("Keywords used as prompt for Groq model: {:?}", config.keywords);
    }

    let endpoint = config.model.endpoint();

    tracing::debug!(
        "Groq API Call:\n  URL: {}\n  Method: POST\n  Headers:\n    Authorization: Bearer <redacted>\n    Content-Type: multipart/form-data\n  Body parameters: {}",
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
                "Failed to connect to Groq API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to Groq timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build Groq API request: {e}. This may be a configuration error.")
            } else {
                format!("Groq network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "Groq API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use Groq's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to Groq. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "Groq API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("Groq API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let groq_response: GroqResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Groq response: {e}"))?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "Groq API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        groq_response.text.len(),
        groq_response
    );

    Ok(groq_response.text)
}
