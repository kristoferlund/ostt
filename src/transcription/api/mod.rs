//! Transcription API client with provider-specific implementations.
//!
//! This module provides a trait-based system for handling multiple transcription providers
//! (OpenAI, Deepgram, etc.) with their respective APIs. Each provider implements the
//! `TranscriptionProvider` trait to handle authentication and API communication.

mod openai;
mod deepgram;
mod deepinfra;
mod groq;
mod berget;

use serde::Deserialize;
use std::path::Path;

use super::model::TranscriptionModel;
use super::provider::TranscriptionProvider;
use crate::config::file::ProvidersConfig;

/// Configuration for transcription requests
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
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
        Self {
            model,
            api_key,
            keywords,
            providers,
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
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    tracing::info!(
        "Transcribing with {} ({})",
        config.model.provider().name(),
        config.model.id()
    );

    let result = match config.model.provider() {
        TranscriptionProvider::OpenAI => {
            openai::transcribe(config, audio_path).await
        }
        TranscriptionProvider::Deepgram => {
            deepgram::transcribe(config, audio_path).await
        }
        TranscriptionProvider::DeepInfra => {
            deepinfra::transcribe(config, audio_path).await
        }
        TranscriptionProvider::Groq => {
            groq::transcribe(config, audio_path).await
        }
        TranscriptionProvider::Berget => {
            berget::transcribe(config, audio_path).await
        }
    }?;

    Ok(result)
}
