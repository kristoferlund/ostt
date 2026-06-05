//! Retry transcription of a previous recording without re-recording audio.

use super::output;
use crate::process;
use crate::recording::recording_history;
use crate::{config, history, transcription};

/// Retries transcription of a previous recording.
///
/// Allows users to re-transcribe a recording with the same or different settings.
/// Useful when transcription failed due to network issues, API key problems, etc.
///
/// # Arguments
/// * `recording_index` - Optional index of recording to retry (1 = most recent, None = most recent)
/// * `clipboard` - If true, copy to clipboard instead of stdout
/// * `output_file` - Optional file path to write output to instead of stdout
/// * `process` - Optional processing action: None = no processing, Some("") = show picker, Some(id) = use action
pub async fn handle_retry(
    config_data: &config::OsttConfig,
    options: RetryOptions<'_>,
) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Retry Command ===");

    let RetryOptions {
        recording_index,
        clipboard,
        paste,
        output_file,
        process,
        model_override,
        param_overrides,
    } = options;

    let all_recordings = recording_history::get_all_recordings()?;

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

    tracing::info!("Retrying transcription for recording #{}", index);

    let context = transcription::build_context(config_data, model_override, param_overrides)?;

    // Transcribe
    tracing::debug!("Starting transcription for retry...");
    match transcription::transcribe(&context.config, audio_path).await {
        Ok(text) => {
            let transcription_text =
                crate::text::apply_replace(text.trim(), &config_data.text.replace)?;
            tracing::debug!("Retry transcription completed: {}", transcription_text);

            history::save_transcription(&transcription_text)?;
            let output_text = process::apply_requested_action(
                &config_data.process,
                &transcription_text,
                &context.keywords,
                process.as_deref(),
            )
            .await?;
            output::write_text(
                &output_text,
                output_file,
                clipboard,
                paste,
                &config_data.output.paste,
                "Output text",
            )?;

            Ok(())
        }
        Err(e) => {
            tracing::error!("Retry transcription failed: {e}");
            Err(anyhow::anyhow!("Transcription failed: {e}"))
        }
    }
}

pub struct RetryOptions<'a> {
    pub recording_index: Option<usize>,
    pub clipboard: bool,
    pub paste: bool,
    pub output_file: Option<String>,
    pub process: Option<String>,
    pub model_override: Option<config::SelectedModel>,
    pub param_overrides: &'a [String],
}
