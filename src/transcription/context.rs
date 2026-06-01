use crate::config::{self, OsttConfig, SelectedModel};

use super::TranscriptionConfig;

pub(crate) struct TranscriptionContext {
    pub(crate) selected_model: SelectedModel,
    pub(crate) config: TranscriptionConfig,
    pub(crate) keywords: Vec<String>,
}

pub(crate) fn build_context(
    ostt_config: &OsttConfig,
    model_override: Option<SelectedModel>,
) -> anyhow::Result<TranscriptionContext> {
    let selected_model = resolve_selected_model(ostt_config, model_override)?;
    let keywords = crate::keywords::load_keywords()?;
    let transcription_config =
        config_for_selected_model(ostt_config, &selected_model, keywords.clone())?;

    Ok(TranscriptionContext {
        selected_model,
        config: transcription_config,
        keywords,
    })
}

fn resolve_selected_model(
    ostt_config: &OsttConfig,
    model_override: Option<SelectedModel>,
) -> anyhow::Result<SelectedModel> {
    if let Some(selected_model) = model_override {
        return Ok(selected_model);
    }

    match (
        ostt_config.transcription.provider.as_deref(),
        ostt_config.transcription.model.as_deref(),
    ) {
        (Some(provider_id), Some(model_id)) => Ok(SelectedModel {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        }),
        _ => Err(anyhow::anyhow!(
            "No model selected. Please run 'ostt auth' to select a transcription model"
        )),
    }
}

fn config_for_selected_model(
    ostt_config: &OsttConfig,
    selected_model: &SelectedModel,
    keywords: Vec<String>,
) -> anyhow::Result<TranscriptionConfig> {
    let api_key = config::get_api_key(&selected_model.provider_id)?;
    super::config_for_selected_model(
        selected_model,
        api_key,
        keywords,
        ostt_config.providers.clone(),
    )
}
