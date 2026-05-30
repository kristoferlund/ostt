//! Recording history management for retry and replay functionality.
//!
//! Manages audio recording files stored in the recordings directory.
//! Files are named with timestamps for automatic chronological sorting.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Cleans up old recordings to keep only the 10 most recent.
pub fn cleanup_old_recordings() {
    let mut recordings = match list_recording_files() {
        Ok(recordings) => recordings,
        Err(err) => {
            tracing::warn!("Failed to list recordings for cleanup: {}", err);
            return;
        }
    };

    if recordings.len() >= 10 {
        recordings.sort();
        let oldest = &recordings[0];

        if let Err(e) = fs::remove_file(oldest) {
            tracing::warn!("Failed to delete old recording: {}", e);
        } else {
            tracing::debug!("Deleted old recording: {}", oldest.display());
        }
    }
}

/// Retrieves all recordings ordered by most recent first.
pub fn get_all_recordings() -> Result<Vec<PathBuf>> {
    let mut recordings = list_recording_files()?;
    recordings.reverse();
    Ok(recordings)
}

fn list_recording_files() -> Result<Vec<PathBuf>> {
    let recordings_dir = crate::app_dirs::recordings_dir()?;
    let entries = fs::read_dir(&recordings_dir)?;
    let mut recordings: Vec<PathBuf> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
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
