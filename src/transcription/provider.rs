//! Transcription provider definitions and methods.
//!
//! Defines supported transcription service providers (e.g., OpenAI).
//! Each provider has its own API endpoint and authentication method.

use serde::{Deserialize, Serialize};

/// Represents a supported transcription provider
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TranscriptionProvider {
    OpenAI,
    Deepgram,
    /// Local Parakeet model (offline, no API required)
    Parakeet,
}

impl TranscriptionProvider {
    pub fn id(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "openai",
            TranscriptionProvider::Deepgram => "deepgram",
            TranscriptionProvider::Parakeet => "parakeet",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "OpenAI",
            TranscriptionProvider::Deepgram => "Deepgram",
            TranscriptionProvider::Parakeet => "Parakeet (Local)",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "openai" => Some(TranscriptionProvider::OpenAI),
            "deepgram" => Some(TranscriptionProvider::Deepgram),
            "parakeet" => Some(TranscriptionProvider::Parakeet),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            TranscriptionProvider::OpenAI,
            TranscriptionProvider::Deepgram,
            TranscriptionProvider::Parakeet,
        ]
    }
}
