//! The selected transcription model, stored under `[transcription]` in `ostt.toml`.

use serde::{Deserialize, Serialize};

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

    if !provider
        .as_ref()
        .is_some_and(TranscriptionProvider::is_local)
        && provider != Some(TranscriptionProvider::Command)
        && provider != Some(TranscriptionProvider::Http)
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

    crate::config::edit::edit_config_file(|doc| {
        crate::config::edit::set_selected_model(doc, provider_id.zip(model_id))
    })
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

        let selected =
            parse_provider_model("deepinfra/openai/whisper-large-v3").expect("parse model");
        assert_eq!(selected.provider_id, "deepinfra");
        assert_eq!(selected.model_id, "openai/whisper-large-v3");
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
