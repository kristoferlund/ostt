//! Groq API implementation.
//!
//! Handles transcription requests to Groq's OpenAI-compatible Whisper API using multipart form data.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "groq",
        model_id: "whisper-large-v3",
        endpoint: "https://api.groq.com/openai/v1/audio/transcriptions",
        display_name: "Whisper Large V3 (high accuracy)",
        description: "Groq-hosted Whisper Large V3 for error-sensitive multilingual transcription and translation. Groq documents this option as the higher-accuracy choice among its Whisper models.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "groq",
        model_id: "whisper-large-v3-turbo",
        endpoint: "https://api.groq.com/openai/v1/audio/transcriptions",
        display_name: "Whisper Large V3 Turbo (fastest)",
        description: "Groq-hosted Whisper Large V3 Turbo, a fine-tuned and pruned Large V3 variant designed for fast multilingual transcription with strong price/performance tradeoffs.",
        languages: &["Multilingual"],
    },
];

const OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "language",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "prompt",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "response_format",
        kind: ModelOptionKind::String,
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

    if let Some(value) = options.get("response_format") {
        super::validate_string_value(
            full_model_id,
            "response_format",
            value,
            &["json", "verbose_json"],
        )?;
    }

    if let Some(value) = options.get("timestamp_granularities") {
        super::validate_string_list_values(
            full_model_id,
            "timestamp_granularities",
            value,
            &["word", "segment"],
        )?;
        if let Some(ModelOptionValue::String(response_format)) = options.get("response_format") {
            if response_format != "verbose_json" {
                anyhow::bail!(
                    "Invalid options for '{}'. Groq timestamp_granularities requires response_format = \"verbose_json\".",
                    full_model_id
                );
            }
        }
    }

    Ok(())
}

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
pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| anyhow::anyhow!("Failed to read audio file: {e}"))?;

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
        .text("model", config.model_id.clone());

    // Debug log: Log the API call details (without the audio data)
    let mut debug_params = vec![format!("model={}", config.model_id)];

    if let Some(language) = config.option_string("language") {
        if !language.is_empty() {
            form = form.text("language", language.to_string());
            debug_params.push(format!("language={language}"));
        }
    }

    if let Some(temperature) = config.option_number("temperature") {
        form = form.text("temperature", temperature.to_string());
        debug_params.push(format!("temperature={temperature}"));
    }

    let prompt = config
        .option_string("prompt")
        .map(ToString::to_string)
        .or_else(|| (!config.keywords.is_empty()).then(|| config.keywords.join(", ")));
    if let Some(prompt) = prompt {
        form = form.text("prompt", prompt.clone());
        debug_params.push(format!("prompt={prompt}"));
        tracing::debug!("Prompt used for Groq model: {prompt}");
    }

    if let Some(response_format) = response_format(config) {
        form = form.text("response_format", response_format.to_string());
        debug_params.push(format!("response_format={response_format}"));
    }

    if let Some(granularities) = config.option_string_list("timestamp_granularities") {
        for granularity in granularities {
            form = form.text("timestamp_granularities[]", granularity.clone());
            debug_params.push(format!("timestamp_granularities[]={granularity}"));
        }
    }

    let endpoint = config.endpoint;

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
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

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

    Ok(groq_response.text.trim().to_string())
}

fn response_format(config: &TranscriptionConfig) -> Option<&str> {
    if let Some(response_format) = config.option_string("response_format") {
        return Some(response_format);
    }

    if config
        .option_string_list("timestamp_granularities")
        .is_some_and(|granularities| !granularities.is_empty())
    {
        return Some("verbose_json");
    }

    Some("json")
}
