use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub languages: Vec<String>,
    pub size_mb: u32,
    pub url: String,
    pub recommended_hardware: Option<String>,
    pub sha256: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelState {
    pub version: u32,
    #[serde(default)]
    pub custom_models: Vec<RegistryEntry>,
}

impl Default for LocalModelState {
    fn default() -> Self {
        Self {
            version: 1,
            custom_models: Vec::new(),
        }
    }
}

/// Returns the directory where local transcription models are stored.
pub fn models_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("OSTT_MODELS_DIR") {
        return PathBuf::from(path);
    }

    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("ostt").join("models")
}

fn state_path() -> PathBuf {
    models_dir().join("models.json")
}

pub fn model_files_dir() -> PathBuf {
    models_dir().join("files")
}

/// Error type for local model-related failures.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Model '{0}' is not downloaded. Run `ostt models download {0}` first.")]
    NotDownloaded(String),
    #[error("Model file not found at {0}")]
    FileNotFound(PathBuf),
    #[error("Failed to load model: {0}")]
    LoadFailed(String),
    #[error("Local model registry source is not configured")]
    RegistryUnavailable,
}

#[derive(Debug, Clone)]
pub struct InstalledModelView {
    pub entry: RegistryEntry,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub is_active: bool,
}

pub fn model_filename(id: &str, url: &str) -> String {
    let extension = url
        .split(['?', '#'])
        .next()
        .and_then(|without_query| without_query.rsplit('/').next())
        .and_then(|segment| segment.rsplit_once('.').map(|(_, extension)| extension))
        .filter(|extension| !extension.is_empty())
        .unwrap_or("bin");

    format!("{id}.{extension}")
}

pub fn is_safe_model_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'))
}

pub fn installed_models(
    registry: &[RegistryEntry],
    state: &LocalModelState,
    selected_model: Option<&str>,
) -> Vec<InstalledModelView> {
    registry
        .iter()
        .chain(state.custom_models.iter())
        .filter_map(|entry| {
            let path = model_files_dir().join(model_filename(&entry.id, &entry.url));
            let metadata = fs::metadata(&path).ok()?;

            Some(InstalledModelView {
                entry: entry.clone(),
                path,
                size_bytes: metadata.len(),
                modified_at: metadata.modified().ok(),
                is_active: selected_model == Some(entry.id.as_str()),
            })
        })
        .collect()
}

pub fn load_registry_entries() -> Result<Vec<RegistryEntry>, ModelError> {
    Err(ModelError::RegistryUnavailable)
}

pub fn load_state() -> LocalModelState {
    let path = state_path();
    if !path.exists() {
        return LocalModelState::default();
    }

    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_state(state: &LocalModelState) -> anyhow::Result<()> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(state)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_custom_model_entries() -> Result<Vec<RegistryEntry>, ModelError> {
    Ok(load_state().custom_models)
}

pub fn resolve_installed_model_path(model_id: &str) -> Result<PathBuf, ModelError> {
    let entry = find_model_entry(model_id)?;
    let path = model_files_dir().join(model_filename(&entry.id, &entry.url));

    if path.exists() {
        Ok(path)
    } else {
        Err(ModelError::NotDownloaded(model_id.to_string()))
    }
}

fn find_model_entry(model_id: &str) -> Result<RegistryEntry, ModelError> {
    if let Some(entry) = load_custom_model_entries()?
        .into_iter()
        .find(|entry| entry.id == model_id)
    {
        return Ok(entry);
    }

    let registry_entries = load_registry_entries()?;
    registry_entries
        .into_iter()
        .find(|entry| entry.id == model_id)
        .ok_or_else(|| ModelError::NotDownloaded(model_id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_isolated_data_dir(test: impl FnOnce(PathBuf)) {
        let _guard = ENV_LOCK.lock().expect("test env lock poisoned");
        let previous = env::var_os("OSTT_MODELS_DIR");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ostt-local-models-test-{unique}"));
        let models_dir = dir.join("models");
        env::set_var("OSTT_MODELS_DIR", &models_dir);

        test(models_dir);

        if let Some(previous) = previous {
            env::set_var("OSTT_MODELS_DIR", previous);
        } else {
            env::remove_var("OSTT_MODELS_DIR");
        }
        let _ = fs::remove_dir_all(dir);
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: "Test Model".to_string(),
            description: "Test model".to_string(),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: None,
            sha256: None,
            category: None,
        }
    }

    fn registry_entry_with_url(id: &str, url: &str) -> RegistryEntry {
        RegistryEntry {
            url: url.to_string(),
            ..registry_entry(id)
        }
    }

    #[test]
    fn load_state_returns_default_when_missing() {
        with_isolated_data_dir(|_| {
            let state = load_state();

            assert_eq!(state.version, 1);
            assert!(state.custom_models.is_empty());
        });
    }

    #[test]
    fn load_state_returns_default_when_corrupted() {
        with_isolated_data_dir(|_| {
            fs::create_dir_all(models_dir()).expect("create models dir");
            fs::write(state_path(), "not json").expect("write corrupted state");

            let state = load_state();

            assert_eq!(state.version, 1);
            assert!(state.custom_models.is_empty());
        });
    }

    #[test]
    fn save_and_load_state_round_trips() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };

            save_state(&state).expect("save state");
            let loaded = load_state();

            assert_eq!(loaded.version, 1);
            assert_eq!(loaded.custom_models.len(), 1);
            assert_eq!(loaded.custom_models[0].id, "custom");
            assert_eq!(loaded.custom_models[0].languages, vec!["en"]);
        });
    }

    #[test]
    fn save_state_creates_parent_directory() {
        with_isolated_data_dir(|dir| {
            assert!(!dir.join("ostt").join("models").exists());

            save_state(&LocalModelState::default()).expect("save state");

            assert!(state_path().exists());
        });
    }

    #[test]
    fn model_filename_uses_id_with_url_extension() {
        assert_eq!(
            model_filename("kb-whisper-large", "https://example.com/models/kb.bin?download=1"),
            "kb-whisper-large.bin"
        );
        assert_eq!(
            model_filename("turbo", "https://example.com/ggml-turbo.gguf#fragment"),
            "turbo.gguf"
        );
        assert_eq!(model_filename("custom", "https://example.com/download"), "custom.bin");
    }

    #[test]
    fn safe_model_id_allows_only_portable_filename_characters() {
        assert!(is_safe_model_id("kb-whisper.large_v3"));
        assert!(!is_safe_model_id(""));
        assert!(!is_safe_model_id("Large"));
        assert!(!is_safe_model_id("model/name"));
        assert!(!is_safe_model_id("model name"));
    }

    #[test]
    fn installed_models_discovers_registry_and_custom_files() {
        with_isolated_data_dir(|_| {
            let registry = vec![registry_entry_with_url(
                "turbo",
                "https://example.com/ggml-turbo.gguf",
            )];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.gguf"), [1, 2, 3]).expect("write registry model");
            fs::write(model_files_dir().join("custom.bin"), [4, 5]).expect("write custom model");

            let installed = installed_models(&registry, &state, None);

            assert_eq!(installed.len(), 2);
            assert!(installed.iter().any(|model| {
                model.entry.id == "turbo"
                    && model.path == model_files_dir().join("turbo.gguf")
                    && model.size_bytes == 3
                    && model.modified_at.is_some()
            }));
            assert!(installed.iter().any(|model| {
                model.entry.id == "custom"
                    && model.path == model_files_dir().join("custom.bin")
                    && model.size_bytes == 2
            }));
        });
    }

    #[test]
    fn installed_models_marks_selected_model_active() {
        with_isolated_data_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1]).expect("write model");

            let installed = installed_models(&registry, &LocalModelState::default(), Some("turbo"));

            assert_eq!(installed.len(), 1);
            assert!(installed[0].is_active);
        });
    }

    #[test]
    fn installed_models_does_not_persist_registry_entries() {
        with_isolated_data_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1]).expect("write model");

            let installed = installed_models(&registry, &LocalModelState::default(), None);

            assert_eq!(installed.len(), 1);
            assert!(!state_path().exists());
        });
    }
}
