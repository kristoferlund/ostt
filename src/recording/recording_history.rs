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
    /// 
    /// Keeps only the 10 most recent recordings. If there are already 10 recordings,
    /// the oldest one (including its audio file) is deleted before saving the new one.
    pub fn save_recording(&self, audio_path: PathBuf, model_id: Option<String>) -> Result<String> {
        // Clean up old recordings if we already have 10
        self.cleanup_old_recordings()?;

        let now = Local::now();
        let recording_id = now.timestamp_millis().to_string();
        let metadata = RecordingMetadata {
            id: recording_id.clone(),
            audio_path: audio_path.clone(),
            model_id,
            created_at: now,
        };
        let metadata_path = self.history_dir.join(format!("{}.json", recording_id));
        let json = serde_json::to_string_pretty(&metadata)?;
        fs::write(metadata_path, json)?;
        tracing::info!("Recording metadata saved with ID: {}", recording_id);

        Ok(recording_id)
    }

    /// Removes the oldest recording if there are more than 10 recordings.
    fn cleanup_old_recordings(&self) -> Result<()> {
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

        // If we have 10 or more recordings, delete the oldest to make room
        if recordings.len() >= 10 {
            recordings.sort_by(|a, b| a.1.cmp(&b.1));
            let oldest_metadata_path = &recordings[0].0;
            
            // Load the metadata to get the audio file path
            if let Ok(metadata_content) = fs::read_to_string(oldest_metadata_path) {
                if let Ok(metadata) = serde_json::from_str::<RecordingMetadata>(&metadata_content) {
                    // Delete the audio file
                    if metadata.audio_path.exists() {
                        if let Err(e) = fs::remove_file(&metadata.audio_path) {
                            tracing::warn!("Failed to delete old recording audio: {}", e);
                        } else {
                            tracing::info!("Deleted old recording audio: {}", metadata.audio_path.display());
                        }
                    }
                }
            }
            
            // Delete the metadata file
            if let Err(e) = fs::remove_file(oldest_metadata_path) {
                tracing::warn!("Failed to delete old recording metadata: {}", e);
            } else {
                tracing::info!("Deleted old recording metadata: {}", oldest_metadata_path.display());
            }
        }

        Ok(())
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
