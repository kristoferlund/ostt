//! Recording history management for retry and replay functionality.
//!
//! Manages audio recording files stored in the recordings directory.
//! Files are named with timestamps for automatic chronological sorting.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages recording history for retry and replay functionality.
pub struct RecordingHistory {
    /// Path to the recordings directory
    recordings_dir: PathBuf,
}

impl RecordingHistory {
    /// Creates a new recording history manager.
    pub fn new(data_dir: &Path) -> Result<Self> {
        let recordings_dir = data_dir.join("recordings");
        fs::create_dir_all(&recordings_dir)?;
        Ok(Self { recordings_dir })
    }

    /// Cleans up old recordings to keep only the 10 most recent.
    /// 
    /// Should be called before saving a new recording.
    pub fn cleanup_old_recordings(&self) -> Result<()> {
        let mut recordings = self.list_recording_files()?;

        // If we have 10 or more recordings, delete the oldest to make room
        if recordings.len() >= 10 {
            // Sort by filename (which includes timestamp, so older files come first)
            recordings.sort();
            let oldest = &recordings[0];
            
            if let Err(e) = fs::remove_file(oldest) {
                tracing::warn!("Failed to delete old recording: {}", e);
            } else {
                tracing::info!("Deleted old recording: {}", oldest.display());
            }
        }

        Ok(())
    }

    /// Lists all recording files in chronological order (oldest first).
    fn list_recording_files(&self) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(&self.recordings_dir)?;
        let mut recordings: Vec<PathBuf> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                // Only include files that start with "ostt-recording-"
                if path.is_file() && path.file_name()?.to_str()?.starts_with("ostt-recording-") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        
        recordings.sort();
        Ok(recordings)
    }

    /// Retrieves all recordings ordered by most recent first.
    pub fn get_all_recordings(&self) -> Result<Vec<PathBuf>> {
        let mut recordings = self.list_recording_files()?;
        recordings.reverse(); // Most recent first
        Ok(recordings)
    }
}
