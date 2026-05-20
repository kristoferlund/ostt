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

    let data_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local")
        .join("share")
        .join("ostt");

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
