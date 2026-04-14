//! Post-process a transcription from history.
//!
//! Loads a transcription from history, optionally shows the action picker,
//! executes the selected action, and outputs the result.

use crate::clipboard::copy_to_clipboard;
use crate::config;
use crate::history::HistoryManager;
use crate::keywords::KeywordsManager;
use crate::process;
use dirs;

/// Handles post-processing of an existing transcription from history.
///
/// Loads a transcription by index, selects a processing action (via picker or
/// direct ID), executes the action, saves the result, and outputs it.
///
/// # Arguments
/// * `index` - History index (1 = most recent, None = most recent)
/// * `action_id` - Optional action ID to skip the picker
/// * `list` - If true, list configured actions and exit
/// * `clipboard` - If true, copy result to clipboard instead of stdout
/// * `output_file` - Optional file path to write result to instead of stdout
pub async fn handle_process(
    index: Option<usize>,
    action_id: Option<String>,
    list: bool,
    clipboard: bool,
    output_file: Option<String>,
) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Process Command ===");

    // Load config
    let config_data = config::OsttConfig::load().map_err(|err| {
        tracing::error!("Failed to load configuration: {err}");
        anyhow::anyhow!("Configuration error: {err}\n\nPlease check your ~/.config/ostt/ostt.toml file and try again.")
    })?;

    // --list mode: print configured actions and exit
    if list {
        if config_data.process.actions.is_empty() {
            println!("No process actions configured. Add actions to ~/.config/ostt/ostt.toml");
            return Ok(());
        }
        for action in &config_data.process.actions {
            println!("{} \u{2014} {}", action.id, action.name);
        }
        return Ok(());
    }

    // Normal mode: validate actions exist
    if config_data.process.actions.is_empty() {
        return Err(anyhow::anyhow!(
            "No process actions configured. Add actions to ~/.config/ostt/ostt.toml"
        ));
    }

    // Load transcription from history
    let data_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local")
        .join("share")
        .join("ostt");

    let mut history_manager = HistoryManager::new(&data_dir)?;
    let n = index.unwrap_or(1);
    let transcription = history_manager
        .get_transcription_by_index(n)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No transcription found at index {n}. Use 'ostt history' to see available transcriptions."
            )
        })?;

    // Determine which action to use
    let action = if let Some(ref id) = action_id {
        config_data
            .process
            .get_action(id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown action '{id}'. Use 'ostt process --list' to see available actions."
                )
            })?
            .clone()
    } else {
        // Show action picker
        match process::picker::show_action_picker(&config_data.process.actions)? {
            process::picker::PickerResult::Selected(selected_id) => config_data
                .process
                .get_action(&selected_id)
                .expect("Picker returned an ID not in config")
                .clone(),
            process::picker::PickerResult::Cancelled => {
                return Ok(());
            }
        }
    };

    // Load keywords
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let keywords_manager = KeywordsManager::new(&config_dir)?;
    let keywords = keywords_manager.load_keywords()?;

    // Execute the action
    let result = process::execute_action(&action, &transcription.text, &keywords).await?;

    // Save processed result to history
    if let Err(e) = history_manager.save_transcription(&result) {
        tracing::warn!("Failed to save processed result to history: {}", e);
    }

    // Output: file > clipboard > stdout
    if let Some(file_path) = output_file {
        match std::fs::write(&file_path, &result) {
            Ok(_) => {
                tracing::debug!("Processed result written to file: {file_path}");
            }
            Err(e) => {
                tracing::warn!("Failed to write to file '{file_path}': {e}");
                return Err(anyhow::anyhow!("Failed to write to file '{file_path}': {e}"));
            }
        }
    } else if clipboard {
        match copy_to_clipboard(&result) {
            Ok(_) => {
                tracing::debug!("Processed result copied to clipboard");
            }
            Err(e) => {
                tracing::warn!("Failed to copy to clipboard: {e}");
            }
        }
    } else {
        println!("{result}");
        tracing::debug!("Processed result printed to stdout");
    }

    Ok(())
}
