//! Replay a previous recording from history using the system audio player.

use crate::recording::RecordingHistory;
use dirs;
use std::process::Command;

/// Plays back a previous recording using the system's best available audio player.
///
/// On macOS: Uses `open` command to open with default application
/// On Linux: Tries dedicated audio players first (mpv, vlc, ffplay, paplay) for better UX,
///           then falls back to xdg-open if none are available
///
/// # Arguments
/// * `recording_index` - Optional index of recording to play (1 = most recent, None = most recent)
pub async fn handle_replay(recording_index: Option<usize>) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Replay Command ===");

    let data_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local")
        .join("share")
        .join("ostt");

    let history = RecordingHistory::new(&data_dir)?;
    let all_recordings = history.get_all_recordings()?;

    if all_recordings.is_empty() {
        return Err(anyhow::anyhow!("No recordings found in history"));
    }

    // Get recording by index (1-indexed, where 1 is most recent)
    let index = recording_index.unwrap_or(1);
    if index < 1 || index > all_recordings.len() {
        return Err(anyhow::anyhow!(
            "Recording index out of range. Available recordings: 1-{}",
            all_recordings.len()
        ));
    }

    let audio_path = &all_recordings[index - 1];

    if !audio_path.exists() {
        return Err(anyhow::anyhow!(
            "Audio file not found: {}",
            audio_path.display()
        ));
    }

    tracing::info!(
        "Playing recording #{}",
        index
    );
    tracing::info!(
        "Audio file path: {}",
        audio_path.display()
    );

    // Platform-specific audio player invocation
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(audio_path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to open audio player: {e}"))?
            .wait()
            .map_err(|e| anyhow::anyhow!("Audio player error: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try dedicated audio players first (prefer nice UI), then fall back to xdg-open
        let players = vec!["mpv", "vlc", "ffplay", "paplay"];
        let mut played = false;

        for player in players {
            if let Ok(mut child) = Command::new(player).arg(audio_path).spawn() {
                let _ = child.wait();
                played = true;
                break;
            }
        }

        // If no dedicated player found, try xdg-open as fallback
        if !played {
            if let Ok(mut child) = Command::new("xdg-open").arg(audio_path).spawn() {
                child
                    .wait()
                    .map_err(|e| anyhow::anyhow!("Audio player error: {e}"))?;
                played = true;
            }
        }

        if !played {
            return Err(anyhow::anyhow!(
                "No audio player found. Install mpv, vlc, ffplay, or paplay"
            ));
        }
    }

    tracing::debug!("Playback finished for recording #{}", index);
    Ok(())
}
