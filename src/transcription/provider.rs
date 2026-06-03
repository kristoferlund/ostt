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
    Berget,
    ElevenLabs,
    Whisper,
    Mistral,
    Command,
    Http,
}

impl TranscriptionProvider {
    pub fn id(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "openai",
            TranscriptionProvider::Deepgram => "deepgram",
            TranscriptionProvider::DeepInfra => "deepinfra",
            TranscriptionProvider::Groq => "groq",
            TranscriptionProvider::AssemblyAI => "assemblyai",
            TranscriptionProvider::Berget => "berget",
            TranscriptionProvider::ElevenLabs => "elevenlabs",
            TranscriptionProvider::Whisper => "whisper",
            TranscriptionProvider::Mistral => "mistral",
            TranscriptionProvider::Command => "command",
            TranscriptionProvider::Http => "http",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TranscriptionProvider::OpenAI => "OpenAI",
            TranscriptionProvider::Deepgram => "Deepgram",
            TranscriptionProvider::DeepInfra => "DeepInfra",
            TranscriptionProvider::Groq => "Groq",
            TranscriptionProvider::AssemblyAI => "AssemblyAI",
            TranscriptionProvider::Berget => "Berget",
            TranscriptionProvider::ElevenLabs => "ElevenLabs",
            TranscriptionProvider::Whisper => "Whisper (local whisper.cpp)",
            TranscriptionProvider::Mistral => "Mistral",
            TranscriptionProvider::Command => "External command",
            TranscriptionProvider::Http => "OpenAI-compatible HTTP",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "openai" => Some(TranscriptionProvider::OpenAI),
            "deepgram" => Some(TranscriptionProvider::Deepgram),
            "deepinfra" => Some(TranscriptionProvider::DeepInfra),
            "groq" => Some(TranscriptionProvider::Groq),
            "assemblyai" => Some(TranscriptionProvider::AssemblyAI),
            "berget" => Some(TranscriptionProvider::Berget),
            "elevenlabs" => Some(TranscriptionProvider::ElevenLabs),
            "whisper" => Some(TranscriptionProvider::Whisper),
            "mistral" => Some(TranscriptionProvider::Mistral),
            "command" => Some(TranscriptionProvider::Command),
            "http" => Some(TranscriptionProvider::Http),
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
            TranscriptionProvider::Berget,
            TranscriptionProvider::ElevenLabs,
            TranscriptionProvider::Whisper,
            TranscriptionProvider::Mistral,
            TranscriptionProvider::Command,
            TranscriptionProvider::Http,
        ]
    }

    pub fn supported_ids() -> Vec<&'static str> {
        Self::all().iter().map(Self::id).collect()
    }

    pub fn requires_auth(&self) -> bool {
        !matches!(
            self,
            TranscriptionProvider::Whisper
                | TranscriptionProvider::Command
                | TranscriptionProvider::Http
        )
    }
}
