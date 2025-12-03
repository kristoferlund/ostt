//! Transcription model definitions and metadata.
//!
//! Defines supported transcription models (e.g., Whisper) with their associated metadata,
//! providers, API endpoints, and model names.

use serde::{Deserialize, Serialize};

use super::provider::TranscriptionProvider;

/// Represents a supported transcription model
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TranscriptionModel {
    /// OpenAI GPT-4o Transcribe model (latest, best accuracy)
    Gpt4oTranscribe,
    /// OpenAI GPT-4o Mini Transcribe model (faster, lighter)
    Gpt4oMiniTranscribe,
    /// OpenAI Whisper model (legacy)
    Whisper,
    /// Deepgram Nova 3 model (latest, fastest)
    DeepgramNova3,
    /// Deepgram Nova 2 model (previous generation)
    DeepgramNova2,
    /// Parakeet TDT 0.6B v2 model (local, English-only, offline)
    ParakeetTdtV2,
    /// Parakeet TDT 0.6B v3 model (local, 25 European languages, offline)
    ParakeetTdtV3,
}

impl TranscriptionModel {
    /// Returns the provider for this model
    pub fn provider(&self) -> TranscriptionProvider {
        match self {
            TranscriptionModel::Gpt4oTranscribe
            | TranscriptionModel::Gpt4oMiniTranscribe
            | TranscriptionModel::Whisper => TranscriptionProvider::OpenAI,
            TranscriptionModel::DeepgramNova3 | TranscriptionModel::DeepgramNova2 => {
                TranscriptionProvider::Deepgram
            }
            TranscriptionModel::ParakeetTdtV2 | TranscriptionModel::ParakeetTdtV3 => {
                TranscriptionProvider::Parakeet
            }
        }
    }

    /// Returns the model identifier as a string
    pub fn id(&self) -> &'static str {
        match self {
            TranscriptionModel::Gpt4oTranscribe => "gpt-4o-transcribe",
            TranscriptionModel::Gpt4oMiniTranscribe => "gpt-4o-mini-transcribe",
            TranscriptionModel::Whisper => "whisper",
            TranscriptionModel::DeepgramNova3 => "nova-3",
            TranscriptionModel::DeepgramNova2 => "nova-2",
            TranscriptionModel::ParakeetTdtV2 => "parakeet-tdt-v2",
            TranscriptionModel::ParakeetTdtV3 => "parakeet-tdt-v3",
        }
    }

    /// Returns a human-readable description of the model
    pub fn description(&self) -> &'static str {
        match self {
            TranscriptionModel::Gpt4oTranscribe => "GPT-4o Transcribe (latest, best accuracy)",
            TranscriptionModel::Gpt4oMiniTranscribe => "GPT-4o Mini Transcribe (faster, lighter)",
            TranscriptionModel::Whisper => "Whisper (legacy)",
            TranscriptionModel::DeepgramNova3 => "Nova 3 (latest, fastest)",
            TranscriptionModel::DeepgramNova2 => "Nova 2 (previous generation)",
            TranscriptionModel::ParakeetTdtV2 => "Parakeet TDT v2 (English, offline, ~600MB)",
            TranscriptionModel::ParakeetTdtV3 => "Parakeet TDT v3 (25 languages, offline, ~2GB)",
        }
    }

    /// Returns the API endpoint for this model
    pub fn endpoint(&self) -> &'static str {
        match self {
            TranscriptionModel::Gpt4oTranscribe
            | TranscriptionModel::Gpt4oMiniTranscribe
            | TranscriptionModel::Whisper => "https://api.openai.com/v1/audio/transcriptions",
            TranscriptionModel::DeepgramNova3 | TranscriptionModel::DeepgramNova2 => {
                "https://api.deepgram.com/v1/listen"
            }
            TranscriptionModel::ParakeetTdtV2 | TranscriptionModel::ParakeetTdtV3 => {
                "local" // Local inference, no API endpoint
            }
        }
    }

    /// Returns the model name to send to the API
    pub fn api_model_name(&self) -> &'static str {
        match self {
            TranscriptionModel::Gpt4oTranscribe => "gpt-4o-transcribe",
            TranscriptionModel::Gpt4oMiniTranscribe => "gpt-4o-mini-transcribe",
            TranscriptionModel::Whisper => "whisper-1",
            TranscriptionModel::DeepgramNova3 => "nova-3",
            TranscriptionModel::DeepgramNova2 => "nova-2",
            TranscriptionModel::ParakeetTdtV2 => "parakeet-tdt-0.6b-v2",
            TranscriptionModel::ParakeetTdtV3 => "parakeet-tdt-0.6b-v3",
        }
    }

    /// Parses a model ID string into a TranscriptionModel
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "gpt-4o-transcribe" => Some(TranscriptionModel::Gpt4oTranscribe),
            "gpt-4o-mini-transcribe" => Some(TranscriptionModel::Gpt4oMiniTranscribe),
            "whisper" => Some(TranscriptionModel::Whisper),
            "nova-3" => Some(TranscriptionModel::DeepgramNova3),
            "nova-2" => Some(TranscriptionModel::DeepgramNova2),
            "parakeet-tdt-v2" => Some(TranscriptionModel::ParakeetTdtV2),
            "parakeet-tdt-v3" => Some(TranscriptionModel::ParakeetTdtV3),
            _ => None,
        }
    }

    /// Returns all available models
    pub fn all() -> &'static [Self] {
        &[
            TranscriptionModel::Gpt4oTranscribe,
            TranscriptionModel::Gpt4oMiniTranscribe,
            TranscriptionModel::Whisper,
            TranscriptionModel::DeepgramNova3,
            TranscriptionModel::DeepgramNova2,
            TranscriptionModel::ParakeetTdtV2,
            TranscriptionModel::ParakeetTdtV3,
        ]
    }

    /// Returns all available model IDs
    pub fn available_ids() -> Vec<&'static str> {
        Self::all().iter().map(|m| m.id()).collect()
    }

    /// Returns all models for a given provider
    pub fn models_for_provider(provider: &TranscriptionProvider) -> Vec<TranscriptionModel> {
        Self::all()
            .iter()
            .filter(|m| m.provider() == *provider)
            .cloned()
            .collect()
    }
}
