//! Transcription history storage and viewing.
//!
//! Manages persistent storage of all transcriptions with SQLite,
//! and provides an interactive terminal UI for browsing and selecting
//! past transcriptions.

pub mod history_view;
pub mod storage;

pub use history_view::HistoryView;
pub use storage::{HistoryManager, TranscriptionEntry};

pub(crate) fn save_transcription(text: &str) -> anyhow::Result<()> {
    let mut history_manager = HistoryManager::new(&crate::app_dirs::data_dir())?;
    if let Err(err) = history_manager.save_transcription(text) {
        tracing::warn!("Failed to save transcription to history: {}", err);
    }
    Ok(())
}
