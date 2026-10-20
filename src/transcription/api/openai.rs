//! OpenAI Whisper API implementation.
//!
//! Handles transcription requests to OpenAI's Whisper API using multipart form data.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "openai",
        model_id: "gpt-transcribe",
        endpoint: "https://api.openai.com/v1/audio/transcriptions",
        display_name: "GPT Transcribe (recommended)",
        description: "OpenAI's recommended file-transcription model. Supports contextual prompts, keyword hints and multiple language hints for multilingual audio and code-switching.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "openai",
        model_id: "gpt-4o-transcribe",
        endpoint: "https://api.openai.com/v1/audio/transcriptions",
        display_name: "GPT-4o Transcribe",
        description: "OpenAI's higher-quality speech-to-text model for transcribing audio in the source language. Supports prompting for domain terms, names, and preferred writing style.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "openai",
        model_id: "gpt-4o-mini-transcribe",
        endpoint: "https://api.openai.com/v1/audio/transcriptions",
        display_name: "GPT-4o Mini Transcribe (faster, lighter)",
        description: "OpenAI's smaller GPT-4o transcription model for lower-cost speech-to-text with contextual prompts and JSON output.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "openai",
        model_id: "gpt-4o-transcribe-diarize",
        endpoint: "https://api.openai.com/v1/audio/transcriptions",
        display_name: "GPT-4o Transcribe Diarize",
        description: "OpenAI's GPT-4o transcription model for diarized JSON output with speaker-segment annotations.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "openai",
        model_id: "whisper-1",
        endpoint: "https://api.openai.com/v1/audio/transcriptions",
        display_name: "Whisper (legacy)",
        description: "OpenAI's hosted Whisper model for multilingual transcription with optional segment or word timestamps. OSTT requests JSON output and transcribes in the source language.",
        languages: &["Multilingual"],
    },
];

const TRANSCRIBE_OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "languages",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "keywords",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "prompt",
        kind: ModelOptionKind::String,
    },
];

const GPT_4O_OPTIONS: &[ModelOptionSpec] = &[
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
        name: "include",
        kind: ModelOptionKind::StringList,
    },
];

const DIARIZE_OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "language",
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
        name: "chunking_strategy",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "known_speaker_names",
        kind: ModelOptionKind::StringList,
    },
    ModelOptionSpec {
        name: "known_speaker_references",
        kind: ModelOptionKind::StringList,
    },
];

const WHISPER_OPTIONS: &[ModelOptionSpec] = &[
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

pub(super) fn option_schema(model_id: &str) -> Option<ModelOptionSchema> {
    match model_id {
        "gpt-transcribe" => Some(ModelOptionSchema::new(TRANSCRIBE_OPTIONS)),
        "gpt-4o-transcribe" | "gpt-4o-mini-transcribe" => {
            Some(ModelOptionSchema::new(GPT_4O_OPTIONS))
        }
        "gpt-4o-transcribe-diarize" => Some(ModelOptionSchema::new(DIARIZE_OPTIONS)),
        "whisper-1" => Some(ModelOptionSchema::new(WHISPER_OPTIONS)),
        _ => None,
    }
}

pub(super) fn validate_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    super::validate_number_range(full_model_id, options, "temperature", 0.0..=1.0)?;

    if let Some(ModelOptionValue::StringList(keywords)) = options.get("keywords") {
        validate_keywords(keywords)?;
    }

    if let Some(value) = options.get("include") {
        super::validate_string_list_values(full_model_id, "include", value, &["logprobs"])?;
    }

    if let Some(value) = options.get("timestamp_granularities") {
        super::validate_string_list_values(
            full_model_id,
            "timestamp_granularities",
            value,
            &["word", "segment"],
        )?;
    }

    if let Some(value) = options.get("chunking_strategy") {
        super::validate_string_value(full_model_id, "chunking_strategy", value, &["auto"])?;
    }

    if let Some(value) = options.get("response_format") {
        let allowed = if full_model_id == "openai/gpt-4o-transcribe-diarize" {
            &["json", "diarized_json"][..]
        } else {
            &["json", "verbose_json"][..]
        };
        super::validate_string_value(full_model_id, "response_format", value, allowed)?;
    }

    if full_model_id == "openai/whisper-1" && options.contains_key("timestamp_granularities") {
        if let Some(ModelOptionValue::String(response_format)) = options.get("response_format") {
            if response_format != "verbose_json" {
                anyhow::bail!(
                    "Invalid params for '{}'. OpenAI timestamp_granularities requires response_format = \"verbose_json\".",
                    full_model_id
                );
            }
        }
    }

    Ok(())
}

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

    // Diarization does not support prompts; GPT Transcribe has a dedicated
    // keyword field instead of folding saved keywords into contextual prose.
    let prompt = config
        .option_string("prompt")
        .map(ToString::to_string)
        .or_else(|| {
            (!matches!(
                config.model_id.as_str(),
                "gpt-transcribe" | "gpt-4o-transcribe-diarize"
            ) && !config.keywords.is_empty())
            .then(|| config.keywords.join(", "))
        });
    if let Some(prompt) = prompt {
        form = form.text("prompt", prompt.clone());
        debug_params.push(format!("prompt={prompt}"));
        tracing::debug!("Prompt used for OpenAI model: {prompt}");
    }

    if config.model_id == "gpt-transcribe" {
        let keywords = config
            .option_string_list("keywords")
            .unwrap_or(&config.keywords);
        validate_keywords(keywords)?;
        form = add_string_list(form, &mut debug_params, "keywords[]", keywords);
        if let Some(languages) = config.option_string_list("languages") {
            form = add_string_list(form, &mut debug_params, "languages[]", languages);
        }
    }

    if let Some(response_format) = response_format(config) {
        form = form.text("response_format", response_format.to_string());
        debug_params.push(format!("response_format={response_format}"));
    }

    if let Some(include) = config.option_string_list("include") {
        form = add_string_list(form, &mut debug_params, "include[]", include);
    }

    if let Some(granularities) = config.option_string_list("timestamp_granularities") {
        form = add_string_list(
            form,
            &mut debug_params,
            "timestamp_granularities[]",
            granularities,
        );
    }

    let chunking_strategy = config
        .option_string("chunking_strategy")
        .or_else(|| (config.model_id == "gpt-4o-transcribe-diarize").then_some("auto"));
    if let Some(chunking_strategy) = chunking_strategy {
        form = form.text("chunking_strategy", chunking_strategy.to_string());
        debug_params.push(format!("chunking_strategy={chunking_strategy}"));
    }

    if let Some(names) = config.option_string_list("known_speaker_names") {
        form = add_string_list(form, &mut debug_params, "known_speaker_names[]", names);
    }

    if let Some(references) = config.option_string_list("known_speaker_references") {
        form = add_string_list(
            form,
            &mut debug_params,
            "known_speaker_references[]",
            references,
        );
    }

    let url = config.endpoint.clone();

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
                "Failed to connect to OpenAI API server. Check your internet connection."
                    .to_string()
            } else if e.is_timeout() {
                "Request to OpenAI timed out. The API server is not responding.".to_string()
            } else if e.to_string().contains("builder") {
                format!(
                    "Failed to build OpenAI API request: {e}. This may be a configuration error."
                )
            } else {
                format!("OpenAI network error: {e}")
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

fn response_format(config: &TranscriptionConfig) -> Option<&str> {
    if let Some(response_format) = config.option_string("response_format") {
        return Some(response_format);
    }

    if config.model_id == "gpt-4o-transcribe-diarize" {
        return Some("diarized_json");
    }

    // Use GPT Transcribe's default JSON response; only send its documented
    // context fields, rather than assuming GPT-4o's optional controls apply.
    if config.model_id == "gpt-transcribe" {
        return None;
    }

    if config.model_id == "whisper-1"
        && config
            .option_string_list("timestamp_granularities")
            .is_some_and(|granularities| !granularities.is_empty())
    {
        return Some("verbose_json");
    }

    Some("json")
}

fn validate_keywords(keywords: &[String]) -> anyhow::Result<()> {
    if keywords
        .iter()
        .any(|keyword| keyword.contains(['<', '>', '\r', '\n']))
    {
        anyhow::bail!("OpenAI GPT Transcribe keywords cannot contain <, > or line breaks.");
    }
    Ok(())
}

fn add_string_list(
    mut form: reqwest::multipart::Form,
    debug_params: &mut Vec<String>,
    field_name: &'static str,
    values: &[String],
) -> reqwest::multipart::Form {
    for value in values {
        form = form.text(field_name, value.clone());
        debug_params.push(format!("{field_name}={value}"));
    }

    form
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::api::test_support::mock_server;
    use crate::transcription::provider::TranscriptionProvider;

    #[tokio::test]
    async fn gpt_transcribe_sends_native_context_fields_and_reads_language_response() {
        for explicit_keywords in [false, true] {
            let mut server = mock_server(
                TranscriptionProvider::OpenAI,
                "gpt-transcribe",
                &[(200, r#"{"text":" Bonjour! ","languages":[{"code":"fr"}]}"#)],
            )
            .await;
            server.config.keywords = vec!["saved keyword".into()];
            server.config.params.insert(
                "languages".into(),
                ModelOptionValue::StringList(vec!["en".into(), "fr".into()]),
            );
            server.config.params.insert(
                "prompt".into(),
                ModelOptionValue::String("A support call".into()),
            );
            if explicit_keywords {
                server.config.params.insert(
                    "keywords".into(),
                    ModelOptionValue::StringList(vec!["AC-42".into()]),
                );
            }
            assert_eq!(
                transcribe(&server.config, &server.audio_path)
                    .await
                    .unwrap(),
                "Bonjour!"
            );
            let requests = (&mut server.task).await.unwrap();
            let request = &requests[0];
            assert!(request.contains("name=\"model\"\r\n\r\ngpt-transcribe\r\n"));
            assert!(request.contains("name=\"languages[]\"\r\n\r\nfr\r\n"));
            assert!(request.contains("name=\"prompt\"\r\n\r\nA support call\r\n"));
            let keyword = if explicit_keywords {
                "AC-42"
            } else {
                "saved keyword"
            };
            assert!(request.contains(&format!("name=\"keywords[]\"\r\n\r\n{keyword}\r\n")));
            assert_eq!(request.matches("name=\"keywords[]\"").count(), 1);
            assert!(!request.contains("name=\"language\""));
            assert!(!request.contains("name=\"response_format\""));
        }
    }

    #[tokio::test]
    async fn diarization_never_receives_saved_keywords_and_chunks_long_audio() {
        let mut server = mock_server(
            TranscriptionProvider::OpenAI,
            "gpt-4o-transcribe-diarize",
            &[(
                200,
                r#"{"text":" Hello ","segments":[{"speaker":"A","text":"Hello"}]}"#,
            )],
        )
        .await;
        server.config.keywords = vec!["OSTT".into()];
        assert_eq!(
            transcribe(&server.config, &server.audio_path)
                .await
                .unwrap(),
            "Hello"
        );
        let requests = (&mut server.task).await.unwrap();
        assert!(!requests[0].contains("name=\"prompt\""));
        assert!(!requests[0].contains("name=\"keywords[]\""));
        assert!(requests[0].contains("name=\"chunking_strategy\"\r\n\r\nauto\r\n"));
        assert!(requests[0].contains("name=\"response_format\"\r\n\r\ndiarized_json\r\n"));
        assert!(option_schema("gpt-4o-transcribe-diarize")
            .unwrap()
            .option("prompt")
            .is_none());
    }

    #[tokio::test]
    async fn gpt_transcribe_rejects_invalid_saved_keywords_before_upload() {
        let mut server = mock_server(TranscriptionProvider::OpenAI, "gpt-transcribe", &[]).await;
        for keyword in ["<term>", "two\nlines", "two\rlines"] {
            server.config.keywords = vec![keyword.into()];
            assert!(transcribe(&server.config, &server.audio_path)
                .await
                .unwrap_err()
                .to_string()
                .contains("keywords cannot contain"));
        }
    }

    #[tokio::test]
    async fn new_model_preserves_provider_errors_and_rejects_malformed_responses() {
        for (status, body, expected) in [
            (400, "invalid languages", "invalid languages"),
            (200, "{}", "Failed to parse OpenAI response"),
        ] {
            let server = mock_server(
                TranscriptionProvider::OpenAI,
                "gpt-transcribe",
                &[(status, body)],
            )
            .await;
            assert!(transcribe(&server.config, &server.audio_path)
                .await
                .unwrap_err()
                .to_string()
                .contains(expected));
        }
    }
}
