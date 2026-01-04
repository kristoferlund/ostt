//! OpenAI Whisper API implementation.
//!
//! Handles transcription requests to OpenAI's Whisper API using multipart form data.

use std::path::Path;
use serde::Deserialize;

use super::TranscriptionConfig;

/// OpenAI API response wrapper
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    text: String,
}

/// Transcribes an audio file using OpenAI's Whisper API.
///
/// Uses multipart form data with bearer token authentication.
/// 
/// Keywords are passed as the `prompt` parameter to guide transcription context.
/// OpenAI's Whisper API uses the prompt to improve accuracy for domain-specific terms.
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
    // Note: gpt-4o-transcribe doesn't support prompt parameter, only whisper-1 and gpt-4o-mini-transcribe do
    if !config.keywords.is_empty() {
        let should_use_prompt = match config.model.api_model_name() {
            "gpt-4o-transcribe" => false, // gpt-4o-transcribe doesn't support prompt
            _ => true, // whisper-1 and gpt-4o-mini-transcribe support it
        };
        
        if should_use_prompt {
            let prompt = config.keywords.join(", ");
            form = form.text("prompt", prompt.clone());
            debug_params.push(format!("prompt={prompt}"));
            tracing::debug!("Keywords used as prompt for OpenAI model: {:?}", config.keywords);
        } else {
            tracing::debug!("Keywords defined but {} does not support prompt parameter. Keywords: {:?}", 
                config.model.api_model_name(), config.keywords);
        }
    }

    let endpoint = config.model.endpoint();
    let url = format!("{endpoint}?response_format=json");
    debug_params.push("response_format=json".to_string());

    tracing::debug!(
        "OpenAI API Call:\n  URL: {}\n  Method: POST\n  Headers:\n    Authorization: Bearer <redacted>\n    Content-Type: multipart/form-data\n  Body parameters: {}",
        url,
        debug_params.join("\n    ")
    );

    let response = match client
        .post(&url)
        .bearer_auth(&config.api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to OpenAI API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to OpenAI timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build OpenAI API request: {e}. This may be a configuration error.")
            } else {
                format!("OpenAI network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "OpenAI API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use OpenAI's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to OpenAI. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "OpenAI API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("OpenAI API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let transcription: OpenAiResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse OpenAI response: {e}"))?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "OpenAI API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        transcription.text.len(),
        transcription
    );

    Ok(transcription.text.trim().to_string())
}
