use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    fetch_registry, load_state, model_destination, LocalModelState, RegistryEntry,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuiModelEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_mb: u32,
    pub is_downloaded: bool,
    pub is_active: bool,
    pub is_available_in_registry: bool,
    pub languages: Vec<String>,
    pub url: String,
    pub recommended_hardware: Option<String>,
    pub category: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadState {
    pub model_id: String,
    pub progress: f64,
    pub speed_mbps: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TuiMode {
    Browse,
    CustomModelInput { input: String },
    Downloading(DownloadState),
    Info { entry: TuiModelEntry },
    ConfirmDelete { entry: TuiModelEntry },
}

#[derive(Clone, Debug)]
pub struct ModelTui {
    pub entries: Vec<TuiModelEntry>,
    pub selected: usize,
    pub mode: TuiMode,
    pub disk_usage_bytes: u64,
}

impl ModelTui {
    pub fn new(entries: Vec<TuiModelEntry>, disk_usage_bytes: u64) -> Self {
        Self {
            entries,
            selected: 0,
            mode: TuiMode::Browse,
            disk_usage_bytes,
        }
    }
}

pub fn build_model_list(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<TuiModelEntry> {
    registry
        .iter()
        .chain(local_state.custom_models.iter())
        .map(|entry| {
            let is_downloaded = model_destination(entry).exists();
            let is_active = selected_model
                .map(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
                .unwrap_or(false);

            TuiModelEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                size_mb: entry.size_mb,
                is_downloaded,
                is_active,
                is_available_in_registry: registry
                    .iter()
                    .any(|registry_entry| registry_entry.id == entry.id),
                languages: entry.languages.clone(),
                url: entry.url.clone(),
                recommended_hardware: entry.recommended_hardware.clone(),
                category: entry.category.clone(),
            }
        })
        .collect()
}

pub fn disk_usage_bytes(entries: &[TuiModelEntry]) -> u64 {
    entries
        .iter()
        .filter(|entry| entry.is_downloaded)
        .filter_map(|entry| {
            let registry_entry = RegistryEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                languages: entry.languages.clone(),
                size_mb: entry.size_mb,
                url: entry.url.clone(),
                recommended_hardware: entry.recommended_hardware.clone(),
                sha256: None,
                category: entry.category.clone(),
            };
            model_destination(&registry_entry).metadata().ok()
        })
        .map(|metadata| metadata.len())
        .sum()
}

pub async fn handle_models_tui() -> anyhow::Result<()> {
    let local_state = load_state();
    let registry = fetch_registry().await.unwrap_or_default();
    let selected_model = config::get_selected_model_entry()?;
    let entries = build_model_list(&local_state, &registry, selected_model.as_ref());
    let _tui = ModelTui::new(entries.clone(), disk_usage_bytes(&entries));

    println!("Local model TUI will open in the next implementation phase.");
    println!("{} model(s) available.", entries.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::local_models::model_files_dir;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_isolated_models_dir(test: impl FnOnce(PathBuf)) {
        let _guard = ENV_LOCK.lock().expect("test env lock poisoned");
        let previous = std::env::var_os("OSTT_MODELS_DIR");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-models-tui-test-{unique}"));
        let models_dir = dir.join("models");
        std::env::set_var("OSTT_MODELS_DIR", &models_dir);

        test(models_dir);

        if let Some(previous) = previous {
            std::env::set_var("OSTT_MODELS_DIR", previous);
        } else {
            std::env::remove_var("OSTT_MODELS_DIR");
        }
        let _ = fs::remove_dir_all(dir);
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: format!("{id} model"),
            description: "Test model".to_string(),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: Some("cpu".to_string()),
            sha256: None,
            category: None,
        }
    }

    #[test]
    fn build_model_list_merges_registry_and_custom_entries() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![RegistryEntry {
                    category: Some("custom".to_string()),
                    ..registry_entry("custom")
                }],
            };

            let entries = build_model_list(&state, &registry, None);

            assert_eq!(entries.len(), 2);
            assert!(entries.iter().any(|entry| {
                entry.id == "turbo" && entry.is_available_in_registry && !entry.is_downloaded
            }));
            assert!(entries.iter().any(|entry| {
                entry.id == "custom"
                    && !entry.is_available_in_registry
                    && entry.category.as_deref() == Some("custom")
            }));
        });
    }

    #[test]
    fn build_model_list_marks_downloaded_and_active_from_filesystem_and_selection() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let selected = SelectedModel {
                provider_id: "local".to_string(),
                model_id: "turbo".to_string(),
            };

            let entries = build_model_list(&LocalModelState::default(), &registry, Some(&selected));

            assert_eq!(entries.len(), 1);
            assert!(entries[0].is_downloaded);
            assert!(entries[0].is_active);
        });
    }

    #[test]
    fn disk_usage_sums_downloaded_model_files_only() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo"), registry_entry("base")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let entries = build_model_list(&LocalModelState::default(), &registry, None);

            assert_eq!(disk_usage_bytes(&entries), 3);
        });
    }
}
