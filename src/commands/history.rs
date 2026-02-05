//! Transcription history viewer.
//!
//! Displays and manages transcription history with copy-to-clipboard functionality.

use crate::history::{HistoryManager, HistoryViewer};
use crate::clipboard::copy_to_clipboard;

/// Displays the transcription history viewer with copy-to-clipboard functionality.
///
/// # Errors
/// - If data directory cannot be determined
/// - If history manager fails to load transcriptions
pub async fn handle_history() -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt History Viewer ===");

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

    let mut viewer = HistoryViewer::new(entries)?;

    match viewer.run()? {
        Some(selected_text) => {
            copy_to_clipboard(&selected_text)?;
            tracing::info!("Selected transcription copied to clipboard");
        }
        None => {
            tracing::debug!("History viewer exited without selection");
        }
    }

    tracing::debug!("History viewer closed");
    Ok(())
}
