//! ElevenLabs Scribe API implementation.
//!
//! Handles transcription requests to ElevenLabs' speech-to-text API using
//! multipart form data. Supports the Scribe v2 and Scribe v1 models.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v2",
        endpoint: "https://api.elevenlabs.io/v1/speech-to-text",
        display_name: "Scribe v2 (highest accuracy, 99 languages)",
        description: "ElevenLabs' latest Scribe speech-to-text model for high-accuracy transcription with broad multilingual support, including support for many languages beyond English.",
        languages: &["Multilingual", "99 languages"],
    },
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v1",
        endpoint: "https://api.elevenlabs.io/v1/speech-to-text",
        display_name: "Scribe v1 (previous generation)",
        description: "ElevenLabs' previous-generation Scribe speech-to-text model for multilingual transcription workloads.",
        languages: &["Multilingual", "99 languages"],
    },
];

const SCRIBE_V1_OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "language_code",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "tag_audio_events",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "timestamps_granularity",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "diarize",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "num_speakers",
        kind: ModelOptionKind::Integer,
    },
    ModelOptionSpec {
        name: "diarization_threshold",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "file_format",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "seed",
        kind: ModelOptionKind::Integer,
    },
    ModelOptionSpec {
        name: "use_multi_channel",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "keyterms",
        kind: ModelOptionKind::StringList,
    },
];

const SCRIBE_V2_OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "language_code",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "tag_audio_events",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "timestamps_granularity",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "diarize",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "detect_speaker_roles",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "num_speakers",
        kind: ModelOptionKind::Integer,
    },
    ModelOptionSpec {
        name: "diarization_threshold",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "file_format",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "seed",
        kind: ModelOptionKind::Integer,
    },
    ModelOptionSpec {
        name: "use_multi_channel",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "keyterms",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "no_verbatim",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "entity_detection",
        kind: ModelOptionKind::StringOrStringList,
    },
    ModelOptionSpec {
        name: "entity_redaction",
        kind: ModelOptionKind::StringOrStringList,
    },
    ModelOptionSpec {
        name: "entity_redaction_mode",
        kind: ModelOptionKind::String,
    },
];

pub(super) fn option_schema(model_id: &str) -> Option<ModelOptionSchema> {
    match model_id {
        "scribe_v2" => Some(ModelOptionSchema::new(SCRIBE_V2_OPTIONS)),
        "scribe_v1" => Some(ModelOptionSchema::new(SCRIBE_V1_OPTIONS)),
        _ => None,
    }
}

pub(super) fn validate_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    super::validate_number_range(full_model_id, options, "diarization_threshold", 0.1..=0.4)?;
    super::validate_number_range(full_model_id, options, "temperature", 0.0..=2.0)?;
    super::validate_integer_range(full_model_id, options, "num_speakers", 1..=32)?;
    super::validate_integer_range(full_model_id, options, "seed", 0..=2_147_483_647)?;

    if let Some(value) = options.get("timestamps_granularity") {
        super::validate_string_value(
            full_model_id,
            "timestamps_granularity",
            value,
            &["none", "word", "character"],
        )?;
    }

    if let Some(value) = options.get("file_format") {
        super::validate_string_value(
            full_model_id,
            "file_format",
            value,
            &["pcm_s16le_16", "other"],
        )?;
    }

    if let Some(value) = options.get("entity_redaction_mode") {
        super::validate_string_value(
            full_model_id,
            "entity_redaction_mode",
            value,
            &["redacted", "entity_type", "enumerated_entity_type"],
        )?;
    }

    if options.contains_key("diarization_threshold") {
        if matches!(options.get("diarize"), Some(ModelOptionValue::Bool(false))) {
            anyhow::bail!(
                "Invalid params for '{}'. ElevenLabs diarization_threshold requires diarize = true.",
                full_model_id
            );
        }
        if options.contains_key("num_speakers") {
            anyhow::bail!(
                "Invalid params for '{}'. ElevenLabs diarization_threshold cannot be used with num_speakers.",
                full_model_id
            );
        }
    }

    if matches!(
        options.get("detect_speaker_roles"),
        Some(ModelOptionValue::Bool(true))
    ) {
        if !matches!(options.get("diarize"), Some(ModelOptionValue::Bool(true))) {
            anyhow::bail!(
                "Invalid params for '{}'. ElevenLabs detect_speaker_roles requires diarize = true.",
                full_model_id
            );
        }
        if matches!(
            options.get("use_multi_channel"),
            Some(ModelOptionValue::Bool(true))
        ) {
            anyhow::bail!(
                "Invalid params for '{}'. ElevenLabs detect_speaker_roles cannot be used with use_multi_channel = true.",
                full_model_id
            );
        }
    }

    Ok(())
}

/// ElevenLabs speech-to-text response structure
#[derive(Debug, Deserialize)]
struct ElevenLabsResponse {
    /// The transcribed text
    text: String,
}

/// Transcribes an audio file using ElevenLabs' Scribe API.
///
/// Sends multipart form data with `xi-api-key` header authentication.
/// Keywords are passed as `keyterms` to improve transcription accuracy for
/// domain-specific terms.
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
        .text("model_id", config.model_id.clone())
        .part("file", file_part);

    if let Some(language_code) = config.option_string("language_code") {
        if !language_code.is_empty() {
            form = form.text("language_code", language_code.to_string());
        }
    }
    if let Some(tag_audio_events) = config.option_bool("tag_audio_events") {
        form = form.text("tag_audio_events", tag_audio_events.to_string());
    }
    if let Some(timestamps_granularity) = config.option_string("timestamps_granularity") {
        form = form.text("timestamps_granularity", timestamps_granularity.to_string());
    }
    if let Some(diarize) = config.option_bool("diarize") {
        form = form.text("diarize", diarize.to_string());
    }
    if let Some(detect_speaker_roles) = config.option_bool("detect_speaker_roles") {
        form = form.text("detect_speaker_roles", detect_speaker_roles.to_string());
    }
    if let Some(num_speakers) = config.option_integer("num_speakers") {
        form = form.text("num_speakers", num_speakers.to_string());
    }
    if let Some(diarization_threshold) = config.option_number("diarization_threshold") {
        form = form.text("diarization_threshold", diarization_threshold.to_string());
    }
    if let Some(temperature) = config.option_number("temperature") {
        form = form.text("temperature", temperature.to_string());
    }
    if let Some(file_format) = config.option_string("file_format") {
        form = form.text("file_format", file_format.to_string());
    }
    if let Some(seed) = config.option_integer("seed") {
        form = form.text("seed", seed.to_string());
    }
    if let Some(use_multi_channel) = config.option_bool("use_multi_channel") {
        form = form.text("use_multi_channel", use_multi_channel.to_string());
    }
    if let Some(no_verbatim) = config.option_bool("no_verbatim") {
        form = form.text("no_verbatim", no_verbatim.to_string());
    }
    form = add_string_or_string_list_fields(form, config, "entity_detection");
    form = add_string_or_string_list_fields(form, config, "entity_redaction");
    if let Some(entity_redaction_mode) = config.option_string("entity_redaction_mode") {
        form = form.text("entity_redaction_mode", entity_redaction_mode.to_string());
    }

    if let Some(keyterms) = config.option_string_list("keyterms") {
        for keyterm in keyterms {
            form = form.text("keyterms", keyterm.clone());
        }
    } else {
        // Each keyterm is passed as a separate form field.
        for keyword in &config.keywords {
            form = form.text("keyterms", keyword.clone());
        }
    }

    let client = reqwest::Client::new();
    let url = &config.endpoint;

    tracing::debug!(
        "ElevenLabs API Call:\n  URL: {}\n  Method: POST\n  Model: {}\n  Keyterms: {:?}",
        url,
        config.model_id,
        config.keywords,
    );

    let response = match client
        .post(url)
        .header("xi-api-key", &config.api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Failed to connect to ElevenLabs API server. Check your internet connection."
                    .to_string()
            } else if e.is_timeout() {
                "Request to ElevenLabs timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!("Failed to build ElevenLabs API request: {e}. This may be a configuration error.")
            } else {
                format!("ElevenLabs network error: {e}")
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
            401 => "ElevenLabs API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use ElevenLabs' API. Check your API key and account status.".to_string(),
            422 => format!("ElevenLabs API validation error: {error_body}"),
            429 => "Too many requests to ElevenLabs. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "ElevenLabs API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("ElevenLabs API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let elevenlabs_response: ElevenLabsResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse ElevenLabs response: {e}"))?;

    Ok(elevenlabs_response.text.trim().to_string())
}

fn add_string_or_string_list_fields(
    mut form: reqwest::multipart::Form,
    config: &TranscriptionConfig,
    name: &'static str,
) -> reqwest::multipart::Form {
    match config.params.get(name) {
        Some(ModelOptionValue::String(value)) => form = form.text(name, value.clone()),
        Some(ModelOptionValue::StringList(values)) => {
            for value in values {
                form = form.text(name, value.clone());
            }
        }
        _ => {}
    }

    form
}
