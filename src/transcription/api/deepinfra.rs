//! DeepInfra API implementation.
//!
//! Handles transcription requests to DeepInfra's inference API using multipart form data.

use serde::Deserialize;
use std::path::Path;

use super::TranscriptionConfig;
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec, ModelSpec};
use indexmap::IndexMap;

pub(super) const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "openai/whisper-large-v3",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "OpenAI Whisper Large V3 (best accuracy)",
        description: "OpenAI Whisper Large V3 hosted through DeepInfra. A general-purpose multilingual Whisper model suited for high-accuracy transcription and translation workloads.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "openai/whisper-large-v3-turbo",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "OpenAI Whisper Large V3 Turbo (fast)",
        description: "OpenAI Whisper Large V3 Turbo hosted through DeepInfra. A pruned Large V3 variant designed for faster multilingual transcription with minor quality tradeoffs.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "Qwen/Qwen3-ASR-0.6B",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "Qwen3 ASR 0.6B (compact)",
        description: "Compact Qwen3 speech recognition model hosted through DeepInfra. Supports language identification and transcription across 30 languages and 22 Chinese dialects.",
        languages: &["Multilingual", "30 languages"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "Qwen/Qwen3-ASR-1.7B",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "Qwen3 ASR 1.7B",
        description: "Qwen3's larger speech recognition model hosted through DeepInfra, with multilingual transcription and optional word timestamps.",
        languages: &["Multilingual", "30 languages"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "nvidia/Nemotron-3.5-ASR-Streaming-Multilingual-0.6b",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "Nemotron 3.5 ASR Multilingual",
        description: "NVIDIA's 0.6B multilingual ASR model. DeepInfra exposes completed-file transcription through its standard inference endpoint despite the model's streaming name.",
        languages: &["Multilingual", "40+ languages"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "mistralai/Voxtral-Mini-3B-2507",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "Mistral Voxtral Mini 3B 2507",
        description: "Mistral Voxtral Mini hosted through DeepInfra. A 3B audio-capable model for transcription, translation, and audio understanding.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "mistralai/Voxtral-Small-24B-2507",
        endpoint: "https://api.deepinfra.com/v1/inference",
        display_name: "Mistral Voxtral Small 24B 2507",
        description: "Mistral Voxtral Small hosted through DeepInfra. A larger audio-capable model for transcription, translation, and audio understanding.",
        languages: &["Multilingual"],
    },
];

const OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "language",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "initial_prompt",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "task",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "chunk_level",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "chunk_length_s",
        kind: ModelOptionKind::Integer,
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

    if let Some(value) = options.get("task") {
        super::validate_string_value(full_model_id, "task", value, &["transcribe", "translate"])?;
    }

    if let Some(value) = options.get("chunk_level") {
        super::validate_string_value(full_model_id, "chunk_level", value, &["segment", "word"])?;
    }

    super::validate_integer_range(full_model_id, options, "chunk_length_s", 1..=30)?;

    Ok(())
}

/// DeepInfra API response structure
#[derive(Debug, Deserialize)]
struct DeepInfraResponse {
    text: String,
}

/// Transcribes an audio file using DeepInfra's Whisper API.
///
/// Uses multipart form data with bearer token authentication.
/// DeepInfra hosts OpenAI's Whisper model and compatible models.
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

    let mut form = reqwest::multipart::Form::new().part("audio", file_part);

    // Debug log: Log the API call details (without the audio data)
    let mut debug_params = vec![];

    // Build the URL with model name in the path
    let endpoint = format!("{}/{}", config.endpoint, config.model_id);

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

    if let Some(task) = config.option_string("task") {
        form = form.text("task", task.to_string());
        debug_params.push(format!("task={task}"));
    }

    let initial_prompt = config
        .option_string("initial_prompt")
        .map(ToString::to_string)
        .or_else(|| (!config.keywords.is_empty()).then(|| config.keywords.join(", ")));
    if let Some(initial_prompt) = initial_prompt {
        form = form.text("initial_prompt", initial_prompt.clone());
        debug_params.push(format!("initial_prompt={initial_prompt}"));
        tracing::debug!("Initial prompt used for DeepInfra model: {initial_prompt}");
    }

    if let Some(chunk_level) = config.option_string("chunk_level") {
        form = form.text("chunk_level", chunk_level.to_string());
        debug_params.push(format!("chunk_level={chunk_level}"));
    }

    if let Some(chunk_length_s) = config.option_integer("chunk_length_s") {
        form = form.text("chunk_length_s", chunk_length_s.to_string());
        debug_params.push(format!("chunk_length_s={chunk_length_s}"));
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
                "Failed to connect to DeepInfra API server. Check your internet connection."
                    .to_string()
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
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        let human_readable = match status.as_u16() {
            401 => "DeepInfra API key is invalid or expired. Please run 'ostt auth' to update your API key.".to_string(),
            403 => "You don't have permission to use DeepInfra's API. Check your API key and account status.".to_string(),
            429 => "Too many requests to DeepInfra. You've hit the API rate limit. Please wait and try again.".to_string(),
            500 | 502 | 503 | 504 => "DeepInfra API server is experiencing issues. Please try again later.".to_string(),
            _ => format!("DeepInfra API error (status {status}): {error_body}"),
        };

        return Err(anyhow::anyhow!(human_readable));
    }

    let deepinfra_response: DeepInfraResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse DeepInfra response: {e}"))?;

    // Debug log: Log the full response for debugging
    tracing::debug!(
        "DeepInfra API Response:\n  Status: Success\n  Transcription length: {} characters\n  Full response: {:#?}",
        deepinfra_response.text.len(),
        deepinfra_response
    );

    Ok(deepinfra_response.text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::api::test_support::mock_server;
    use crate::transcription::provider::TranscriptionProvider;

    #[tokio::test]
    async fn new_asr_models_use_the_documented_file_inference_contract() {
        for model in [
            "Qwen/Qwen3-ASR-0.6B",
            "Qwen/Qwen3-ASR-1.7B",
            "nvidia/Nemotron-3.5-ASR-Streaming-Multilingual-0.6b",
        ] {
            let mut server = mock_server(
                TranscriptionProvider::DeepInfra,
                model,
                &[(200, r#"{"text":" Hello! ","language":"en","segments":[]}"#)],
            )
            .await;
            server.config.endpoint.push_str("/v1/inference");
            server.config.keywords = vec!["OSTT".into()];
            server.config.params.insert(
                "chunk_level".into(),
                ModelOptionValue::String("word".into()),
            );
            assert_eq!(
                transcribe(&server.config, &server.audio_path)
                    .await
                    .unwrap(),
                "Hello!"
            );
            let requests = (&mut server.task).await.unwrap();
            assert!(requests[0].starts_with(&format!("POST /v1/inference/{model} HTTP/1.1\r\n")));
            assert!(requests[0].contains("name=\"audio\"; filename="));
            assert!(requests[0].contains("name=\"initial_prompt\"\r\n\r\nOSTT\r\n"));
            assert!(requests[0].contains("name=\"chunk_level\"\r\n\r\nword\r\n"));
        }
    }
}
