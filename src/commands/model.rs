use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    load_state, model_destination, LocalModelState, RegistryEntry,
};
use crate::transcription::{TranscriptionModel, TranscriptionProvider};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ModelSelectionEntryKind {
    Cloud { model: TranscriptionModel },
    Local { entry: RegistryEntry, is_downloaded: bool },
    LocalManagement,
}

#[derive(Debug, Clone)]
pub struct ModelSelectionEntry {
    pub provider_id: String,
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub kind: ModelSelectionEntryKind,
}

#[derive(Debug, Clone)]
pub struct ModelSelectionSection {
    pub provider_id: String,
    pub title: String,
    pub entries: Vec<ModelSelectionEntry>,
}

pub async fn handle_model() -> anyhow::Result<()> {
    anyhow::bail!("`ostt model` UI is not implemented yet")
}

pub fn build_model_sections(
    authorized_provider_ids: &[String],
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<ModelSelectionSection> {
    let mut sections = build_cloud_sections(authorized_provider_ids, selected_model);
    sections.push(build_local_section(local_state, registry, selected_model));
    sections
}

pub fn build_cloud_sections(
    authorized_provider_ids: &[String],
    selected_model: Option<&SelectedModel>,
) -> Vec<ModelSelectionSection> {
    let authorized: HashSet<&str> = authorized_provider_ids.iter().map(String::as_str).collect();

    TranscriptionProvider::all()
        .iter()
        .filter(|provider| **provider != TranscriptionProvider::Local)
        .filter(|provider| authorized.contains(provider.id()))
        .filter_map(|provider| {
            let entries: Vec<ModelSelectionEntry> =
                TranscriptionModel::models_for_provider(provider)
                    .into_iter()
                    .map(|model| {
                        let model_id = model.id().to_string();
                        ModelSelectionEntry {
                            provider_id: provider.id().to_string(),
                            model_id: model_id.clone(),
                            name: model_id,
                            description: model.description().to_string(),
                            is_active: selected_model
                                .map(|selected| {
                                    selected.provider_id == provider.id() && selected.model_id == model.id()
                                })
                                .unwrap_or(false),
                            kind: ModelSelectionEntryKind::Cloud { model },
                        }
                    })
                    .collect();

            (!entries.is_empty()).then(|| ModelSelectionSection {
                provider_id: provider.id().to_string(),
                title: provider.name().to_string(),
                entries,
            })
        })
        .collect()
}

pub fn build_local_section(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> ModelSelectionSection {
    let mut seen = HashSet::new();
    let mut entries: Vec<ModelSelectionEntry> = registry
        .iter()
        .chain(local_state.custom_models.iter())
        .filter(|entry| seen.insert(entry.id.as_str()))
        .map(|entry| {
            let is_downloaded = model_destination(entry).exists();
            ModelSelectionEntry {
                provider_id: "local".to_string(),
                model_id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                is_active: selected_model
                    .map(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
                    .unwrap_or(false),
                kind: ModelSelectionEntryKind::Local {
                    entry: entry.clone(),
                    is_downloaded,
                },
            }
        })
        .collect();

    entries.push(ModelSelectionEntry {
        provider_id: "local".to_string(),
        model_id: "__manage_local_models__".to_string(),
        name: "Manage local models...".to_string(),
        description: "Download, inspect, or remove local models".to_string(),
        is_active: false,
        kind: ModelSelectionEntryKind::LocalManagement,
    });

    ModelSelectionSection {
        provider_id: "local".to_string(),
        title: "Local".to_string(),
        entries,
    }
}

pub fn load_local_selection_state() -> anyhow::Result<LocalModelState> {
    Ok(load_state())
}

pub fn no_selected_model_error() -> anyhow::Error {
    anyhow::anyhow!(
        "No transcription model selected.\n\nRun `ostt model` to choose an online or local transcription model.\nRun `ostt auth login` first to add credentials for cloud providers."
    )
}

pub fn missing_cloud_credentials_error(provider: &TranscriptionProvider) -> anyhow::Error {
    anyhow::anyhow!(
        "{} is selected, but no {} API key is configured.\n\nRun `ostt auth login` and choose {}.",
        provider.name(),
        provider.name(),
        provider.name()
    )
}

pub fn missing_local_model_error(model_id: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Local model `{model_id}` is selected but not downloaded.\n\nRun `ostt model` to download or select a local model."
    )
}

pub fn validate_selected_model_is_usable(selected: &SelectedModel) -> anyhow::Result<()> {
    if selected.provider_id == "local" {
        let state = load_state();
        let entry = state
            .custom_models
            .iter()
            .find(|entry| entry.id == selected.model_id);

        if entry.is_none_or(|entry| !model_destination(entry).exists()) {
            return Err(missing_local_model_error(&selected.model_id));
        }

        return Ok(());
    }

    let provider = TranscriptionProvider::from_id(&selected.provider_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown transcription provider: {}", selected.provider_id))?;
    if config::get_api_key(provider.id())?.is_none() {
        return Err(missing_cloud_credentials_error(&provider));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestEnv {
        previous_home: Option<std::ffi::OsString>,
        previous_models_dir: Option<std::ffi::OsString>,
        dir: std::path::PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let previous_home = std::env::var_os("HOME");
            let previous_models_dir = std::env::var_os("OSTT_MODELS_DIR");
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("ostt-model-command-test-{unique}"));
            fs::create_dir_all(&dir).expect("create temp dir");
            std::env::set_var("HOME", &dir);
            std::env::set_var("OSTT_MODELS_DIR", dir.join("models"));

            Self {
                previous_home,
                previous_models_dir,
                dir,
            }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            if let Some(previous_home) = self.previous_home.take() {
                std::env::set_var("HOME", previous_home);
            } else {
                std::env::remove_var("HOME");
            }

            if let Some(previous_models_dir) = self.previous_models_dir.take() {
                std::env::set_var("OSTT_MODELS_DIR", previous_models_dir);
            } else {
                std::env::remove_var("OSTT_MODELS_DIR");
            }

            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: id.to_string(),
            description: format!("{id} description"),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: None,
            sha256: None,
            category: None,
        }
    }

    #[test]
    fn cloud_sections_include_only_authenticated_providers() {
        let sections = build_cloud_sections(&["openai".to_string(), "local".to_string()], None);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].provider_id, "openai");
        assert!(sections[0]
            .entries
            .iter()
            .all(|entry| entry.provider_id == "openai"));
    }

    #[test]
    fn grouped_model_data_includes_cloud_and_local_sections() {
        let registry = vec![registry_entry("turbo")];
        let state = LocalModelState::default();

        let sections = build_model_sections(&["groq".to_string()], &state, &registry, None);

        assert_eq!(sections[0].provider_id, "groq");
        assert_eq!(sections[1].provider_id, "local");
        assert!(sections[1]
            .entries
            .iter()
            .any(|entry| entry.model_id == "turbo"));
    }

    #[test]
    fn local_section_marks_downloaded_status_and_management_row() {
        let _guard = env_lock().lock().expect("lock env");
        let _env = TestEnv::new();
        let registry = vec![registry_entry("turbo"), registry_entry("small")];
        fs::create_dir_all(crate::transcription::local_models::model_files_dir())
            .expect("create model files dir");
        fs::write(model_destination(&registry[0]), b"model").expect("write model file");

        let section = build_local_section(&LocalModelState::default(), &registry, None);

        assert!(matches!(
            section.entries[0].kind,
            ModelSelectionEntryKind::Local {
                is_downloaded: true,
                ..
            }
        ));
        assert!(matches!(
            section.entries[1].kind,
            ModelSelectionEntryKind::Local {
                is_downloaded: false,
                ..
            }
        ));
        assert!(matches!(
            section.entries.last().expect("management row").kind,
            ModelSelectionEntryKind::LocalManagement
        ));
    }

    #[test]
    fn active_selection_is_provider_aware() {
        let selected = SelectedModel {
            provider_id: "local".to_string(),
            model_id: "whisper".to_string(),
        };
        let sections = build_model_sections(
            &["openai".to_string()],
            &LocalModelState::default(),
            &[registry_entry("whisper")],
            Some(&selected),
        );

        let openai_whisper = sections[0]
            .entries
            .iter()
            .find(|entry| entry.model_id == "whisper")
            .expect("openai whisper");
        let local_whisper = sections[1]
            .entries
            .iter()
            .find(|entry| entry.model_id == "whisper")
            .expect("local whisper");

        assert!(!openai_whisper.is_active);
        assert!(local_whisper.is_active);
    }

    #[test]
    fn recovery_messages_are_actionable() {
        assert!(no_selected_model_error().to_string().contains("ostt model"));
        assert!(missing_cloud_credentials_error(&TranscriptionProvider::OpenAI)
            .to_string()
            .contains("ostt auth login"));
        assert!(missing_local_model_error("turbo")
            .to_string()
            .contains("download or select"));
    }
}
