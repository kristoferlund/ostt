use super::AudioRecorder;
use crate::app_dirs::recordings_dir;
use anyhow::Context;
use std::path::PathBuf;

pub(crate) fn save_recording(
    audio_recorder: &mut AudioRecorder,
    output_format: &str,
) -> anyhow::Result<Option<PathBuf>> {
    tracing::debug!("Stopping recording and saving audio...");

    let extension = recording_file_extension(output_format);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let filename = format!("ostt-recording-{timestamp}.{extension}");
    let filepath = recordings_dir()
        .context("failed to get recordings directory")?
        .join(&filename);

    audio_recorder
        .stop_recording(Some(filepath.clone()), output_format)
        .inspect_err(|e| {
            tracing::error!("Failed to save recording: {}", e);
        })
        .context("failed to stop recorder and encode audio")?;

    if !filepath.exists() {
        tracing::warn!("Recording stopped before any audio was captured");
        return Ok(None);
    }

    tracing::info!("Recording saved to: {}", filepath.display());
    Ok(Some(filepath))
}

fn recording_file_extension(output_format: &str) -> &str {
    match output_format.split_whitespace().next().unwrap_or("mp3") {
        "libopus" | "libvorbis" => "ogg",
        "flac" => "flac",
        "aac" => "m4a",
        "pcm_s16le" => "wav",
        codec => codec,
    }
}
