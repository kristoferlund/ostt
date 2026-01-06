//! Retry transcription of a previous recording without re-recording audio.

use crate::clipboard::copy_to_clipboard;
use crate::config;
use crate::history::HistoryManager;
use crate::keywords::KeywordsManager;
use crate::recording::RecordingHistory;
use crate::transcription;
use crate::ui::ErrorScreen;
use dirs;

/// Retries transcription of a previous recording.
///
/// Allows users to re-transcribe a recording with the same or different settings.
/// Useful when transcription failed due to network issues, API key problems, etc.
///
/// # Arguments
/// * `recording_index` - Optional index of recording to retry (1 = most recent, None = most recent)
pub async fn handle_retry(recording_index: Option<usize>) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Retry Command ===");

    let data_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local")
        .join("share")
        .join("ostt");

    let recording_history = RecordingHistory::new(&data_dir)?;
    let all_recordings = recording_history.get_all_recordings()?;

    if all_recordings.is_empty() {
        return Err(anyhow::anyhow!("No recordings found in history"));
    }

    // Get recording by index (1-indexed, where 1 is most recent)
    let index = recording_index.unwrap_or(1);
    if index < 1 || index > all_recordings.len() {
        return Err(anyhow::anyhow!(
            "Recording index out of range. Available recordings: 1-{}",
            all_recordings.len()
        ));
    }

    let audio_path = &all_recordings[index - 1];

    if !audio_path.exists() {
        return Err(anyhow::anyhow!(
            "Audio file not found: {}",
            audio_path.display()
        ));
    }

    tracing::info!(
        "Retrying transcription for recording #{}",
        index
    );

    // Load configuration
    let config_data = match config::OsttConfig::load() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Failed to load configuration: {err}");
            let error_message = format!(
                "Configuration Error:\n\n{err}\n\nPlease check your ~/.config/ostt/ostt.toml file and try again."
            );
            let mut error_screen = ErrorScreen::new()?;
            error_screen.show_error(&error_message)?;
            error_screen.cleanup()?;
            return Err(anyhow::anyhow!("Configuration error: {err}"));
        }
    };

    // Get the selected model from config
    let selected_model_id = config::get_selected_model().ok().flatten();

    if let Some(model_id) = selected_model_id {
        // Get API key
        let model = transcription::TranscriptionModel::from_id(&model_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {model_id}"))?;
        let provider = model.provider();

        let api_key = match config::get_api_key(provider.id())? {
            Some(key) => key,
            None => {
                return Err(anyhow::anyhow!(
                    "No API key for {}. Please run 'ostt auth'",
                    provider.name()
                ));
            }
        };

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
        tracing::info!("Starting transcription for retry...");
        match transcription::transcribe(&transcription_config, audio_path).await {
            Ok(text) => {
                let trimmed_text = text.trim().to_string();
                tracing::info!("Retry transcription completed: {}", trimmed_text);

                // Save to history
                let mut history_manager = HistoryManager::new(&data_dir)?;
                let history_note = format!("[Retried from recording #{}]", index);
                if let Err(e) = history_manager
                    .save_transcription(&format!("{} {}", history_note, trimmed_text))
                {
                    tracing::warn!("Failed to save transcription to history: {}", e);
                }

                // Copy to clipboard
                match copy_to_clipboard(&trimmed_text) {
                    Ok(_) => {
                        tracing::debug!("Retried transcription copied to clipboard");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to copy to clipboard: {e}");
                    }
                }

                Ok(())
            }
            Err(e) => {
                tracing::error!("Retry transcription failed: {e}");
                Err(anyhow::anyhow!("Transcription failed: {e}"))
            }
        }
    } else {
        Err(anyhow::anyhow!(
            "No model selected. Please run 'ostt auth' to select a transcription model"
        ))
    }
}
