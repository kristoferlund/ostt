use crate::clipboard::copy_to_clipboard;
use crate::config::{self, OsttConfig, SelectedModel};
use crate::history::HistoryManager;
use crate::keywords::KeywordsManager;
use crate::transcription::{self, TranscriptionConfig};

pub(crate) struct TranscriptionContext {
    pub(crate) selected_model: SelectedModel,
    pub(crate) config: TranscriptionConfig,
    pub(crate) keywords: Vec<String>,
}

pub(crate) fn load_config() -> anyhow::Result<OsttConfig> {
    OsttConfig::load().map_err(|err| {
        tracing::error!("Failed to load configuration: {err}");
        anyhow::anyhow!("Configuration error: {err}\n\nPlease check your ~/.config/ostt/ostt.toml file and try again.")
    })
}

pub(crate) fn load_keywords() -> anyhow::Result<Vec<String>> {
    let keywords_manager = KeywordsManager::new(&crate::app_dirs::config_dir())?;
    keywords_manager.load_keywords()
}

pub(crate) fn resolve_selected_model(
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

pub(crate) fn transcription_config(
    ostt_config: &OsttConfig,
    selected_model: &SelectedModel,
    keywords: Vec<String>,
) -> anyhow::Result<TranscriptionConfig> {
    let api_key = config::get_api_key(&selected_model.provider_id)?;
    transcription::config_for_selected_model(
        selected_model,
        api_key,
        keywords,
        ostt_config.providers.clone(),
    )
}

pub(crate) fn build_transcription_context(
    ostt_config: &OsttConfig,
    model_override: Option<SelectedModel>,
) -> anyhow::Result<TranscriptionContext> {
    let selected_model = resolve_selected_model(ostt_config, model_override)?;
    let keywords = load_keywords()?;
    let transcription_config =
        transcription_config(ostt_config, &selected_model, keywords.clone())?;

    Ok(TranscriptionContext {
        selected_model,
        config: transcription_config,
        keywords,
    })
}

pub(crate) fn save_transcription_history(text: &str) -> anyhow::Result<()> {
    let mut history_manager = HistoryManager::new(&crate::app_dirs::data_dir())?;
    if let Err(err) = history_manager.save_transcription(text) {
        tracing::warn!("Failed to save transcription to history: {}", err);
    }
    Ok(())
}

pub(crate) fn write_text_output(
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
    label: &str,
) -> anyhow::Result<()> {
    if let Some(file_path) = output_file {
        std::fs::write(&file_path, output_text).map_err(|err| {
            tracing::warn!("Failed to write to file '{file_path}': {err}");
            anyhow::anyhow!("Failed to write to file '{file_path}': {err}")
        })?;
        tracing::debug!("{label} written to file: {file_path}");
    } else if clipboard {
        match copy_to_clipboard(output_text) {
            Ok(()) => tracing::debug!("{label} copied to clipboard"),
            Err(err) => tracing::warn!("Failed to copy to clipboard: {err}"),
        }
    } else {
        println!("{output_text}");
        tracing::debug!("{label} printed to stdout");
    }

    Ok(())
}
