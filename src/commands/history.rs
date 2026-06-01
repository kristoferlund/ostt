//! Transcription history view.
//!
//! Displays and manages transcription history with copy-to-clipboard functionality.

use crate::clipboard::copy_to_clipboard;
use crate::history::{HistoryManager, HistoryView};

/// Displays the transcription history view with copy-to-clipboard functionality.
///
/// # Errors
/// - If data directory cannot be determined
/// - If history manager fails to load transcriptions
pub async fn handle_history() -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt History View ===");

    let data_dir = crate::app_dirs::data_dir();

    let mut history_manager = HistoryManager::new(&data_dir)?;
    let entries = history_manager.get_all_transcriptions()?;

    if entries.is_empty() {
        println!("No transcription history found.");
        return Ok(());
    }

    let mut view = HistoryView::new(entries)?;

    match view.run()? {
        Some(selected_text) => {
            copy_to_clipboard(&selected_text)?;
            tracing::info!("Selected transcription copied to clipboard");
        }
        None => {
            tracing::debug!("History view exited without selection");
        }
    }

    tracing::debug!("History view closed");
    Ok(())
}

pub fn handle_history_list(limit: Option<usize>, json: bool) -> Result<(), anyhow::Error> {
    let data_dir = crate::app_dirs::data_dir();
    let mut history_manager = HistoryManager::new(&data_dir)?;
    let mut entries = history_manager.get_all_transcriptions()?;
    if let Some(limit) = limit {
        entries.truncate(limit);
    }

    if json {
        let rows: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                serde_json::json!({
                    "index": index + 1,
                    "id": entry.id,
                    "created_at": entry.created_at.to_rfc3339(),
                    "text": entry.text,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    for (index, entry) in entries.iter().enumerate() {
        let preview = entry.text.lines().next().unwrap_or("");
        println!(
            "{}\t{}\t{}",
            index + 1,
            entry.created_at.format("%Y-%m-%d %H:%M"),
            preview
        );
    }

    Ok(())
}

pub fn handle_history_show(index: Option<usize>) -> Result<(), anyhow::Error> {
    let data_dir = crate::app_dirs::data_dir();
    let mut history_manager = HistoryManager::new(&data_dir)?;
    let n = index.unwrap_or(1);
    let entry = history_manager
        .get_transcription_by_index(n)?
        .ok_or_else(|| anyhow::anyhow!("No transcription found at index {n}."))?;
    println!("{}", entry.text);
    Ok(())
}

pub fn handle_history_copy(index: Option<usize>) -> Result<(), anyhow::Error> {
    let data_dir = crate::app_dirs::data_dir();
    let mut history_manager = HistoryManager::new(&data_dir)?;
    let n = index.unwrap_or(1);
    let entry = history_manager
        .get_transcription_by_index(n)?
        .ok_or_else(|| anyhow::anyhow!("No transcription found at index {n}."))?;
    copy_to_clipboard(&entry.text)?;
    println!("Copied transcription #{n} to clipboard.");
    Ok(())
}
