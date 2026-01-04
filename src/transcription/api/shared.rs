//! Shared utilities for transcription API providers.
//!
//! Contains common response structures and helper functions used across
//! multiple provider implementations (DeepInfra, Groq, etc.).

use serde::Deserialize;

/// Response structure for Whisper-based transcription APIs that return `{"text": "..."}`.
///
/// This is the standard response format used by Whisper API implementations.
/// Used by providers: DeepInfra, Groq
#[derive(Debug, Deserialize)]
pub struct WhisperApiResponse {
    /// The transcribed text from the audio file
    pub text: String,
}

/// Configuration for providers that support multipart form data with bearer auth.
///
/// This represents the common interface for providers like DeepInfra and Groq
/// that use similar request/response patterns.
pub struct ProviderConfig {
    /// Human-readable provider name for error messages
    pub provider_name: &'static str,
    /// Part name for audio file in multipart form (e.g., "audio" or "file")
    pub file_part_name: &'static str,
    /// Whether to include model name as form parameter
    pub needs_model_param: bool,
}

impl ProviderConfig {
    /// Configuration for DeepInfra provider
    pub fn deepinfra() -> Self {
        Self {
            provider_name: "DeepInfra",
            file_part_name: "audio",
            needs_model_param: false,
        }
    }

    /// Configuration for Groq provider
    pub fn groq() -> Self {
        Self {
            provider_name: "Groq",
            file_part_name: "file",
            needs_model_param: true,
        }
    }
}
