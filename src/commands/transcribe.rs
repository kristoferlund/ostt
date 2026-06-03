//! Transcribe a pre-recorded audio file without recording.
//!
//! Accepts an audio file path and transcribes it using the configured provider/model,
//! reusing the same transcription pipeline as the `record` command.

use super::output;
use crate::process;
use crate::{config, history, transcription};
use std::path::PathBuf;

/// Handles transcription of a pre-recorded audio file.
///
/// Transcribes the given audio file using the currently configured provider and model.
/// Supports the same output options as `record` and `retry`.
///
/// # Arguments
/// * `file` - Path to the audio file to transcribe
/// * `clipboard` - If true, copy to clipboard instead of stdout
/// * `output_file` - Optional file path to write output to instead of stdout
/// * `process` - Optional processing action: None = no processing, Some("") = show picker, Some(id) = use action
pub async fn handle_transcribe(
    config_data: &config::OsttConfig,
    file: PathBuf,
    clipboard: bool,
    output_file: Option<String>,
    process: Option<String>,
    model_override: Option<config::SelectedModel>,
    param_overrides: &[String],
) -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Transcribe Command ===");

    // Validate the input file exists
    if !file.exists() {
        return Err(anyhow::anyhow!("Audio file not found: {}", file.display()));
    }

    tracing::info!("Transcribing file: {}", file.display());

    let context = transcription::build_context(config_data, model_override, param_overrides)?;

    // Transcribe
    tracing::debug!("Starting transcription...");
    let text = transcription::transcribe(&context.config, &file)
        .await
        .map_err(|e| {
            tracing::error!("Transcription failed: {e}");
            anyhow::anyhow!("Transcription failed: {e}")
        })?;

    let transcription_text =
        crate::text::apply_replacements(text.trim(), &config_data.text.replacements)?;
    tracing::debug!("Transcription completed: {}", transcription_text);

    history::save_transcription(&transcription_text)?;
    let output_text = process::apply_requested_action(
        &config_data.process,
        &transcription_text,
        &context.keywords,
        process.as_deref(),
    )
    .await?;
    output::write_text(&output_text, output_file, clipboard, "Output text")?;

    Ok(())
}
