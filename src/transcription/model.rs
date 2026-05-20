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
    /// AssemblyAI Universal 3 Pro model (best accuracy)
    AssemblyAIUniversal3Pro,
    /// Berget KB Whisper Large model (Swedish optimized)
    BergetWhisperKBLarge,
    /// Berget NB Whisper Large model (Norwegian optimized)
    BergetWhisperNBLarge,
    /// Berget Whisper Large V3 model (general-purpose, OpenAI)
    BergetWhisperLargeV3,
    /// ElevenLabs Scribe v2 model (highest accuracy, 99 languages)
    ElevenLabsScribeV2,
    /// ElevenLabs Scribe v1 model (previous generation)
    ElevenLabsScribeV1,
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
            TranscriptionModel::AssemblyAIUniversal3Pro => TranscriptionProvider::AssemblyAI,
            TranscriptionModel::BergetWhisperKBLarge
            | TranscriptionModel::BergetWhisperNBLarge
            | TranscriptionModel::BergetWhisperLargeV3 => TranscriptionProvider::Berget,
            TranscriptionModel::ElevenLabsScribeV2 | TranscriptionModel::ElevenLabsScribeV1 => {
                TranscriptionProvider::ElevenLabs
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
            TranscriptionModel::DeepInfraWhisperLargeV3 => "deepinfra-whisper-large-v3",
            TranscriptionModel::DeepInfraWhisperBase => "deepinfra-whisper-base",
            TranscriptionModel::GroqWhisperLargeV3 => "groq-whisper-large-v3",
            TranscriptionModel::GroqWhisperLargeV3Turbo => "groq-whisper-large-v3-turbo",
            TranscriptionModel::AssemblyAIUniversal3Pro => "assemblyai-universal-3-pro",
            TranscriptionModel::BergetWhisperKBLarge => "berget-whisper-kb-large",
            TranscriptionModel::BergetWhisperNBLarge => "berget-whisper-nb-large",
            TranscriptionModel::BergetWhisperLargeV3 => "berget-whisper-large-v3",
            TranscriptionModel::ElevenLabsScribeV2 => "elevenlabs-scribe-v2",
            TranscriptionModel::ElevenLabsScribeV1 => "elevenlabs-scribe-v1",
        }
    }

    /// Returns a human-readable description of the model
    pub fn description(&self) -> &'static str {
        self.name()
    }

    /// Returns the display name for this model.
    pub fn name(&self) -> &'static str {
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
            TranscriptionModel::AssemblyAIUniversal3Pro => "Universal 3 Pro (best accuracy)",
            TranscriptionModel::BergetWhisperKBLarge => "KB Whisper Large (Swedish optimized)",
            TranscriptionModel::BergetWhisperNBLarge => "NB Whisper Large (Norwegian optimized)",
            TranscriptionModel::BergetWhisperLargeV3 => "Whisper Large V3 (general-purpose)",
            TranscriptionModel::ElevenLabsScribeV2 => "Scribe v2 (highest accuracy, 99 languages)",
            TranscriptionModel::ElevenLabsScribeV1 => "Scribe v1 (previous generation)",
        }
    }

    /// Returns a longer model description for information screens.
    pub fn detailed_description(&self) -> &'static str {
        match self {
            TranscriptionModel::Gpt4oTranscribe => "OpenAI's higher-quality speech-to-text model for transcribing audio in the source language. Supports prompting for domain terms, names, and preferred writing style.",
            TranscriptionModel::Gpt4oMiniTranscribe => "OpenAI's smaller GPT-4o transcription model, intended for faster and lower-cost speech-to-text while retaining support for prompts and plain text or JSON output.",
            TranscriptionModel::Whisper => "OpenAI's hosted Whisper model. Supports transcription in the source language, translation to English, verbose JSON, SRT/VTT output, and segment or word timestamps.",
            TranscriptionModel::DeepgramNova3 => "Deepgram's highest-performing general-purpose ASR model for batch or streaming use cases including meetings, event captioning, multi-speaker audio, noisy audio, far-field audio, and multilingual transcription.",
            TranscriptionModel::DeepgramNova2 => "Deepgram's previous-generation Nova model. Useful when a language or feature is better covered by Nova 2, including filler word identification and broad multilingual speech recognition.",
            TranscriptionModel::DeepInfraWhisperLargeV3 => "OpenAI Whisper Large V3 hosted through DeepInfra. A general-purpose multilingual Whisper model suited for high-accuracy transcription and translation workloads.",
            TranscriptionModel::DeepInfraWhisperBase => "OpenAI Whisper Base hosted through DeepInfra. A smaller Whisper model option for lighter and faster multilingual transcription workloads.",
            TranscriptionModel::GroqWhisperLargeV3 => "Groq-hosted Whisper Large V3 for error-sensitive multilingual transcription and translation. Groq documents this option as the higher-accuracy choice among its Whisper models.",
            TranscriptionModel::GroqWhisperLargeV3Turbo => "Groq-hosted Whisper Large V3 Turbo, a fine-tuned and pruned Large V3 variant designed for fast multilingual transcription with strong price/performance tradeoffs.",
            TranscriptionModel::AssemblyAIUniversal3Pro => "AssemblyAI's Universal-3 Pro speech-to-text model for high-accuracy transcription and audio understanding in cloud workflows.",
            TranscriptionModel::BergetWhisperKBLarge => "KBLab's Swedish-optimized Whisper Large model from the National Library of Sweden, trained on more than 50,000 hours of Swedish speech. KBLab reports substantially lower Swedish WER than OpenAI Whisper Large V3 across FLEURS, CommonVoice, and NST evaluations.",
            TranscriptionModel::BergetWhisperNBLarge => "NbAiLab's Norwegian NB-Whisper Large model from the National Library of Norway. It is trained on about 66,000 hours of speech and targets Norwegian ASR, including Bokmal, Nynorsk, English, and varied regional Norwegian speech.",
            TranscriptionModel::BergetWhisperLargeV3 => "General-purpose OpenAI Whisper Large V3 hosted through Berget for multilingual transcription and translation when no Swedish- or Norwegian-specialized model is preferred.",
            TranscriptionModel::ElevenLabsScribeV2 => "ElevenLabs' latest Scribe speech-to-text model for high-accuracy transcription with broad multilingual support, including support for many languages beyond English.",
            TranscriptionModel::ElevenLabsScribeV1 => "ElevenLabs' previous-generation Scribe speech-to-text model for multilingual transcription workloads.",
        }
    }

    /// Returns language coverage information for this model.
    pub fn languages(&self) -> &'static [&'static str] {
        match self {
            TranscriptionModel::Gpt4oTranscribe | TranscriptionModel::Gpt4oMiniTranscribe => {
                &["Multilingual"]
            }
            TranscriptionModel::Whisper => &["Multilingual", "translation to English"],
            TranscriptionModel::DeepgramNova3 | TranscriptionModel::DeepgramNova2 => {
                &["Multilingual", "Swedish", "Norwegian", "English"]
            }
            TranscriptionModel::BergetWhisperKBLarge => &["Swedish"],
            TranscriptionModel::BergetWhisperNBLarge => {
                &["Norwegian", "Bokmal", "Nynorsk", "English"]
            }
            TranscriptionModel::ElevenLabsScribeV2 | TranscriptionModel::ElevenLabsScribeV1 => {
                &["Multilingual", "99 languages"]
            }
            TranscriptionModel::DeepInfraWhisperLargeV3
            | TranscriptionModel::DeepInfraWhisperBase
            | TranscriptionModel::GroqWhisperLargeV3
            | TranscriptionModel::GroqWhisperLargeV3Turbo
            | TranscriptionModel::AssemblyAIUniversal3Pro
            | TranscriptionModel::BergetWhisperLargeV3 => &["Multilingual"],
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
            TranscriptionModel::AssemblyAIUniversal3Pro => "https://api.assemblyai.com/v2",
            TranscriptionModel::BergetWhisperKBLarge
            | TranscriptionModel::BergetWhisperNBLarge
            | TranscriptionModel::BergetWhisperLargeV3 => {
                "https://api.berget.ai/v1/audio/transcriptions"
            }
            TranscriptionModel::ElevenLabsScribeV2 | TranscriptionModel::ElevenLabsScribeV1 => {
                "https://api.elevenlabs.io/v1/speech-to-text"
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
            TranscriptionModel::AssemblyAIUniversal3Pro => "universal-3-pro",
            TranscriptionModel::BergetWhisperKBLarge => "KBLab/kb-whisper-large",
            TranscriptionModel::BergetWhisperNBLarge => "NbAiLab/nb-whisper-large",
            TranscriptionModel::BergetWhisperLargeV3 => "openai/whisper-large-v3",
            TranscriptionModel::ElevenLabsScribeV2 => "scribe_v2",
            TranscriptionModel::ElevenLabsScribeV1 => "scribe_v1",
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
            "assemblyai-universal-3-pro" => Some(TranscriptionModel::AssemblyAIUniversal3Pro),
            "berget-whisper-kb-large" => Some(TranscriptionModel::BergetWhisperKBLarge),
            "berget-whisper-nb-large" => Some(TranscriptionModel::BergetWhisperNBLarge),
            "berget-whisper-large-v3" => Some(TranscriptionModel::BergetWhisperLargeV3),
            "elevenlabs-scribe-v2" => Some(TranscriptionModel::ElevenLabsScribeV2),
            "elevenlabs-scribe-v1" => Some(TranscriptionModel::ElevenLabsScribeV1),
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
            TranscriptionModel::AssemblyAIUniversal3Pro,
            TranscriptionModel::BergetWhisperKBLarge,
            TranscriptionModel::BergetWhisperNBLarge,
            TranscriptionModel::BergetWhisperLargeV3,
            TranscriptionModel::ElevenLabsScribeV2,
            TranscriptionModel::ElevenLabsScribeV1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_models_have_complete_metadata() {
        for model in TranscriptionModel::all() {
            assert!(!model.id().trim().is_empty(), "missing id for {model:?}");
            assert!(
                TranscriptionModel::from_id(model.id()).is_some(),
                "id is not parseable for {model:?}"
            );
            assert!(
                TranscriptionModel::models_for_provider(&model.provider()).contains(model),
                "provider mapping omits {model:?}"
            );
            assert!(
                !model.name().trim().is_empty(),
                "missing name for {model:?}"
            );
            assert!(
                !model.detailed_description().trim().is_empty(),
                "missing detailed description for {model:?}"
            );
            assert!(
                !model.languages().is_empty(),
                "missing languages for {model:?}"
            );
            assert!(
                model
                    .languages()
                    .iter()
                    .all(|language| !language.trim().is_empty()),
                "blank language entry for {model:?}"
            );
            assert!(
                !model.endpoint().trim().is_empty(),
                "missing endpoint for {model:?}"
            );
            assert!(
                !model.api_model_name().trim().is_empty(),
                "missing API model name for {model:?}"
            );
        }
    }

    #[test]
    fn all_models_are_listed_once() {
        let mut ids = HashSet::new();
        for model in TranscriptionModel::all() {
            assert!(ids.insert(model.id()), "duplicate model id: {}", model.id());
        }
        assert_eq!(ids.len(), 15, "update this count when adding a model");
    }
}
