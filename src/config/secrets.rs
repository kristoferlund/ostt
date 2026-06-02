//! API credentials storage for ostt.
//!
//! This module handles secure storage of API credentials with restricted file permissions.
//! Credentials are stored in the user's local data directory (~/.local/share/ostt).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::file::{get_config_path, OsttConfig};
use crate::transcription::{find_model, TranscriptionProvider};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedModel {
    pub provider_id: String,
    pub model_id: String,
}

pub fn parse_provider_model(value: &str) -> anyhow::Result<SelectedModel> {
    let (provider_id, model_id) = value.split_once('/').ok_or_else(|| {
        anyhow::anyhow!("Expected model in PROVIDER/MODEL format, for example deepgram/nova-3")
    })?;

    if provider_id.trim().is_empty() || model_id.trim().is_empty() {
        anyhow::bail!("Expected model in PROVIDER/MODEL format, for example deepgram/nova-3");
    }

    if provider_id == "local" {
        anyhow::bail!("Provider 'local' is no longer supported. Use 'whisper/{model_id}'.");
    }

    let provider = TranscriptionProvider::from_id(provider_id);
    if provider.is_none() {
        anyhow::bail!(
            "Unknown provider '{}'. Supported providers: {}.",
            provider_id,
            TranscriptionProvider::supported_ids().join(", ")
        );
    }

    if provider != Some(TranscriptionProvider::Whisper)
        && find_model(provider_id, model_id).is_none()
    {
        anyhow::bail!(
            "Unknown model '{}' for provider '{}'. Please run 'ostt model' to select a supported model.",
            model_id,
            provider_id
        );
    }

    Ok(SelectedModel {
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
    })
}

/// Returns the path to the secrets directory (~/.local/share/ostt).
///
/// Creates the directory if it doesn't exist.
///
/// # Errors
/// - If the local data directory cannot be determined
/// - If the secrets directory cannot be created
fn get_secrets_dir() -> anyhow::Result<PathBuf> {
    let secrets_dir = crate::app_dirs::data_dir();
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
///
/// # Errors
/// - If the config file cannot be written
pub fn save_selected_model(provider_id: &str, model_id: &str) -> anyhow::Result<()> {
    save_transcription_selection(Some(provider_id), Some(model_id))?;
    tracing::info!("Model selected: {} ({})", model_id, provider_id);
    Ok(())
}

pub fn clear_selected_model() -> anyhow::Result<()> {
    save_transcription_selection(None, None)
}

pub fn get_selected_model_entry() -> anyhow::Result<Option<SelectedModel>> {
    let config = match OsttConfig::load() {
        Ok(c) => c,
        Err(e) if e.to_string().contains("No such file") => return Ok(None),
        Err(e) => return Err(anyhow::anyhow!("{e}")),
    };
    let entry = match (
        config.transcription.provider.as_deref(),
        config.transcription.model.as_deref(),
    ) {
        (Some(provider_id), Some(model_id)) => Some(SelectedModel {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        }),
        _ => None,
    };
    Ok(entry)
}

fn save_transcription_selection(
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> anyhow::Result<()> {
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
            // Stop skipping only at top-level sections, not subsections like
            // [transcription.local] which belong to the section being removed.
            let is_subsection = trimmed.starts_with(&format!("[{section}."));
            if !is_subsection {
                skipping = false;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_provider_model_accepts_simple_model_ids() {
        let selected = parse_provider_model("deepgram/nova-3").expect("parse model");
        assert_eq!(selected.provider_id, "deepgram");
        assert_eq!(selected.model_id, "nova-3");
    }

    #[test]
    fn parse_provider_model_splits_on_first_slash_only() {
        let selected = parse_provider_model("berget/openai/whisper-large-v3").expect("parse model");
        assert_eq!(selected.provider_id, "berget");
        assert_eq!(selected.model_id, "openai/whisper-large-v3");

        let selected = parse_provider_model("deepinfra/openai/whisper-base").expect("parse model");
        assert_eq!(selected.provider_id, "deepinfra");
        assert_eq!(selected.model_id, "openai/whisper-base");
    }

    #[test]
    fn parse_provider_model_rejects_missing_provider_or_model() {
        assert!(parse_provider_model("openai/").is_err());
        assert!(parse_provider_model("/nova-3").is_err());
        assert!(parse_provider_model("nova-3").is_err());
    }

    #[test]
    fn parse_provider_model_rejects_unknown_cloud_tuple() {
        let err = parse_provider_model("openai/nova-3").expect_err("reject mismatch");
        assert!(err
            .to_string()
            .contains("Unknown model 'nova-3' for provider 'openai'"));
    }
}
