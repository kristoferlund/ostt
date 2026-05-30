//! Transcription model registry and metadata.

use super::provider::TranscriptionProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub provider_id: &'static str,
    pub model_id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub languages: &'static [&'static str],
}

impl ModelSpec {
    pub fn full_id(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}

const MODELS: &[ModelSpec] = &[
    ModelSpec {
        provider_id: "openai",
        model_id: "gpt-4o-transcribe",
        display_name: "GPT-4o Transcribe (latest, best accuracy)",
        description: "OpenAI's higher-quality speech-to-text model for transcribing audio in the source language. Supports prompting for domain terms, names, and preferred writing style.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "openai",
        model_id: "gpt-4o-mini-transcribe",
        display_name: "GPT-4o Mini Transcribe (faster, lighter)",
        description: "OpenAI's smaller GPT-4o transcription model, intended for faster and lower-cost speech-to-text while retaining support for prompts and plain text or JSON output.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "openai",
        model_id: "whisper-1",
        display_name: "Whisper (legacy)",
        description: "OpenAI's hosted Whisper model. Supports transcription in the source language, translation to English, verbose JSON, SRT/VTT output, and segment or word timestamps.",
        languages: &["Multilingual", "translation to English"],
    },
    ModelSpec {
        provider_id: "deepgram",
        model_id: "nova-3",
        display_name: "Nova 3 (latest, fastest)",
        description: "Deepgram's highest-performing general-purpose ASR model for batch or streaming use cases including meetings, event captioning, multi-speaker audio, noisy audio, far-field audio, and multilingual transcription.",
        languages: &["Multilingual", "Swedish", "Norwegian", "English"],
    },
    ModelSpec {
        provider_id: "deepgram",
        model_id: "nova-2",
        display_name: "Nova 2 (previous generation)",
        description: "Deepgram's previous-generation Nova model. Useful when a language or feature is better covered by Nova 2, including filler word identification and broad multilingual speech recognition.",
        languages: &["Multilingual", "Swedish", "Norwegian", "English"],
    },
    ModelSpec {
        provider_id: "groq",
        model_id: "whisper-large-v3",
        display_name: "Whisper Large V3 (high accuracy)",
        description: "Groq-hosted Whisper Large V3 for error-sensitive multilingual transcription and translation. Groq documents this option as the higher-accuracy choice among its Whisper models.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "groq",
        model_id: "whisper-large-v3-turbo",
        display_name: "Whisper Large V3 Turbo (fastest)",
        description: "Groq-hosted Whisper Large V3 Turbo, a fine-tuned and pruned Large V3 variant designed for fast multilingual transcription with strong price/performance tradeoffs.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "openai/whisper-large-v3",
        display_name: "Whisper Large V3 (best accuracy)",
        description: "OpenAI Whisper Large V3 hosted through DeepInfra. A general-purpose multilingual Whisper model suited for high-accuracy transcription and translation workloads.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "deepinfra",
        model_id: "openai/whisper-base",
        display_name: "Whisper Base (fast, lightweight)",
        description: "OpenAI Whisper Base hosted through DeepInfra. A smaller Whisper model option for lighter and faster multilingual transcription workloads.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "assemblyai",
        model_id: "universal-3-pro",
        display_name: "Universal 3 Pro (best accuracy)",
        description: "AssemblyAI's Universal-3 Pro speech-to-text model for high-accuracy transcription and audio understanding in cloud workflows.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "berget",
        model_id: "KBLab/kb-whisper-large",
        display_name: "KB Whisper Large (Swedish optimized)",
        description: "KBLab's Swedish-optimized Whisper Large model from the National Library of Sweden, trained on more than 50,000 hours of Swedish speech. KBLab reports substantially lower Swedish WER than OpenAI Whisper Large V3 across FLEURS, CommonVoice, and NST evaluations.",
        languages: &["Swedish"],
    },
    ModelSpec {
        provider_id: "berget",
        model_id: "NbAiLab/nb-whisper-large",
        display_name: "NB Whisper Large (Norwegian optimized)",
        description: "NbAiLab's Norwegian NB-Whisper Large model from the National Library of Norway. It is trained on about 66,000 hours of speech and targets Norwegian ASR, including Bokmal, Nynorsk, English, and varied regional Norwegian speech.",
        languages: &["Norwegian", "Bokmal", "Nynorsk", "English"],
    },
    ModelSpec {
        provider_id: "berget",
        model_id: "openai/whisper-large-v3",
        display_name: "Whisper Large V3 (general-purpose)",
        description: "General-purpose OpenAI Whisper Large V3 hosted through Berget for multilingual transcription and translation when no Swedish- or Norwegian-specialized model is preferred.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v2",
        display_name: "Scribe v2 (highest accuracy, 99 languages)",
        description: "ElevenLabs' latest Scribe speech-to-text model for high-accuracy transcription with broad multilingual support, including support for many languages beyond English.",
        languages: &["Multilingual", "99 languages"],
    },
    ModelSpec {
        provider_id: "elevenlabs",
        model_id: "scribe_v1",
        display_name: "Scribe v1 (previous generation)",
        description: "ElevenLabs' previous-generation Scribe speech-to-text model for multilingual transcription workloads.",
        languages: &["Multilingual", "99 languages"],
    },
    ModelSpec {
        provider_id: "mistral",
        model_id: "voxtral-mini-latest",
        display_name: "Voxtral Mini Transcribe (fast, efficient, 13 languages)",
        description: "Mistral's Voxtral Mini transcription model for fast, efficient multilingual speech-to-text via the Mistral API.",
        languages: &["Multilingual"],
    },
    ModelSpec {
        provider_id: "mistral",
        model_id: "voxtral-mini-2602",
        display_name: "Voxtral Mini 2602 (newer version, improved accuracy)",
        description: "Pinned Voxtral Mini 2602 transcription model for stable Mistral speech-to-text behavior.",
        languages: &["Multilingual"],
    },
];

pub fn all_models() -> &'static [ModelSpec] {
    MODELS
}

pub fn find_model(provider_id: &str, model_id: &str) -> Option<&'static ModelSpec> {
    MODELS
        .iter()
        .find(|model| model.provider_id == provider_id && model.model_id == model_id)
}

pub fn models_for_provider(provider: &TranscriptionProvider) -> Vec<&'static ModelSpec> {
    MODELS
        .iter()
        .filter(|model| model.provider_id == provider.id())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_allows_same_model_id_under_multiple_providers() {
        assert!(find_model("deepinfra", "openai/whisper-large-v3").is_some());
        assert!(find_model("berget", "openai/whisper-large-v3").is_some());
    }

    #[test]
    fn all_cloud_models_have_valid_provider_and_metadata() {
        for model in all_models() {
            assert!(TranscriptionProvider::from_id(model.provider_id).is_some());
            assert_ne!(model.provider_id, "local");
            assert!(!model.model_id.trim().is_empty());
            assert!(!model.display_name.trim().is_empty());
            assert!(!model.description.trim().is_empty());
            assert!(!model.languages.is_empty());
            assert!(model
                .languages
                .iter()
                .all(|language| !language.trim().is_empty()));
        }
    }

    #[test]
    fn full_ids_are_unique() {
        let mut ids = HashSet::new();
        for model in all_models() {
            assert!(
                ids.insert(model.full_id()),
                "duplicate full id: {}",
                model.full_id()
            );
        }
    }

    #[test]
    fn old_ostt_invented_ids_are_not_accepted() {
        assert!(find_model("deepinfra", "deepinfra-whisper-large-v3").is_none());
        assert!(find_model("groq", "groq-whisper-large-v3").is_none());
        assert!(find_model("berget", "berget-whisper-large-v3").is_none());
        assert!(find_model("elevenlabs", "elevenlabs-scribe-v2").is_none());
    }
}
