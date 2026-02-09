//! AssemblyAI API implementation.
//!
//! Handles transcription requests to AssemblyAI's API using an upload→transcribe→poll pattern.
//! Unlike other providers that use a single synchronous request, AssemblyAI requires:
//! 1. Upload audio binary data to get an upload URL
//! 2. Submit a transcription request with the upload URL and options
//! 3. Poll for the completed transcript

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::TranscriptionConfig;

/// Response from the upload endpoint
#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

/// Request body for the transcription endpoint
#[derive(Debug, Serialize)]
struct TranscriptRequest {
    audio_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disfluencies: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_profanity: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_detection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    word_boost: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    boost_param: Option<String>,
}

/// Response from the transcription endpoint (both submit and poll)
#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    id: String,
    status: String,
    text: Option<String>,
    error: Option<String>,
}

/// Transcribes an audio file using AssemblyAI's API.
///
/// Uses a three-step process: upload audio, submit transcription request, poll for result.
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data = std::fs::read(audio_path).map_err(|e| {
        anyhow::anyhow!("Failed to read audio file: {e}")
    })?;

    let client = reqwest::Client::new();
    let base_url = config.model.endpoint();

    // Step 1: Upload audio
    tracing::debug!("Uploading audio to AssemblyAI...");
    let upload_response = match client
        .post(format!("{base_url}/upload"))
        .header("authorization", &config.api_key)
        .header("content-type", "application/octet-stream")
        .body(audio_data)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to AssemblyAI API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to AssemblyAI timed out. The API server is not responding.".to_string()
            } else {
                format!("AssemblyAI network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !upload_response.status().is_success() {
        let status = upload_response.status();
        let error_body = upload_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(format_error(status.as_u16(), &error_body)));
    }

    let upload: UploadResponse = upload_response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse AssemblyAI upload response: {e}"))?;

    tracing::debug!("Audio uploaded successfully");

    // Step 2: Submit transcription request
    let assemblyai_config = &config.providers.assemblyai;

    let mut request = TranscriptRequest {
        audio_url: upload.upload_url,
        speech_model: Some(config.model.api_model_name().to_string()),
        format_text: Some(assemblyai_config.format_text),
        disfluencies: Some(assemblyai_config.disfluencies),
        filter_profanity: Some(assemblyai_config.filter_profanity),
        language_detection: Some(assemblyai_config.language_detection),
        word_boost: None,
        boost_param: None,
    };

    // Add keywords as word_boost if any
    if !config.keywords.is_empty() {
        request.word_boost = Some(config.keywords.clone());
        request.boost_param = Some("high".to_string());
    }

    tracing::debug!("Submitting transcription request...");
    let submit_response = match client
        .post(format!("{base_url}/transcript"))
        .header("authorization", &config.api_key)
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return Err(anyhow::anyhow!("AssemblyAI submit error: {e}"));
        }
    };

    if !submit_response.status().is_success() {
        let status = submit_response.status();
        let error_body = submit_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(format_error(status.as_u16(), &error_body)));
    }

    let transcript: TranscriptResponse = submit_response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse AssemblyAI submit response: {e}"))?;

    let transcript_id = transcript.id;
    tracing::debug!("Transcription submitted, id: {transcript_id}");

    // Step 3: Poll for result
    let poll_url = format!("{base_url}/transcript/{transcript_id}");
    let mut poll_interval = tokio::time::interval(std::time::Duration::from_secs(1));

    loop {
        poll_interval.tick().await;

        let poll_response = client
            .get(&poll_url)
            .header("authorization", &config.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("AssemblyAI poll error: {e}"))?;

        if !poll_response.status().is_success() {
            let status = poll_response.status();
            let error_body = poll_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(format_error(status.as_u16(), &error_body)));
        }

        let result: TranscriptResponse = poll_response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse AssemblyAI poll response: {e}"))?;

        match result.status.as_str() {
            "completed" => {
                let text = result.text.unwrap_or_default();
                tracing::debug!("Transcription completed: {} chars", text.len());
                return Ok(text.trim().to_string());
            }
            "error" => {
                let error = result.error.unwrap_or_else(|| "Unknown transcription error".to_string());
                return Err(anyhow::anyhow!("AssemblyAI transcription failed: {error}"));
            }
            status => {
                tracing::debug!("Transcription status: {status}, polling...");
            }
        }
    }
}

/// Formats HTTP error codes into human-readable messages.
fn format_error(status: u16, error_body: &str) -> String {
    match status {
        401 => "AssemblyAI API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
        403 => "You don't have permission to use AssemblyAI's API. Check your API key and account status.".to_string(),
        429 => "Too many requests to AssemblyAI. You've hit the API rate limit. Please wait and try again.".to_string(),
        500 | 502 | 503 | 504 => "AssemblyAI API server is experiencing issues. Please try again later.".to_string(),
        _ => format!("AssemblyAI API error (status {status}): {error_body}"),
    }
}
