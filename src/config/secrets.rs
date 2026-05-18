//! API credentials storage for ostt.
//!
//! This module handles secure storage of API credentials with restricted file permissions.
//! Credentials are stored in the user's local data directory (~/.local/share/ostt).

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::file::{get_config_path, OsttConfig};
use crate::transcription::model::TranscriptionModel;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedModel {
    pub provider_id: String,
    pub model_id: String,
}

/// Returns the path to the secrets directory (~/.local/share/ostt).
///
/// Creates the directory if it doesn't exist.
///
/// # Errors
/// - If the local data directory cannot be determined
/// - If the secrets directory cannot be created
fn get_secrets_dir() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::home_dir().context("Could not find home directory")?;
    let secrets_dir = data_dir.join(".local").join("share").join("ostt");
    fs::create_dir_all(&secrets_dir)?;
    Ok(secrets_dir)
}

/// Saves an API key for the specified provider.
///
/// Stores credentials in ~/.local/share/ostt/credentials with restricted permissions (0600).
///
/// # Errors
/// - If the secrets directory cannot be determined or created
/// - If the credentials file cannot be read or written
/// - If the TOML cannot be serialized
pub fn save_api_key(provider_id: &str, api_key: &str) -> anyhow::Result<()> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    let mut credentials: HashMap<String, String> = if credentials_file.exists() {
        let content = fs::read_to_string(&credentials_file)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    credentials.insert(provider_id.to_string(), api_key.to_string());

    let content = toml::to_string(&credentials)?;
    fs::write(&credentials_file, content)?;

    #[cfg(unix)]
    {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&credentials_file, Permissions::from_mode(0o600))?;
    }

    tracing::info!("API key saved for provider: {}", provider_id);
    Ok(())
}

/// Retrieves the API key for the specified provider.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the credentials file cannot be read
/// - If the TOML cannot be parsed
pub fn get_api_key(provider_id: &str) -> anyhow::Result<Option<String>> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    if !credentials_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&credentials_file)?;
    let credentials: HashMap<String, String> = toml::from_str(&content).unwrap_or_default();

    Ok(credentials.get(provider_id).cloned())
}

/// Returns all available provider IDs that have API keys saved.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the credentials file cannot be read
/// - If the TOML cannot be parsed
pub fn get_authorized_providers() -> anyhow::Result<Vec<String>> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    if !credentials_file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&credentials_file)?;
    let credentials: HashMap<String, String> = toml::from_str(&content).unwrap_or_default();

    Ok(credentials.keys().cloned().collect())
}

/// Clears the API key for the specified provider.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the credentials file cannot be read or written
/// - If the TOML cannot be serialized
pub fn clear_api_key(provider_id: &str) -> anyhow::Result<()> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    if !credentials_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&credentials_file)?;
    let mut credentials: HashMap<String, String> = toml::from_str(&content).unwrap_or_default();

    if credentials.remove(provider_id).is_some() {
        let content = toml::to_string(&credentials)?;
        fs::write(&credentials_file, content)?;
        tracing::info!("API key cleared for provider: {}", provider_id);
    }

    Ok(())
}

/// Saves the selected model globally (only ONE model is selected at a time).
///
/// Stores provider/model selection in the main config file under `[transcription]`.
/// The legacy `~/.local/share/ostt/model` file is still read as a fallback.
///
/// # Errors
/// - If the secrets directory cannot be determined or created
/// - If the model file cannot be written
pub fn save_selected_model(provider_id: &str, model_id: &str) -> anyhow::Result<()> {
    save_transcription_selection(Some(provider_id), Some(model_id))?;

    tracing::info!("Model selected: {}", model_id);
    Ok(())
}

pub fn clear_selected_model() -> anyhow::Result<()> {
    save_transcription_selection(None, None)?;

    Ok(())
}

pub fn get_selected_model_entry() -> anyhow::Result<Option<SelectedModel>> {
    if let Ok(config) = OsttConfig::load() {
        if let (Some(provider_id), Some(model_id)) = (
            config.transcription.provider.as_deref(),
            config.transcription.model.as_deref(),
        ) {
            return Ok(Some(SelectedModel {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
            }));
        }
    }

    legacy_selected_model_entry()
}

pub(crate) fn legacy_selected_model_entry() -> anyhow::Result<Option<SelectedModel>> {
    let secrets_dir = get_secrets_dir()?;
    let model_file = secrets_dir.join("model");

    if !model_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&model_file)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Ok(selected_model) = serde_json::from_str::<SelectedModel>(trimmed) {
        return Ok(Some(selected_model));
    }

    let provider_id = TranscriptionModel::from_id(trimmed)
        .map(|model| model.provider().id().to_string())
        .unwrap_or_else(|| "local".to_string());

    Ok(Some(SelectedModel {
        provider_id,
        model_id: trimmed.to_string(),
    }))
}

fn save_transcription_selection(provider_id: Option<&str>, model_id: Option<&str>) -> anyhow::Result<()> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        let mut config = OsttConfig::default();
        config.transcription.provider = provider_id.map(ToString::to_string);
        config.transcription.model = model_id.map(ToString::to_string);
        return config.save();
    }

    let content = fs::read_to_string(&config_path)?;
    let without_transcription = remove_toml_section(&content, "transcription");
    let updated = match (provider_id, model_id) {
        (Some(provider_id), Some(model_id)) => insert_transcription_section(
            &without_transcription,
            &format!(
                "[transcription]\nprovider = {}\nmodel = {}\n",
                toml_basic_string(provider_id),
                toml_basic_string(model_id)
            ),
        ),
        _ => without_transcription,
    };
    fs::write(config_path, updated)?;
    Ok(())
}

fn remove_toml_section(content: &str, section: &str) -> String {
    let header = format!("[{section}]");
    let mut output = Vec::new();
    let mut skipping = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == header {
            skipping = true;
            continue;
        }
        if skipping && trimmed.starts_with('[') && trimmed.ends_with(']') {
            skipping = false;
        }
        if !skipping {
            output.push(line);
        }
    }

    trim_extra_blank_lines(&output.join("\n"))
}

fn insert_transcription_section(content: &str, section: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    let insert_at = lines
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .unwrap_or(lines.len());
    let mut section_lines: Vec<&str> = section.trim_end().lines().collect();
    section_lines.push("");
    lines.splice(insert_at..insert_at, section_lines);
    format!("{}\n", trim_extra_blank_lines(&lines.join("\n")))
}

fn trim_extra_blank_lines(content: &str) -> String {
    let mut output = Vec::new();
    let mut previous_blank = false;
    for line in content.lines() {
        let blank = line.trim().is_empty();
        if blank && previous_blank {
            continue;
        }
        output.push(line);
        previous_blank = blank;
    }
    output.join("\n").trim().to_string()
}

fn toml_basic_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Retrieves the currently selected model.
///
/// Returns the model ID of the currently selected transcription model.
/// Only one model is selected globally at any time.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the model file cannot be read
pub fn get_selected_model() -> anyhow::Result<Option<String>> {
    Ok(get_selected_model_entry()?.map(|selected| selected.model_id))
}
