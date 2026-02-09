//! Transcribe a pre-recorded audio file without recording.
//!
//! Accepts an audio file path and transcribes it using the configured provider/model,
//! reusing the same transcription pipeline as the `record` command.

use crate::clipboard::copy_to_clipboard;
use crate::config;
use crate::history::HistoryManager;
use crate::keywords::KeywordsManager;
use crate::transcription;
use dirs;
use std::path::PathBuf;

/// Handles transcription of a pre-recorded audio file.
///
/// Transcribes the given audio file using the currently configured provider and model.
/// Supports the same output options as `record` and `retry`.
///
/// # Arguments
/// * `file` - Path to the audio file to transcribe
/// * `clipboard` - If true, copy to clipboard instead of stdout
/// * `output_file` - Optional file path to write output to instead of stdout
pub async fn handle_transcribe(
    file: PathBuf,
    clipboard: bool,
    output_file: Option<String>,
) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Transcribe Command ===");

    // Validate the input file exists
    if !file.exists() {
        return Err(anyhow::anyhow!(
            "Audio file not found: {}",
            file.display()
        ));
    }

    tracing::info!("Transcribing file: {}", file.display());

    // Load configuration
    let config_data = match config::OsttConfig::load() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Failed to load configuration: {err}");
            return Err(anyhow::anyhow!("Configuration error: {err}"));
        }
    };

    // Get the selected model from config
    let selected_model_id = config::get_selected_model().ok().flatten();

    let model_id = selected_model_id.ok_or_else(|| {
        anyhow::anyhow!("No model selected. Please run 'ostt auth' to select a transcription model")
    })?;

    let model = transcription::TranscriptionModel::from_id(&model_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown model: {model_id}"))?;
    let provider = model.provider();

    let api_key = config::get_api_key(provider.id())?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No API key for {}. Please run 'ostt auth'",
                provider.name()
            )
        })?;

    // Load keywords
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let keywords_manager = KeywordsManager::new(&config_dir)?;
    let keywords = keywords_manager.load_keywords()?;

    // Prepare transcription config
    let transcription_config = transcription::TranscriptionConfig::new(
        model,
        api_key,
        keywords,
        config_data.providers.clone(),
    );

    // Transcribe
    tracing::debug!("Starting transcription...");
    let text = transcription::transcribe(&transcription_config, &file)
        .await
        .map_err(|e| {
            tracing::error!("Transcription failed: {e}");
            anyhow::anyhow!("Transcription failed: {e}")
        })?;

    let trimmed_text = text.trim().to_string();
    tracing::debug!("Transcription completed: {}", trimmed_text);

    // Save to history
    let data_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local")
        .join("share")
        .join("ostt");
    let mut history_manager = HistoryManager::new(&data_dir)?;
    let history_note = format!("[Transcribed from {}]", file.display());
    if let Err(e) = history_manager.save_transcription(&format!("{history_note} {trimmed_text}")) {
        tracing::warn!("Failed to save transcription to history: {}", e);
    }

    // Determine output destination: file > clipboard > stdout (default)
    if let Some(file_path) = output_file {
        std::fs::write(&file_path, &trimmed_text)
            .map_err(|e| anyhow::anyhow!("Failed to write to file '{file_path}': {e}"))?;
        tracing::debug!("Transcribed text written to file: {file_path}");
    } else if clipboard {
        if let Err(e) = copy_to_clipboard(&trimmed_text) {
            tracing::warn!("Failed to copy to clipboard: {e}");
        } else {
            tracing::debug!("Transcription copied to clipboard");
        }
    } else {
        // Default: stdout
        println!("{trimmed_text}");
        tracing::debug!("Transcribed text printed to stdout");
    }

    Ok(())
}
