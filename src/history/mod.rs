//! Transcription history storage and viewing.
//!
//! Manages persistent storage of all transcriptions with SQLite,
//! and provides an interactive terminal UI for browsing and selecting
//! past transcriptions.

pub mod history_view;
pub mod storage;

pub use history_view::HistoryView;
pub use storage::{HistoryManager, TranscriptionEntry};
