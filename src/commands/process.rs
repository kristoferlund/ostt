//! Post-process a transcription from history.
//!
//! Loads a transcription from history, optionally shows the action picker,
//! executes the selected action, and outputs the result.

use super::output;
use crate::config;
use crate::history::HistoryManager;
use crate::keywords;
use crate::process;

/// Handles post-processing of an existing transcription from history.
///
/// Loads a transcription by index, selects a processing action (via picker or
/// direct ID), executes the action, saves the result, and outputs it.
///
/// # Arguments
/// * `index` - History index (1 = most recent, None = most recent)
/// * `action_id` - Optional positional action ID to skip the picker
/// * `list` - If true, list configured actions and exit
/// * `clipboard` - If true, copy result to clipboard instead of stdout
/// * `output_file` - Optional file path to write result to instead of stdout
pub async fn handle_process(
    config_data: &config::OsttConfig,
    index: Option<usize>,
    action_id: Option<String>,
    list: bool,
    clipboard: bool,
    output_file: Option<String>,
) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Process Command ===");

    if config_data.process.actions.is_empty() {
        return Err(anyhow::anyhow!(
            "No process actions configured. Add actions to ~/.config/ostt/ostt.toml"
        ));
    }

    // --list mode: print configured actions and exit
    if list {
        for action in &config_data.process.actions {
            println!("{} \u{2014} {}", action.id, action.name);
        }
        return Ok(());
    }

    // Load transcription from history
    let data_dir = crate::app_dirs::data_dir();

    let mut history_manager = HistoryManager::new(&data_dir)?;
    let n = index.unwrap_or(1);
    let transcription = history_manager
        .get_transcription_by_index(n)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No transcription found at index {n}. Use 'ostt history' to see available transcriptions."
            )
        })?;

    // Determine which action to use and whether the picker was shown
    let (action, picker_was_shown) = if let Some(ref id) = action_id {
        let a = config_data
            .process
            .get_action(id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown action '{id}'. Use 'ostt process list' to see available actions."
                )
            })?
            .clone();
        (a, false)
    } else {
        // Show action picker
        match process::process_view::show_action_picker(&config_data.process.actions)? {
            process::process_view::PickerResult::Selected(selected_id) => {
                let a = config_data
                    .process
                    .get_action(&selected_id)
                    .expect("Picker returned an ID not in config")
                    .clone();
                (a, true)
            }
            process::process_view::PickerResult::Cancelled => {
                return Ok(());
            }
        }
    };

    tracing::info!("Executing action '{}' on transcription #{}", action.id, n);

    // Load keywords
    let keywords = keywords::load_keywords()?;

    // Use animation if the picker was shown (we're in a TUI flow),
    // otherwise execute directly (no TUI was started)
    let result = if picker_was_shown {
        match process::execute_action_with_animation(&action, &transcription.text, &keywords)
            .await?
        {
            Some(r) => r,
            None => {
                // User cancelled during processing
                return Ok(());
            }
        }
    } else {
        process::execute_action(&action, &transcription.text, &keywords).await?
    };

    output::write_text(&result, output_file, clipboard, "Processed result")?;

    tracing::info!("=== ostt Process Command Completed ===");
    Ok(())
}

pub fn handle_process_list(
    config_data: &config::OsttConfig,
    json: bool,
) -> Result<(), anyhow::Error> {
    if config_data.process.actions.is_empty() {
        return Err(anyhow::anyhow!(
            "No process actions configured. Add actions to ~/.config/ostt/ostt.toml"
        ));
    }

    if json {
        let actions: Vec<_> = config_data
            .process
            .actions
            .iter()
            .map(|action| {
                serde_json::json!({
                    "id": action.id,
                    "name": action.name,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&actions)?);
        return Ok(());
    }

    for action in &config_data.process.actions {
        println!("{} — {}", action.id, action.name);
    }

    Ok(())
}
