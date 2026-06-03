//! Mistral Voxtral API implementation.
//!
//! Handles transcription requests to Mistral's Voxtral transcription API using
//! multipart form data. Supports the Voxtral Mini Transcribe model.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "mistral",
        model_id: "voxtral-mini-latest",
        endpoint: "https://api.mistral.ai/v1/audio/transcriptions",
        display_name: "Voxtral Mini Transcribe (fast, efficient, 13 languages)",
        description: "Mistral's Voxtral Mini transcription model for fast, efficient multilingual speech-to-text via the Mistral API.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "mistral",
        model_id: "voxtral-mini-2602",
        endpoint: "https://api.mistral.ai/v1/audio/transcriptions",
        display_name: "Voxtral Mini 2602 (newer version, improved accuracy)",
        description: "Pinned Voxtral Mini 2602 transcription model for stable Mistral speech-to-text behavior.",
        languages: &["Multilingual"],
    },
];

const OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "language",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "diarize",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "context_bias",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "timestamp_granularities",
        kind: ModelOptionKind::StringList,
    },
];

pub(super) fn option_schema(_model_id: &str) -> Option<ModelOptionSchema> {
    Some(ModelOptionSchema::new(OPTIONS))
}

pub(super) fn validate_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    super::validate_number_range(full_model_id, options, "temperature", 0.0..=1.0)?;

    if let Some(value) = options.get("timestamp_granularities") {
        super::validate_string_list_values(
            full_model_id,
            "timestamp_granularities",
            value,
            &["segment", "word"],
        )?;
    }

    if options.contains_key("language") && options.contains_key("timestamp_granularities") {
        anyhow::bail!(
            "Invalid params for '{}'. Mistral timestamp_granularities is not compatible with language.",
            full_model_id
        );
    }

    Ok(())
}

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
pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
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
        .text("model", config.model_id.clone());

    // Debug log: Log the API call details (without the audio data)
    let mut debug_params = vec![format!("model={}", config.model_id)];

    if let Some(language) = config.option_string("language") {
        form = form.text("language", language.to_string());
        debug_params.push(format!("language={language}"));
    }

    if let Some(diarize) = config.option_bool("diarize") {
        form = form.text("diarize", diarize.to_string());
        debug_params.push(format!("diarize={diarize}"));
    }

    if let Some(temperature) = config.option_number("temperature") {
        form = form.text("temperature", temperature.to_string());
        debug_params.push(format!("temperature={temperature}"));
    }

    if let Some(granularities) = config.option_string_list("timestamp_granularities") {
        for granularity in granularities {
            form = form.text("timestamp_granularities", granularity.clone());
            debug_params.push(format!("timestamp_granularities={granularity}"));
        }
    }

    if let Some(context_bias) = config.option_string_list("context_bias") {
        for value in context_bias {
            form = form.text("context_bias", value.clone());
            debug_params.push(format!("context_bias={value}"));
        }
    } else if !config.keywords.is_empty() {
        for keyword in &config.keywords {
            form = form.text("context_bias", keyword.clone());
        }
        tracing::debug!(
            "Keywords used as context_bias for Mistral model: {:?}",
            config.keywords
        );
    }

    let endpoint = &config.endpoint;

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
                format!(
                    "Failed to build Mistral API request: {e}. This may be a configuration error."
                )
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
