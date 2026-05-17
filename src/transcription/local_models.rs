use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalModelState {
    #[serde(default)]
    pub custom_models: Vec<RegistryEntry>,
}

/// Returns the directory where local transcription models are stored.
pub fn models_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("ostt").join("models")
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

pub fn load_custom_model_entries() -> Result<Vec<RegistryEntry>, ModelError> {
    let state_path = models_dir().join("models.json");

    if !state_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&state_path)
        .map_err(|err| ModelError::LoadFailed(format!("{}: {err}", state_path.display())))?;
    let state: LocalModelState = serde_json::from_str(&content)
        .map_err(|err| ModelError::LoadFailed(format!("{}: {err}", state_path.display())))?;

    Ok(state.custom_models)
}

pub fn resolve_installed_model_path(model_id: &str) -> Result<PathBuf, ModelError> {
    let entry = find_model_entry(model_id)?;
    let path = models_dir()
        .join("files")
        .join(model_filename(&entry.id, &entry.url));

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
