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
mod local;
mod openai;

use serde::Deserialize;
use std::path::Path;

use super::model::TranscriptionModel;
use super::provider::TranscriptionProvider;
use crate::config::file::{LocalTranscriptionConfig, ProvidersConfig};

/// Configuration for transcription requests
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// The provider to use
    pub provider: TranscriptionProvider,
    /// The selected model ID, including data-driven local model IDs
    pub model_id: String,
    /// The model to use
    pub model: TranscriptionModel,
    /// The API key for authentication
    pub api_key: String,
    /// Keywords to improve transcription accuracy
    pub keywords: Vec<String>,
    /// Provider-specific configurations
    pub providers: ProvidersConfig,
}

impl TranscriptionConfig {
    /// Creates a new transcription configuration
    pub fn new(
        model: TranscriptionModel,
        api_key: String,
        keywords: Vec<String>,
        providers: ProvidersConfig,
    ) -> Self {
        let provider = model.provider();
        let model_id = model.id().to_string();

        Self {
            provider,
            model_id,
            model,
            api_key,
            keywords,
            providers,
        }
    }

    /// Creates a local transcription configuration with a registry-backed model ID.
    pub fn new_local(model_id: String, keywords: Vec<String>, providers: ProvidersConfig) -> Self {
        Self {
            provider: TranscriptionProvider::Local,
            model_id,
            model: TranscriptionModel::Whisper,
            api_key: String::new(),
            keywords,
            providers,
        }
    }

    /// Returns the local-specific config only for local transcription requests.
    pub fn local_config(&self) -> Option<&LocalTranscriptionConfig> {
        if self.provider == TranscriptionProvider::Local {
            Some(&self.providers.local)
        } else {
            None
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
        TranscriptionProvider::Local => local::transcribe(config, audio_path).await,
    }?;

    Ok(result)
}
