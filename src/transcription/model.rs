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
    /// DeepInfra Whisper Large V3 model
    DeepInfraWhisperLargeV3,
    /// DeepInfra Whisper Base model
    DeepInfraWhisperBase,
    /// Groq Whisper Large V3 model
    GroqWhisperLargeV3,
    /// Groq Whisper Large V3 Turbo model (faster)
    GroqWhisperLargeV3Turbo,
    /// Berget Whisper KB Large model (Swedish optimized)
    BergetWhisperKBLarge,
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
            TranscriptionModel::DeepInfraWhisperLargeV3
            | TranscriptionModel::DeepInfraWhisperBase => TranscriptionProvider::DeepInfra,
            TranscriptionModel::GroqWhisperLargeV3
            | TranscriptionModel::GroqWhisperLargeV3Turbo => TranscriptionProvider::Groq,
            TranscriptionModel::BergetWhisperKBLarge => TranscriptionProvider::Berget,
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
            TranscriptionModel::DeepInfraWhisperLargeV3 => "deepinfra-whisper-large-v3",
            TranscriptionModel::DeepInfraWhisperBase => "deepinfra-whisper-base",
            TranscriptionModel::GroqWhisperLargeV3 => "groq-whisper-large-v3",
            TranscriptionModel::GroqWhisperLargeV3Turbo => "groq-whisper-large-v3-turbo",
            TranscriptionModel::BergetWhisperKBLarge => "berget-whisper-kb-large",
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
            TranscriptionModel::DeepInfraWhisperLargeV3 => "Whisper Large V3 (best accuracy)",
            TranscriptionModel::DeepInfraWhisperBase => "Whisper Base (fast, lightweight)",
            TranscriptionModel::GroqWhisperLargeV3 => "Whisper Large V3 (high accuracy)",
            TranscriptionModel::GroqWhisperLargeV3Turbo => "Whisper Large V3 Turbo (fastest)",
            TranscriptionModel::BergetWhisperKBLarge => "KB Whisper Large (Swedish optimized)",
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
            TranscriptionModel::DeepInfraWhisperLargeV3
            | TranscriptionModel::DeepInfraWhisperBase => "https://api.deepinfra.com/v1/inference",
            TranscriptionModel::GroqWhisperLargeV3
            | TranscriptionModel::GroqWhisperLargeV3Turbo => {
                "https://api.groq.com/openai/v1/audio/transcriptions"
            }
            TranscriptionModel::BergetWhisperKBLarge => {
                "https://api.berget.ai/v1/audio/transcriptions"
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
            TranscriptionModel::DeepInfraWhisperLargeV3 => "openai/whisper-large-v3",
            TranscriptionModel::DeepInfraWhisperBase => "openai/whisper-base",
            TranscriptionModel::GroqWhisperLargeV3 => "whisper-large-v3",
            TranscriptionModel::GroqWhisperLargeV3Turbo => "whisper-large-v3-turbo",
            TranscriptionModel::BergetWhisperKBLarge => "KBLab/kb-whisper-large",
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
            "deepinfra-whisper-large-v3" => Some(TranscriptionModel::DeepInfraWhisperLargeV3),
            "deepinfra-whisper-base" => Some(TranscriptionModel::DeepInfraWhisperBase),
            "groq-whisper-large-v3" => Some(TranscriptionModel::GroqWhisperLargeV3),
            "groq-whisper-large-v3-turbo" => Some(TranscriptionModel::GroqWhisperLargeV3Turbo),
            "berget-whisper-kb-large" => Some(TranscriptionModel::BergetWhisperKBLarge),
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
            TranscriptionModel::DeepInfraWhisperLargeV3,
            TranscriptionModel::DeepInfraWhisperBase,
            TranscriptionModel::GroqWhisperLargeV3,
            TranscriptionModel::GroqWhisperLargeV3Turbo,
            TranscriptionModel::BergetWhisperKBLarge,
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
