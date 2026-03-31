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
    DeepInfra,
    Groq,
    AssemblyAI,
}

impl TranscriptionProvider {
    pub fn id(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "openai",
            TranscriptionProvider::Deepgram => "deepgram",
            TranscriptionProvider::DeepInfra => "deepinfra",
            TranscriptionProvider::Groq => "groq",
            TranscriptionProvider::AssemblyAI => "assemblyai",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "OpenAI",
            TranscriptionProvider::Deepgram => "Deepgram",
            TranscriptionProvider::DeepInfra => "DeepInfra",
            TranscriptionProvider::Groq => "Groq",
            TranscriptionProvider::AssemblyAI => "AssemblyAI",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "openai" => Some(TranscriptionProvider::OpenAI),
            "deepgram" => Some(TranscriptionProvider::Deepgram),
            "deepinfra" => Some(TranscriptionProvider::DeepInfra),
            "groq" => Some(TranscriptionProvider::Groq),
            "assemblyai" => Some(TranscriptionProvider::AssemblyAI),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            TranscriptionProvider::OpenAI,
            TranscriptionProvider::Deepgram,
            TranscriptionProvider::DeepInfra,
            TranscriptionProvider::Groq,
            TranscriptionProvider::AssemblyAI,
        ]
    }
}
