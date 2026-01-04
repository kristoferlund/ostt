//! Recording history management for retry and replay functionality.
//!
//! Stores metadata about recent recordings so they can be replayed or retried
//! with the same configuration.

use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata about a recorded session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    /// Unique identifier for this recording session
    pub id: String,
    /// Path to the audio file
    pub audio_path: PathBuf,
    /// Model used for transcription (if transcribed)
    pub model_id: Option<String>,
    /// Timestamp when recording was created
    pub created_at: DateTime<Local>,
}

/// Manages recording history for retry and replay functionality.
pub struct RecordingHistory {
    /// Path to the history directory
    history_dir: PathBuf,
}

impl RecordingHistory {
    /// Creates a new recording history manager.
    pub fn new(data_dir: &Path) -> Result<Self> {
        let history_dir = data_dir.join("recording_history");
        fs::create_dir_all(&history_dir)?;
        Ok(Self { history_dir })
    }

    /// Saves recording metadata for a new recording session.
    pub fn save_recording(&self, audio_path: PathBuf, model_id: Option<String>) -> Result<String> {
        let now = Local::now();
        let recording_id = now.timestamp_millis().to_string();
        let metadata = RecordingMetadata {
            id: recording_id.clone(),
            audio_path,
            model_id,
            created_at: now,
        };
        let metadata_path = self.history_dir.join(format!("{}.json", recording_id));
        let json = serde_json::to_string_pretty(&metadata)?;
        fs::write(metadata_path, json)?;
        tracing::info!("Recording metadata saved with ID: {}", recording_id);
        Ok(recording_id)
    }

    /// Retrieves the most recent recording metadata (for retry).
    pub fn get_last_recording(&self) -> Result<Option<RecordingMetadata>> {
        let entries = fs::read_dir(&self.history_dir)?;
        let mut recordings: Vec<(PathBuf, DateTime<Local>)> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                    let metadata = fs::read_to_string(&path).ok()?;
                    let meta: RecordingMetadata = serde_json::from_str(&metadata).ok()?;
                    Some((path, meta.created_at))
                } else {
                    None
                }
            })
            .collect();
        if recordings.is_empty() {
            return Ok(None);
        }
        recordings.sort_by(|a, b| b.1.cmp(&a.1));
        let metadata_content = fs::read_to_string(&recordings[0].0)?;
        let metadata = serde_json::from_str(&metadata_content)?;
        Ok(Some(metadata))
    }

    /// Retrieves all recordings ordered by most recent first.
    pub fn get_all_recordings(&self) -> Result<Vec<RecordingMetadata>> {
        let entries = fs::read_dir(&self.history_dir)?;
        let mut recordings: Vec<(PathBuf, DateTime<Local>)> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                    let metadata = fs::read_to_string(&path).ok()?;
                    let meta: RecordingMetadata = serde_json::from_str(&metadata).ok()?;
                    Some((path, meta.created_at))
                } else {
                    None
                }
            })
            .collect();
        recordings.sort_by(|a, b| b.1.cmp(&a.1));
        let mut results = Vec::new();
        for (path, _) in recordings {
            let metadata_content = fs::read_to_string(path)?;
            let metadata = serde_json::from_str(&metadata_content)?;
            results.push(metadata);
        }
        Ok(results)
    }

    /// Retrieves a recording by its ID.
    pub fn get_recording(&self, id: &str) -> Result<Option<RecordingMetadata>> {
        let metadata_path = self.history_dir.join(format!("{}.json", id));
        if !metadata_path.exists() {
            return Ok(None);
        }
        let metadata_content = fs::read_to_string(metadata_path)?;
        let metadata = serde_json::from_str(&metadata_content)?;
        Ok(Some(metadata))
    }
}
