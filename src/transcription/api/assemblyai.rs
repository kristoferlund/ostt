//! AssemblyAI API implementation.
//!
//! Handles transcription requests to AssemblyAI's API using an upload→transcribe→poll pattern.
//! Unlike other providers that use a single synchronous request, AssemblyAI requires:
//! 1. Upload audio binary data to get an upload URL
//! 2. Submit a transcription request with the upload URL and options
//! 3. Poll for the completed transcript
//!
//! Performance optimizations based on AssemblyAI best practices:
//! - 3-second polling intervals (AssemblyAI recommended, not too aggressive)
//! - Exponential backoff retry for upload failures
//! - Connection pooling via shared client configuration

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::TranscriptionConfig;

/// Maximum number of poll attempts before timing out (5 minutes at 3-second intervals)
const MAX_POLL_ATTEMPTS: u32 = 100;

/// Polling interval in seconds (AssemblyAI recommends 3 seconds between polls)
const POLL_INTERVAL_SECS: u64 = 3;

/// Maximum retry attempts for transient upload errors
const MAX_UPLOAD_RETRIES: u32 = 3;

/// Initial retry delay for upload failures (doubles with each retry)
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// Response from the upload endpoint
#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

/// Options for language detection
#[derive(Debug, Serialize)]
struct LanguageDetectionOptionsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback_language: Option<String>,
}

/// Request body for the transcription endpoint
#[derive(Debug, Serialize)]
struct TranscriptRequest {
    audio_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disfluencies: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_profanity: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_detection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_detection_options: Option<LanguageDetectionOptionsRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    punctuate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyterms_prompt: Option<Vec<String>>,
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
/// Polls at 3-second intervals with a maximum timeout of 5 minutes.
/// Implements retry logic with exponential backoff for upload failures.
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data = std::fs::read(audio_path).map_err(|e| {
        anyhow::anyhow!("Failed to read audio file: {e}")
    })?;

    // Configure client with timeouts and connection pooling for better performance
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))  // Overall request timeout
        .connect_timeout(Duration::from_secs(10))  // Connection establishment timeout
        .pool_max_idle_per_host(10)  // Connection pooling for reuse
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {e}"))?;
    
    let base_url = config.model.endpoint();

    // Step 1: Upload audio with retry logic for transient failures
    let upload_url = upload_with_retry(&client, base_url, &config.api_key, &audio_data).await?;

    // Step 2: Submit transcription request
    let assemblyai_config = &config.providers.assemblyai;

    // Build language_detection_options if any values are set
    let language_detection_options = if assemblyai_config.language_detection_options.expected_languages.is_some()
        || assemblyai_config.language_detection_options.fallback_language.is_some()
    {
        Some(LanguageDetectionOptionsRequest {
            expected_languages: assemblyai_config.language_detection_options.expected_languages.clone(),
            fallback_language: assemblyai_config.language_detection_options.fallback_language.clone(),
        })
    } else {
        None
    };

    let mut request = TranscriptRequest {
        audio_url: upload_url,
        speech_models: Some(vec![config.model.api_model_name().to_string()]),
        format_text: Some(assemblyai_config.format_text),
        disfluencies: Some(assemblyai_config.disfluencies),
        filter_profanity: Some(assemblyai_config.filter_profanity),
        language_detection: Some(assemblyai_config.language_detection),
        language_detection_options,
        punctuate: Some(assemblyai_config.punctuate),
        keyterms_prompt: None,
    };

    // Add keywords as keyterms_prompt if any
    if !config.keywords.is_empty() {
        request.keyterms_prompt = Some(config.keywords.clone());
    }

    tracing::debug!("Submitting transcription request...");
    let submit_response = match client
        .post(format!("{base_url}/transcript"))
        .header("Authorization", &config.api_key)
        .header("Content-Type", "application/json")
        .json(&request)
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

    // Step 3: Poll for result with timeout
    // Poll at 3-second intervals (AssemblyAI recommended)
    let poll_url = format!("{base_url}/transcript/{transcript_id}");
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;

        if attempts > MAX_POLL_ATTEMPTS {
            return Err(anyhow::anyhow!(
                "AssemblyAI transcription timed out after {} seconds. The audio may be too long or the API is experiencing delays.",
                MAX_POLL_ATTEMPTS as u64 * POLL_INTERVAL_SECS
            ));
        }

        let poll_response = match client
            .get(&poll_url)
            .header("Authorization", &config.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = if e.is_connect() {
                    "Failed to connect to AssemblyAI API server while polling. Check your internet connection.".to_string()
                } else if e.is_timeout() {
                    "AssemblyAI poll request timed out. The API server is not responding.".to_string()
                } else {
                    format!("AssemblyAI poll network error: {e}")
                };
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        if !poll_response.status().is_success() {
            let status = poll_response.status();
            let error_body = poll_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(format_error(status.as_u16(), &error_body)));
        }

        let result: TranscriptResponse = poll_response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse AssemblyAI poll response: {e}"))?;

        tracing::debug!(
            "Poll attempt {}/{}: status={}, id={}",
            attempts, MAX_POLL_ATTEMPTS, result.status, result.id
        );

        match result.status.as_str() {
            "completed" => {
                let text = result.text.ok_or_else(|| {
                    anyhow::anyhow!("AssemblyAI returned completed status but no transcript text")
                })?;
                let trimmed = text.trim().to_string();
                tracing::debug!("Transcription completed: {} chars", trimmed.len());
                return Ok(trimmed);
            }
            "error" => {
                let error = result.error.unwrap_or_else(|| "Unknown transcription error".to_string());
                return Err(anyhow::anyhow!("AssemblyAI transcription failed: {error}"));
            }
            _ => {
                // Still processing (queued, processing, etc.)
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }
        }
    }
}

/// Uploads audio to AssemblyAI with exponential backoff retry logic.
/// 
/// AssemblyAI recommends implementing retry logic for transient upload errors
/// that may occur due to temporary server issues.
async fn upload_with_retry(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    audio_data: &[u8],
) -> anyhow::Result<String> {
    let mut retries = 0;
    let mut delay_ms = INITIAL_RETRY_DELAY_MS;

    loop {
        tracing::debug!("Uploading audio to AssemblyAI (attempt {} of {})...", retries + 1, MAX_UPLOAD_RETRIES + 1);
        
        match try_upload(client, base_url, api_key, audio_data).await {
            Ok(upload_url) => return Ok(upload_url),
            Err(e) => {
                retries += 1;
                if retries > MAX_UPLOAD_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Failed to upload audio after {} attempts: {}",
                        MAX_UPLOAD_RETRIES + 1,
                        e
                    ));
                }
                
                tracing::warn!("Upload attempt {} failed: {}. Retrying in {}ms...", retries, e, delay_ms);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2; // Exponential backoff
            }
        }
    }
}

/// Attempts a single upload request to AssemblyAI.
async fn try_upload(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    audio_data: &[u8],
) -> anyhow::Result<String> {
    let upload_response = client
        .post(format!("{base_url}/upload"))
        .header("Authorization", api_key)
        .header("Content-Type", "application/octet-stream")
        .body(audio_data.to_vec())
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                anyhow::anyhow!("Failed to connect to AssemblyAI API server. Check your internet connection.")
            } else if e.is_timeout() {
                anyhow::anyhow!("Request to AssemblyAI timed out. The API server is not responding.")
            } else {
                anyhow::anyhow!("AssemblyAI network error: {e}")
            }
        })?;

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
    Ok(upload.upload_url)
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
