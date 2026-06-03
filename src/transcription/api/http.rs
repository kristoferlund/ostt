use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

use indexmap::IndexMap;

use super::{TranscriptionConfig, TranscriptionResponse};
use crate::config::ModelOptionValue;
use crate::transcription::model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec};

const HTTP_OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "model",
        kind: ModelOptionKind::String,
    },
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
];

pub(super) fn option_schema(_model_id: &str) -> Option<ModelOptionSchema> {
    Some(ModelOptionSchema::new(HTTP_OPTIONS))
}

pub(super) fn validate_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    super::validate_number_range(full_model_id, options, "temperature", 0.0..=1.0)?;
    if let Some(value) = options.get("response_format") {
        super::validate_string_value(full_model_id, "response_format", value, &["json"])?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct HttpTranscriptionResponse {
    text: String,
}

pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let model = config.option_string("model").ok_or_else(|| {
        anyhow::anyhow!(
            "HTTP profile '{}' is missing required param 'model'. Configure [http.{}.params].model or pass --param model=...",
            config.model_id,
            config.model_id
        )
    })?;

    let audio_data = std::fs::read(audio_path)
        .map_err(|err| anyhow::anyhow!("failed to read audio file: {err}"))?;
    let file_name = audio_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_part = reqwest::multipart::Part::bytes(audio_data)
        .file_name(file_name)
        .mime_str("application/octet-stream")
        .map_err(|err| anyhow::anyhow!("failed to create file part for upload: {err}"))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());
    let mut debug_params = vec![format!("model={model}")];

    if let Some(language) = config.option_string("language") {
        form = form.text("language", language.to_string());
        debug_params.push(format!("language={language}"));
    }
    if let Some(prompt) = config.option_string("prompt") {
        form = form.text("prompt", prompt.to_string());
        debug_params.push(format!("prompt={prompt}"));
    }
    if let Some(temperature) = config.option_number("temperature") {
        form = form.text("temperature", temperature.to_string());
        debug_params.push(format!("temperature={temperature}"));
    }
    if let Some(response_format) = config.option_string("response_format") {
        form = form.text("response_format", response_format.to_string());
        debug_params.push(format!("response_format={response_format}"));
    }

    let mut client = reqwest::Client::builder();
    if let Some(timeout_secs) = config.timeout_secs {
        client = client.timeout(Duration::from_secs(timeout_secs));
    }
    let client = client.build()?;

    tracing::debug!(
        "OpenAI-compatible HTTP transcription:\n  URL: {}\n  Body parameters: {}",
        config.endpoint,
        debug_params.join("\n    ")
    );

    let mut request = client.post(&config.endpoint).multipart(form);
    if config.api_key_configured {
        request = request.bearer_auth(&config.api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|err| anyhow::anyhow!("HTTP transcription request failed: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("HTTP transcription failed with status {status}: {body}");
    }

    let parsed: HttpTranscriptionResponse = response
        .json()
        .await
        .map_err(|err| anyhow::anyhow!("failed to parse HTTP transcription response: {err}"))?;
    Ok(TranscriptionResponse { text: parsed.text }.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_response_format_for_json_only_v1() {
        let mut options = IndexMap::new();
        options.insert(
            "response_format".to_string(),
            ModelOptionValue::String("text".to_string()),
        );

        let err = validate_options("http/speaches", &options)
            .unwrap_err()
            .to_string();

        assert!(err.contains("response_format"));
        assert!(err.contains("json"));
    }
}
