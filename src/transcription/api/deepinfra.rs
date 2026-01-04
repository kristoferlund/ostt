//! DeepInfra API implementation.
//!
//! Handles transcription requests to DeepInfra's inference API using multipart form data.

use std::path::Path;

use super::TranscriptionConfig;
use super::shared::WhisperApiResponse;

/// Transcribes an audio file using DeepInfra's Whisper API.
///
/// Uses multipart form data with bearer token authentication.
/// DeepInfra hosts OpenAI's Whisper model and compatible models.
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

    let mut form = reqwest::multipart::Form::new().part("audio", file_part);

    // Debug log: Log the API call details (without the audio data)
    let mut debug_params = vec![];

    // Build the URL with model name in the path
    let endpoint = format!(
        "{}/{}",
        config.model.endpoint(),
        config.model.api_model_name()
    );

    // Add keywords as prompt for better transcription context (similar to OpenAI)
    if !config.keywords.is_empty() {
        let prompt = config.keywords.join(", ");
        form = form.text("prompt", prompt.clone());
        debug_params.push(format!("prompt={prompt}"));
        tracing::debug!("Keywords used as prompt for DeepInfra model: {:?}", config.keywords);
    }

    tracing::debug!(
        "DeepInfra API Call:\n  URL: {}\n  Method: POST\n  Headers:\n    Authorization: Bearer <redacted>\n    Content-Type: multipart/form-data\n  Body parameters: {}",
        endpoint,
        if debug_params.is_empty() {
            "none".to_string()
        } else {
            debug_params.join("\n    ")
        }
    );

    let response = match client
        .post(&endpoint)
        .bearer_auth(&config.api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to DeepInfra API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to DeepInfra timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build DeepInfra API request: {e}. This may be a configuration error.")
            } else {
                format!("DeepInfra network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "DeepInfra API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use DeepInfra's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to DeepInfra. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "DeepInfra API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("DeepInfra API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let deepinfra_response: WhisperApiResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse DeepInfra response: {e}"))?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "DeepInfra API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        deepinfra_response.text.len(),
        deepinfra_response
    );

    Ok(deepinfra_response.text)
}
