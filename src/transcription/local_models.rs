use std::fs;
use std::path::PathBuf;

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

pub fn model_filename(id: &str, url: &str) -> String {
    url.rsplit('/')
        .next()
        .and_then(|segment| segment.split('?').next())
        .filter(|segment| !segment.is_empty() && segment.contains('.'))
        .map(str::to_string)
        .unwrap_or_else(|| format!("{id}.gguf"))
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
        let previous = env::var_os("XDG_DATA_HOME");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ostt-local-models-test-{unique}"));
        env::set_var("XDG_DATA_HOME", &dir);

        test(dir.clone());

        if let Some(previous) = previous {
            env::set_var("XDG_DATA_HOME", previous);
        } else {
            env::remove_var("XDG_DATA_HOME");
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
}
