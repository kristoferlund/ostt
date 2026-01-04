//! Deepgram API implementation.
//!
//! Handles transcription requests to Deepgram's API using binary audio data.

use std::path::Path;
use serde::Deserialize;
use urlencoding;

use super::TranscriptionConfig;
use super::super::model::TranscriptionModel;

/// Deepgram response structure (kept for potential future use)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DeepgramResult {
    transcript: String,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: String,
}

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    results: DeepgramResults,
}

#[derive(Debug, Deserialize)]
struct DeepgramResults {
    channels: Vec<DeepgramChannel>,
}

/// Transcribes an audio file using Deepgram's API.
///
/// Sends raw binary audio data with Token authentication and model specified in query parameters.
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data = std::fs::read(audio_path).map_err(|e| {
        anyhow::anyhow!("Failed to read audio file: {e}")
    })?;

    let client = reqwest::Client::new();

    // Build the API URL with query parameters
    let mut url = format!(
        "{}?model={}",
        config.model.endpoint(),
        config.model.api_model_name()
    );

    // Add Deepgram feature flags from provider configuration
    let deepgram_config = &config.providers.deepgram;
    if deepgram_config.filler_words {
        url.push_str("&filler_words=true");
    }
    if deepgram_config.measurements {
        url.push_str("&measurements=true");
    }
    if deepgram_config.numerals {
        url.push_str("&numerals=true");
    }
    if deepgram_config.paragraphs {
        url.push_str("&paragraphs=true");
    }
    if deepgram_config.profanity_filter {
        url.push_str("&profanity_filter=true");
    }
    if deepgram_config.punctuate {
        url.push_str("&punctuate=true");
    }
    if deepgram_config.smart_format {
        url.push_str("&smart_format=true");
    }
    if deepgram_config.utterances {
        url.push_str("&utterances=true");
    }
    if deepgram_config.utt_split != 0.8 {
        url.push_str(&format!("&utt_split={}", deepgram_config.utt_split));
    }
    if deepgram_config.mip_opt_out {
        url.push_str("&mip_opt_out=true");
    }

    // Add keywords/keyterms if any (nova-3 uses keyterms, nova-2 uses keywords)
    if !config.keywords.is_empty() {
        let param_name = match config.model {
            TranscriptionModel::DeepgramNova3 => "keyterm",
            TranscriptionModel::DeepgramNova2 => "keywords",
            _ => "keywords", // fallback
        };
        for keyword in &config.keywords {
            url.push_str(&format!("&{}={}", param_name, urlencoding::encode(keyword)));
        }
    }

    let response = match client
        .post(&url)
        .header("Authorization", format!("Token {}", config.api_key))
        .header("Content-Type", "audio/mpeg")
        .body(audio_data)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to Deepgram API server. Check your internet connection.".to_string()
            } else if e.is_timeout() {
                "Request to Deepgram timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build Deepgram API request: {e}. This may be a configuration error.")
            } else {
                format!("Deepgram network error: {e}")
            };
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "Deepgram API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use Deepgram's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to Deepgram. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "Deepgram API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("Deepgram API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let deepgram_response: DeepgramResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Deepgram response: {e}"))?;

    // Extract transcript from the nested response structure
    let transcript = deepgram_response
        .results
        .channels
        .first()
        .and_then(|channel| channel.alternatives.first())
        .map(|alt| alt.transcript.clone())
        .ok_or_else(|| anyhow::anyhow!("No transcript found in Deepgram response"))?;

    Ok(transcript.trim().to_string())
}
