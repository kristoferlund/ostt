//! Deepgram API implementation.
//!
//! Handles transcription requests to Deepgram's API using binary audio data.

use serde::Deserialize;
use std::path::Path;
use urlencoding;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "deepgram",
        model_id: "nova-3",
        endpoint: "https://api.deepgram.com/v1/listen",
        display_name: "Nova 3 (latest, fastest)",
        description: "Deepgram's highest-performing general-purpose ASR model for batch or streaming use cases including meetings, event captioning, multi-speaker audio, noisy audio, far-field audio, and multilingual transcription.",
        languages: &["Multilingual", "Swedish", "Norwegian", "English"],
    },
    ModelSpec {
        provider_id: "deepgram",
        model_id: "nova-2",
        endpoint: "https://api.deepgram.com/v1/listen",
        display_name: "Nova 2 (previous generation)",
        description: "Deepgram's previous-generation Nova model. Useful when a language or feature is better covered by Nova 2, including filler word identification and broad multilingual speech recognition.",
        languages: &["Multilingual", "Swedish", "Norwegian", "English"],
    },
];

const fn option(name: &'static str, kind: ModelOptionKind) -> ModelOptionSpec {
    ModelOptionSpec { name, kind }
}

const NOVA_3_OPTIONS: &[ModelOptionSpec] = &[
    option("custom_intent", ModelOptionKind::StringList),
    option("custom_intent_mode", ModelOptionKind::String),
    option("custom_topic", ModelOptionKind::StringList),
    option("custom_topic_mode", ModelOptionKind::String),
    option("detect_entities", ModelOptionKind::Bool),
    option("detect_language", ModelOptionKind::BoolOrStringList),
    option("diarize", ModelOptionKind::Bool),
    option("diarize_model", ModelOptionKind::String),
    option("dictation", ModelOptionKind::Bool),
    option("encoding", ModelOptionKind::String),
    option("extra", ModelOptionKind::StringList),
    option("filler_words", ModelOptionKind::Bool),
    option("intents", ModelOptionKind::Bool),
    option("keyterm", ModelOptionKind::StringList),
    option("keywords", ModelOptionKind::StringList),
    option("language", ModelOptionKind::String),
    option("measurements", ModelOptionKind::Bool),
    option("mip_opt_out", ModelOptionKind::Bool),
    option("multichannel", ModelOptionKind::Bool),
    option("numerals", ModelOptionKind::Bool),
    option("paragraphs", ModelOptionKind::Bool),
    option("profanity_filter", ModelOptionKind::Bool),
    option("punctuate", ModelOptionKind::Bool),
    option("redact", ModelOptionKind::StringList),
    option("replace", ModelOptionKind::StringList),
    option("search", ModelOptionKind::StringList),
    option("sentiment", ModelOptionKind::Bool),
    option("smart_format", ModelOptionKind::Bool),
    option("summarize", ModelOptionKind::BoolOrString),
    option("tag", ModelOptionKind::StringList),
    option("topics", ModelOptionKind::Bool),
    option("utterances", ModelOptionKind::Bool),
    option("utt_split", ModelOptionKind::Number),
    option("version", ModelOptionKind::String),
];

const NOVA_2_OPTIONS: &[ModelOptionSpec] = &[
    option("custom_intent", ModelOptionKind::StringList),
    option("custom_intent_mode", ModelOptionKind::String),
    option("custom_topic", ModelOptionKind::StringList),
    option("custom_topic_mode", ModelOptionKind::String),
    option("detect_entities", ModelOptionKind::Bool),
    option("detect_language", ModelOptionKind::BoolOrStringList),
    option("diarize", ModelOptionKind::Bool),
    option("diarize_model", ModelOptionKind::String),
    option("dictation", ModelOptionKind::Bool),
    option("encoding", ModelOptionKind::String),
    option("extra", ModelOptionKind::StringList),
    option("filler_words", ModelOptionKind::Bool),
    option("intents", ModelOptionKind::Bool),
    option("keywords", ModelOptionKind::StringList),
    option("language", ModelOptionKind::String),
    option("measurements", ModelOptionKind::Bool),
    option("mip_opt_out", ModelOptionKind::Bool),
    option("multichannel", ModelOptionKind::Bool),
    option("numerals", ModelOptionKind::Bool),
    option("paragraphs", ModelOptionKind::Bool),
    option("profanity_filter", ModelOptionKind::Bool),
    option("punctuate", ModelOptionKind::Bool),
    option("redact", ModelOptionKind::StringList),
    option("replace", ModelOptionKind::StringList),
    option("search", ModelOptionKind::StringList),
    option("sentiment", ModelOptionKind::Bool),
    option("smart_format", ModelOptionKind::Bool),
    option("summarize", ModelOptionKind::BoolOrString),
    option("tag", ModelOptionKind::StringList),
    option("topics", ModelOptionKind::Bool),
    option("utterances", ModelOptionKind::Bool),
    option("utt_split", ModelOptionKind::Number),
    option("version", ModelOptionKind::String),
];

pub(super) fn option_schema(model_id: &str) -> Option<ModelOptionSchema> {
    match model_id {
        "nova-3" => Some(ModelOptionSchema::new(NOVA_3_OPTIONS)),
        "nova-2" => Some(ModelOptionSchema::new(NOVA_2_OPTIONS)),
        _ => None,
    }
}

fn push_query_pair(url: &mut String, name: &str, value: &str) {
    url.push_str(&format!("&{}={}", name, urlencoding::encode(value)));
}

fn push_bool_option(config: &TranscriptionConfig, url: &mut String, name: &str) {
    if let Some(value) = config.option_bool(name) {
        url.push_str(&format!("&{name}={value}"));
    }
}

fn push_string_option(config: &TranscriptionConfig, url: &mut String, name: &str) {
    if let Some(value) = config.option_string(name) {
        push_query_pair(url, name, value);
    }
}

fn push_string_list_option(config: &TranscriptionConfig, url: &mut String, name: &str) {
    if let Some(values) = config.option_string_list(name) {
        for value in values {
            push_query_pair(url, name, value);
        }
    }
}

fn push_bool_or_string_option(config: &TranscriptionConfig, url: &mut String, name: &str) {
    match config.model_options.get(name) {
        Some(ModelOptionValue::Bool(value)) => url.push_str(&format!("&{name}={value}")),
        Some(ModelOptionValue::String(value)) => push_query_pair(url, name, value),
        _ => {}
    }
}

fn push_bool_or_string_list_option(config: &TranscriptionConfig, url: &mut String, name: &str) {
    match config.model_options.get(name) {
        Some(ModelOptionValue::Bool(value)) => url.push_str(&format!("&{name}={value}")),
        Some(ModelOptionValue::StringList(values)) => {
            for value in values {
                push_query_pair(url, name, value);
            }
        }
        _ => {}
    }
}

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
pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| anyhow::anyhow!("Failed to read audio file: {e}"))?;

    let client = reqwest::Client::new();

    // Build the API URL with query parameters
    let mut url = format!("{}?model={}", config.endpoint, config.model_id);

    for name in [
        "detect_entities",
        "diarize",
        "dictation",
        "filler_words",
        "intents",
        "measurements",
        "mip_opt_out",
        "multichannel",
        "numerals",
        "paragraphs",
        "profanity_filter",
        "punctuate",
        "sentiment",
        "smart_format",
        "topics",
        "utterances",
    ] {
        push_bool_option(config, &mut url, name);
    }

    for name in [
        "custom_intent_mode",
        "custom_topic_mode",
        "diarize_model",
        "encoding",
        "language",
        "version",
    ] {
        push_string_option(config, &mut url, name);
    }

    for name in [
        "custom_intent",
        "custom_topic",
        "extra",
        "keyterm",
        "keywords",
        "redact",
        "replace",
        "search",
        "tag",
    ] {
        push_string_list_option(config, &mut url, name);
    }

    push_bool_or_string_list_option(config, &mut url, "detect_language");
    push_bool_or_string_option(config, &mut url, "summarize");

    if let Some(utt_split) = config.option_number("utt_split") {
        url.push_str(&format!("&utt_split={utt_split}"));
    }

    // Add keywords/keyterms if any (nova-3 uses keyterms, nova-2 uses keywords)
    if !config.keywords.is_empty() {
        let param_name = if config.model_id == "nova-3" {
            "keyterm"
        } else {
            "keywords"
        };
        if !config.model_options.contains_key(param_name) {
            for keyword in &config.keywords {
                push_query_pair(&mut url, param_name, keyword);
            }
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
                "Failed to connect to Deepgram API server. Check your internet connection."
                    .to_string()
            } else if e.is_timeout() {
                "Request to Deepgram timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!(
                    "Failed to build Deepgram API request: {e}. This may be a configuration error."
                )
            } else {
                format!("Deepgram network error: {e}")
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
