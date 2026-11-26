//! ElevenLabs Scribe API implementation.
//!
//! Handles transcription requests to ElevenLabs' speech-to-text API using
//! multipart form data. Supports the Scribe v3, v2, Medical and legacy v1 models.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v3",
        endpoint: "https://api.elevenlabs.io/v1/speech-to-text",
        display_name: "Scribe v3 (general purpose)",
        description: "ElevenLabs' most accurate Scribe speech-to-text model for recorded audio, the successor to Scribe v2. Adds number formatting and secondary language hints.",
        languages: &["Multilingual", "90+ languages"],
    },
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v2",
        endpoint: "https://api.elevenlabs.io/v1/speech-to-text",
        display_name: "Scribe v2 (previous generation)",
        description: "ElevenLabs' previous Scribe speech-to-text model for high-accuracy transcription with broad multilingual support, including support for many languages beyond English.",
        languages: &["Multilingual", "90+ languages"],
    },
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v2_medical",
        endpoint: "https://api.elevenlabs.io/v1/speech-to-text",
        display_name: "Scribe v2 Medical",
        description: "Scribe v2 specialized for clinical audio and medical terminology, with the same multilingual coverage and API as Scribe v2.",
        languages: &["Multilingual", "90+ languages"],
    },
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v1",
        endpoint: "https://api.elevenlabs.io/v1/speech-to-text",
        display_name: "Scribe v1 (legacy)",
        description: "ElevenLabs' previous-generation Scribe speech-to-text model for multilingual transcription workloads.",
        languages: &["Multilingual", "90+ languages"],
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

const SCRIBE_V3_OPTIONS: &[ModelOptionSpec] = &[
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
    ModelOptionSpec {
        name: "numbers_format",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "secondary_language_codes",
        kind: ModelOptionKind::StringList,
    },
];

pub(super) fn option_schema(model_id: &str) -> Option<ModelOptionSchema> {
    match model_id {
        "scribe_v3" => Some(ModelOptionSchema::new(SCRIBE_V3_OPTIONS)),
        "scribe_v2" | "scribe_v2_medical" => Some(ModelOptionSchema::new(SCRIBE_V2_OPTIONS)),
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

    if let Some(value) = options.get("numbers_format") {
        super::validate_string_value(full_model_id, "numbers_format", value, &["digits", "words"])?;
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

/// Checks the provider, model and `--param` options once they are combined:
/// `language_code` may come from a different section than the secondary codes.
pub(super) fn validate_merged_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    let has_secondary_codes = matches!(
        options.get("secondary_language_codes"),
        Some(ModelOptionValue::StringList(codes)) if !codes.is_empty()
    );
    if has_secondary_codes && !options.contains_key("language_code") {
        anyhow::bail!(
            "Invalid params for '{}'. ElevenLabs secondary_language_codes requires language_code.",
            full_model_id
        );
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
    if let Some(numbers_format) = config.option_string("numbers_format") {
        form = form.text("numbers_format", numbers_format.to_string());
    }
    // Each secondary language code is passed as a separate form field.
    if let Some(codes) = config.option_string_list("secondary_language_codes") {
        for code in codes {
            form = form.text("secondary_language_codes", code.clone());
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::api::test_support::mock_server;
    use crate::transcription::provider::TranscriptionProvider;

    #[tokio::test]
    async fn medical_model_uses_scribe_file_endpoint_and_v2_controls() {
        let mut server = mock_server(
            TranscriptionProvider::ElevenLabs,
            "scribe_v2_medical",
            &[(200, r#"{"text":" Clinical note. "}"#)],
        )
        .await;
        server.config.keywords = vec!["metformin".into()];
        server
            .config
            .params
            .insert("diarize".into(), ModelOptionValue::Bool(true));
        assert_eq!(
            transcribe(&server.config, &server.audio_path)
                .await
                .unwrap(),
            "Clinical note."
        );
        let requests = (&mut server.task).await.unwrap();
        assert!(requests[0].contains("name=\"model_id\"\r\n\r\nscribe_v2_medical\r\n"));
        assert!(requests[0].contains("name=\"diarize\"\r\n\r\ntrue\r\n"));
        assert!(requests[0].contains("name=\"keyterms\"\r\n\r\nmetformin\r\n"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("xi-api-key: test-key\r\n"));
        assert_eq!(
            option_schema("scribe_v2"),
            option_schema("scribe_v2_medical")
        );
    }

    /// ElevenLabs rejects a JSON array sent as one field, so every secondary
    /// language code must be its own `secondary_language_codes` form field.
    #[tokio::test]
    async fn scribe_v3_sends_number_format_and_one_field_per_secondary_language() {
        let mut server = mock_server(
            TranscriptionProvider::ElevenLabs,
            "scribe_v3",
            &[(200, r#"{"text":" 42 kronor. "}"#)],
        )
        .await;
        server.config.params.insert(
            "numbers_format".into(),
            ModelOptionValue::String("digits".into()),
        );
        server.config.params.insert(
            "language_code".into(),
            ModelOptionValue::String("en".into()),
        );
        server.config.params.insert(
            "secondary_language_codes".into(),
            ModelOptionValue::StringList(vec!["sv".into(), "de".into()]),
        );
        server
            .config
            .params
            .insert("no_verbatim".into(), ModelOptionValue::Bool(true));
        assert_eq!(
            transcribe(&server.config, &server.audio_path)
                .await
                .unwrap(),
            "42 kronor."
        );
        let requests = (&mut server.task).await.unwrap();
        assert!(requests[0].contains("name=\"model_id\"\r\n\r\nscribe_v3\r\n"));
        assert!(requests[0].contains("name=\"numbers_format\"\r\n\r\ndigits\r\n"));
        assert!(requests[0].contains("name=\"secondary_language_codes\"\r\n\r\nsv\r\n"));
        assert!(requests[0].contains("name=\"secondary_language_codes\"\r\n\r\nde\r\n"));
        assert!(requests[0].contains("name=\"no_verbatim\"\r\n\r\ntrue\r\n"));
    }

    /// Scribe v3 takes every Scribe v2 param unchanged, and ElevenLabs returns
    /// `invalid_parameters` when the two v3-only params are sent to another
    /// model, so they must not be offered for Scribe v2.
    #[test]
    fn v3_only_params_extend_the_v2_schema_and_stay_off_older_models() {
        let v3 = option_schema("scribe_v3").unwrap();
        let v2 = option_schema("scribe_v2").unwrap();
        for option in v2.options() {
            assert_eq!(v3.option(option.name), Some(option), "{}", option.name);
        }
        for name in ["numbers_format", "secondary_language_codes"] {
            assert!(v3.option(name).is_some(), "{name}");
            for older in ["scribe_v2", "scribe_v2_medical", "scribe_v1"] {
                assert!(
                    option_schema(older).unwrap().option(name).is_none(),
                    "{older} {name}"
                );
            }
        }
    }

    /// ElevenLabs answers a request that has secondary language codes but no
    /// primary `language_code` with a server error, so it is refused up front.
    /// An empty list sends no field, so it needs no primary language.
    #[test]
    fn secondary_language_codes_require_a_primary_language_code() {
        let mut options = IndexMap::from([(
            "secondary_language_codes".to_string(),
            ModelOptionValue::StringList(vec![]),
        )]);
        assert!(validate_merged_options("elevenlabs/scribe_v3", &options).is_ok());
        options.insert(
            "secondary_language_codes".to_string(),
            ModelOptionValue::StringList(vec!["de".into()]),
        );
        assert!(validate_merged_options("elevenlabs/scribe_v3", &options)
            .unwrap_err()
            .to_string()
            .contains("secondary_language_codes requires language_code"));
        options.insert(
            "language_code".to_string(),
            ModelOptionValue::String("en".into()),
        );
        assert!(validate_merged_options("elevenlabs/scribe_v3", &options).is_ok());
    }

    /// A value ElevenLabs does not know must fail before the recording is uploaded.
    #[test]
    fn numbers_format_accepts_only_digits_or_words() {
        for (value, valid) in [("digits", true), ("words", true), ("roman", false)] {
            let options = IndexMap::from([(
                "numbers_format".to_string(),
                ModelOptionValue::String(value.into()),
            )]);
            assert_eq!(
                validate_options("elevenlabs/scribe_v3", &options).is_ok(),
                valid,
                "{value}"
            );
        }
    }
}
