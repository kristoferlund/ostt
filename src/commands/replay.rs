//! Replay a previous recording from history using the system audio player.

use crate::recording::RecordingHistory;
use std::process::Command;
use dirs;

/// Plays back a previous recording using the system's default audio player.
///
/// On macOS: Uses `open` command to open with default application
/// On Linux: Tries xdg-open first, then falls back to common audio players (mpv, vlc, ffplay, paplay)
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

    let recording = &all_recordings[index - 1];
    let audio_path = &recording.audio_path;

    if !audio_path.exists() {
        return Err(anyhow::anyhow!(
            "Audio file not found: {}",
            audio_path.display()
        ));
    }

    tracing::info!(
        "Playing recording #{} from {}",
        index,
        recording.created_at.format("%Y-%m-%d %H:%M:%S")
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
        let result = Command::new("xdg-open")
            .arg(audio_path)
            .spawn();

        match result {
            Ok(mut child) => {
                child
                    .wait()
                    .map_err(|e| anyhow::anyhow!("Audio player error: {e}"))?;
            }
            Err(_) => {
                // Fallback to common audio players if xdg-open fails
                let players = vec!["mpv", "vlc", "ffplay", "paplay"];
                let mut played = false;

                for player in players {
                    if let Ok(mut child) = Command::new(player).arg(audio_path).spawn() {
                        let _ = child.wait();
                        played = true;
                        break;
                    }
                }

                if !played {
                    return Err(anyhow::anyhow!(
                        "No audio player found. Install mpv, vlc, ffplay, or paplay"
                    ));
                }
            }
        }
    }

    tracing::info!("Playback finished for recording #{}", index);
    Ok(())
}
