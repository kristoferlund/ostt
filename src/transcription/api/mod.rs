//! Transcription API client with provider-specific implementations.
//!
//! This module provides a trait-based system for handling multiple transcription providers
//! (OpenAI, Deepgram, etc.) with their respective APIs. Each provider implements the
//! `TranscriptionProvider` trait to handle authentication and API communication.

mod assemblyai;
mod berget;
mod deepgram;
mod deepinfra;
mod elevenlabs;
mod groq;
pub(crate) mod local;
mod mistral;
mod openai;

use serde::Deserialize;
use std::path::Path;

use super::model::{ModelOptionSchema, ModelSpec};
use super::provider::TranscriptionProvider;
use crate::config::file::{LocalTranscriptionConfig, ModelOptionValue};
use indexmap::IndexMap;
use std::ops::RangeInclusive;

/// Configuration for transcription requests
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// The provider to use
    pub provider: TranscriptionProvider,
    /// The selected model ID, including data-driven local model IDs
    pub model_id: String,
    /// Base API endpoint for the selected model
    pub endpoint: &'static str,
    /// The API key for authentication
    pub api_key: String,
    /// Keywords to improve transcription accuracy
    pub keywords: Vec<String>,
    /// Validated request params for the selected model
    pub params: IndexMap<String, ModelOptionValue>,
}

impl TranscriptionConfig {
    /// Creates a new transcription configuration
    pub fn new_cloud(
        provider: TranscriptionProvider,
        model_id: String,
        endpoint: &'static str,
        api_key: String,
        keywords: Vec<String>,
        params: IndexMap<String, ModelOptionValue>,
    ) -> Self {
        Self {
            provider,
            model_id,
            endpoint,
            api_key,
            keywords,
            params,
        }
    }

    /// Creates a local transcription configuration with a registry-backed model ID.
    pub fn new_local(
        model_id: String,
        keywords: Vec<String>,
        params: IndexMap<String, ModelOptionValue>,
    ) -> Self {
        Self {
            provider: TranscriptionProvider::Whisper,
            model_id,
            endpoint: "",
            api_key: String::new(),
            keywords,
            params,
        }
    }

    /// Returns the built-in whisper defaults only for whisper transcription requests.
    pub fn local_config(&self) -> Option<&LocalTranscriptionConfig> {
        if self.provider == TranscriptionProvider::Whisper {
            None
        } else {
            None
        }
    }

    pub fn option_bool(&self, name: &str) -> Option<bool> {
        match self.params.get(name) {
            Some(ModelOptionValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn option_string(&self, name: &str) -> Option<&str> {
        match self.params.get(name) {
            Some(ModelOptionValue::String(value)) => Some(value),
            _ => None,
        }
    }

    pub fn option_number(&self, name: &str) -> Option<f64> {
        match self.params.get(name) {
            Some(ModelOptionValue::Number(value)) => Some(*value),
            Some(ModelOptionValue::Integer(value)) => Some(*value as f64),
            _ => None,
        }
    }

    pub fn option_integer(&self, name: &str) -> Option<i64> {
        match self.params.get(name) {
            Some(ModelOptionValue::Integer(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn option_string_list(&self, name: &str) -> Option<&[String]> {
        match self.params.get(name) {
            Some(ModelOptionValue::StringList(value)) => Some(value),
            _ => None,
        }
    }
}

/// Response from transcription APIs (unified across providers).
#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionResponse {
    /// The transcribed text from the audio file
    pub text: String,
}

/// Transcribes an audio file using the configured transcription model.
///
/// This function routes the request to the appropriate provider-specific implementation
/// based on the configured model. The caller doesn't need to know which provider is being used.
///
/// # Errors
/// - If the audio file cannot be read from disk
/// - If the API request fails due to network issues (connection, timeout)
/// - If the API returns an HTTP error (401 for invalid key, 429 for rate limit, etc.)
/// - If the API response cannot be parsed
pub async fn transcribe(config: &TranscriptionConfig, audio_path: &Path) -> anyhow::Result<String> {
    tracing::info!(
        "Transcribing with {} ({})",
        config.provider.name(),
        config.model_id
    );

    let result = match config.provider {
        TranscriptionProvider::OpenAI => openai::transcribe(config, audio_path).await,
        TranscriptionProvider::Deepgram => deepgram::transcribe(config, audio_path).await,
        TranscriptionProvider::DeepInfra => deepinfra::transcribe(config, audio_path).await,
        TranscriptionProvider::Groq => groq::transcribe(config, audio_path).await,
        TranscriptionProvider::AssemblyAI => assemblyai::transcribe(config, audio_path).await,
        TranscriptionProvider::Berget => berget::transcribe(config, audio_path).await,
        TranscriptionProvider::ElevenLabs => elevenlabs::transcribe(config, audio_path).await,
        TranscriptionProvider::Whisper => local::transcribe(config, audio_path).await,
        TranscriptionProvider::Mistral => mistral::transcribe(config, audio_path).await,
    }?;

    Ok(result)
}

pub(crate) fn option_schema(provider_id: &str, model_id: &str) -> Option<ModelOptionSchema> {
    match provider_id {
        "openai" => openai::option_schema(model_id),
        "deepgram" => deepgram::option_schema(model_id),
        "deepinfra" => deepinfra::option_schema(model_id),
        "groq" => groq::option_schema(model_id),
        "assemblyai" => assemblyai::option_schema(model_id),
        "berget" => berget::option_schema(model_id),
        "elevenlabs" => elevenlabs::option_schema(model_id),
        "mistral" => mistral::option_schema(model_id),
        "whisper" => local::option_schema(model_id),
        _ => None,
    }
}

pub(crate) fn validate_params(
    provider_id: &str,
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    match provider_id {
        "openai" => openai::validate_options(full_model_id, options),
        "groq" => groq::validate_options(full_model_id, options),
        "deepinfra" => deepinfra::validate_options(full_model_id, options),
        "assemblyai" => assemblyai::validate_options(full_model_id, options),
        "berget" => berget::validate_options(full_model_id, options),
        "elevenlabs" => elevenlabs::validate_options(full_model_id, options),
        "mistral" => mistral::validate_options(full_model_id, options),
        "whisper" => local::validate_options(full_model_id, options),
        _ => Ok(()),
    }
}

pub(super) fn validate_string_value(
    full_model_id: &str,
    option_name: &str,
    value: &ModelOptionValue,
    allowed: &[&str],
) -> anyhow::Result<()> {
    let ModelOptionValue::String(value) = value else {
        return Ok(());
    };

    if !allowed.contains(&value.as_str()) {
        anyhow::bail!(
            "Invalid value for param '{}' in '{}'. Expected one of: {}.",
            option_name,
            full_model_id,
            allowed.join(", ")
        );
    }

    Ok(())
}

pub(super) fn validate_string_list_values(
    full_model_id: &str,
    option_name: &str,
    value: &ModelOptionValue,
    allowed: &[&str],
) -> anyhow::Result<()> {
    let ModelOptionValue::StringList(values) = value else {
        return Ok(());
    };

    for value in values {
        if !allowed.contains(&value.as_str()) {
            anyhow::bail!(
                "Invalid value for param '{}' in '{}'. Expected one of: {}.",
                option_name,
                full_model_id,
                allowed.join(", ")
            );
        }
    }

    Ok(())
}

pub(super) fn validate_integer_range(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
    option_name: &str,
    range: RangeInclusive<i64>,
) -> anyhow::Result<()> {
    if let Some(ModelOptionValue::Integer(value)) = options.get(option_name) {
        if !range.contains(value) {
            anyhow::bail!(
                "Invalid value for param '{}' in '{}'. Expected {}-{}.",
                option_name,
                full_model_id,
                range.start(),
                range.end()
            );
        }
    }

    Ok(())
}

pub(super) fn validate_number_range(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
    option_name: &str,
    range: RangeInclusive<f64>,
) -> anyhow::Result<()> {
    let value = match options.get(option_name) {
        Some(ModelOptionValue::Number(value)) => *value,
        Some(ModelOptionValue::Integer(value)) => *value as f64,
        _ => return Ok(()),
    };

    if !range.contains(&value) {
        anyhow::bail!(
            "Invalid value for param '{}' in '{}'. Expected {}-{}.",
            option_name,
            full_model_id,
            range.start(),
            range.end()
        );
    }

    Ok(())
}

pub(super) fn validate_number_min(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
    option_name: &str,
    min: f64,
) -> anyhow::Result<()> {
    let value = match options.get(option_name) {
        Some(ModelOptionValue::Number(value)) => *value,
        Some(ModelOptionValue::Integer(value)) => *value as f64,
        _ => return Ok(()),
    };

    if value < min {
        anyhow::bail!(
            "Invalid value for param '{}' in '{}'. Expected >= {}.",
            option_name,
            full_model_id,
            min
        );
    }

    Ok(())
}

pub(crate) fn all_models() -> Vec<&'static ModelSpec> {
    [
        openai::MODELS,
        deepgram::MODELS,
        groq::MODELS,
        deepinfra::MODELS,
        assemblyai::MODELS,
        berget::MODELS,
        elevenlabs::MODELS,
        mistral::MODELS,
    ]
    .into_iter()
    .flatten()
    .collect()
}
