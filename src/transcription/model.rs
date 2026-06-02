//! Transcription model registry and metadata.

use super::provider::TranscriptionProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub provider_id: &'static str,
    pub model_id: &'static str,
    pub endpoint: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub languages: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelOptionKind {
    Bool,
    BoolOrString,
    BoolOrStringList,
    Integer,
    Number,
    String,
    StringOrStringList,
    StringList,
}

impl ModelOptionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bool => "boolean",
            Self::BoolOrString => "boolean or string",
            Self::BoolOrStringList => "boolean or string list",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::String => "string",
            Self::StringOrStringList => "string or string list",
            Self::StringList => "string list",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelOptionSpec {
    pub name: &'static str,
    pub kind: ModelOptionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelOptionSchema {
    options: &'static [ModelOptionSpec],
}

impl ModelOptionSchema {
    pub(crate) const fn new(options: &'static [ModelOptionSpec]) -> Self {
        Self { options }
    }

    pub fn option(&self, name: &str) -> Option<&'static ModelOptionSpec> {
        self.options.iter().find(|option| option.name == name)
    }

    pub fn option_names(&self) -> Vec<&'static str> {
        self.options.iter().map(|option| option.name).collect()
    }

    pub fn options(&self) -> &'static [ModelOptionSpec] {
        self.options
    }
}

impl ModelSpec {
    pub fn full_id(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}

pub fn all_models() -> Vec<&'static ModelSpec> {
    super::api::all_models()
}

pub fn find_model(provider_id: &str, model_id: &str) -> Option<&'static ModelSpec> {
    all_models()
        .into_iter()
        .find(|model| model.provider_id == provider_id && model.model_id == model_id)
}

pub fn models_for_provider(provider: &TranscriptionProvider) -> Vec<&'static ModelSpec> {
    all_models()
        .into_iter()
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
