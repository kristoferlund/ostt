//! Audio recording and transcription.
//!
//! Handles audio recording with real-time waveform visualization, optional transcription,
//! and history management. Supports external triggers via SIGUSR1 signal.

use crate::config::{OsttConfig, ProcessAction, SelectedModel};
use crate::history;
use crate::keywords;
use crate::process::{self, process_view::PickerResult};
use crate::recording::{
    active::ActiveRecordingGuard, recording_history, storage, AudioRecorder, RecordingCommand,
    RecordingTui,
};
use crate::transcription::TranscriptionAnimation;
use crate::ui::cancel_requested;
use anyhow::Context;
use ratatui::widgets::ListState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Handles audio recording and optional transcription.
///
/// Records audio with real-time waveform visualization, optionally transcribes the recording,
/// and saves to history. Supports external triggers via SIGUSR1 signal.
pub async fn handle_record(
    config: &OsttConfig,
    clipboard: bool,
    output_file: Option<String>,
    process: Option<String>,
    model_override: Option<SelectedModel>,
    param_overrides: &[String],
) -> anyhow::Result<()> {
    tracing::info!("=== ostt Audio Recorder Started ===");
    tracing::info!(
        "Configuration loaded: device={}, peak_threshold={}%, reference_level={}dBFS",
        config.audio.device,
        config.audio.peak_volume_threshold,
        config.audio.reference_level_db
    );

    let mut audio_recorder = AudioRecorder::new(config);
    audio_recorder
        .start_recording()
        .context("failed to start audio recording")?;
    let actual_sample_rate = audio_recorder.sample_rate();

    // The UI depends on the device's actual sample rate for timing and spectrum analysis.
    let mut tui = RecordingTui::new(config, actual_sample_rate)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to initialize recording UI")?;
    let term = Arc::new(AtomicBool::new(false));

    // External popup/launcher integrations use SIGUSR1 to finish the active recording.
    let active_recording_guard =
        ActiveRecordingGuard::start(term.clone()).context("failed to mark recorder as active")?;

    // Cancel means discard the in-memory samples; only a transcribe action persists audio.
    if !run_recording_loop(&mut tui, &mut audio_recorder, actual_sample_rate, &term)
        .context("recording loop failed")?
    {
        return finish_recording_without_output(&mut tui);
    }

    // Once recording has stopped, external triggers should no longer target this process.
    drop(active_recording_guard);

    let output_format = resolve_recording_output_format(config, model_override.as_ref());
    let Some(filepath) = storage::save_recording(&mut audio_recorder, &output_format)
        .context("failed to save recording")?
    else {
        return finish_recording_without_output(&mut tui);
    };

    // Prune only after a real recording was saved so cancellation cannot mutate history.
    recording_history::prune_old_recordings();

    let transcription_context =
        crate::transcription::build_context(config, model_override, param_overrides)
            .context("failed to build transcription context")?;
    let model_id = transcription_context.selected_model.model_id.clone();
    let filepath_str = filepath.to_string_lossy().to_string();

    let mut transcription_error = None;
    let maybe_transcribed_text = match transcribe_recording_with_animation(
        &mut tui,
        transcription_context.config,
        &model_id,
        &filepath_str,
    )
    .await
    {
        Ok(text) => {
            let text = crate::text::apply_replacements(text.trim(), &config.text.replacements)?;
            history::save_transcription(&text).context("failed to save transcription history")?;
            Some(text)
        }
        Err(e) => {
            tracing::warn!("Transcription failed: {}", e);
            transcription_error = Some(e.to_string());
            None
        }
    };

    // A transcription failure is non-fatal for record mode; it still leaves the audio in history.
    let output_text = match maybe_transcribed_text {
        Some(transcribed_text) if process.is_some() => {
            let Some(action) =
                process::select_requested_action(&config.process, process.as_deref(), |actions| {
                    pick_action_id_with_recording_tui(&mut tui, actions)
                })
                .context("failed to select process action")?
            else {
                return finish_recording_with_output(
                    &mut tui,
                    &transcribed_text,
                    output_file,
                    clipboard,
                );
            };

            Some(
                run_process_action_with_animation(&mut tui, action, transcribed_text)
                    .await
                    .context("failed to process transcription")?,
            )
        }
        Some(transcribed_text) => Some(transcribed_text),
        None => None,
    };

    match output_text {
        Some(output_text) => {
            finish_recording_with_output(&mut tui, &output_text, output_file, clipboard)
        }
        None => {
            finish_recording_without_output(&mut tui)?;
            if let Some(error) = transcription_error {
                eprintln!("Warning: Transcription failed: {error}");
            }
            Ok(())
        }
    }
}

fn resolve_recording_output_format(
    config: &OsttConfig,
    model_override: Option<&SelectedModel>,
) -> String {
    let selected_model = model_override.cloned().or_else(|| {
        match (
            config.transcription.provider.as_ref(),
            config.transcription.model.as_ref(),
        ) {
            (Some(provider_id), Some(model_id)) => Some(SelectedModel {
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
            }),
            _ => None,
        }
    });

    selected_model
        .as_ref()
        .map(|selected_model| crate::config::resolve_output_format(config, selected_model))
        .unwrap_or_else(|| config.audio.output_format.clone())
}

fn finish_recording_without_output(tui: &mut RecordingTui) -> anyhow::Result<()> {
    tui.cleanup()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to clean up recording UI")?;
    tracing::info!("=== ostt Audio Recorder Exited Successfully ===");
    Ok(())
}

fn finish_recording_with_output(
    tui: &mut RecordingTui,
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
) -> anyhow::Result<()> {
    tui.cleanup()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to clean up recording UI")?;

    write_record_output(output_text, output_file, clipboard)
        .context("failed to write recording output")?;

    tracing::info!("=== ostt Audio Recorder Exited Successfully ===");
    Ok(())
}

fn run_recording_loop(
    tui: &mut RecordingTui,
    audio_recorder: &mut AudioRecorder,
    actual_sample_rate: u32,
    term: &AtomicBool,
) -> anyhow::Result<bool> {
    tracing::debug!(
        "Entering recording loop. Press 'Enter' to transcribe or 'Escape'/'q' to cancel."
    );

    let mut frame_count = 0u64;
    loop {
        if term.load(Ordering::Relaxed) {
            tracing::debug!("Received SIGUSR1: transcribing via external trigger");
            return Ok(true);
        }

        match tui.handle_input().map_err(|e| {
            tracing::error!("Input handling error: {}", e);
            anyhow::anyhow!(e.to_string())
        })? {
            RecordingCommand::Continue => {
                frame_count += 1;
                if frame_count.is_multiple_of(60) {
                    let sample_count = audio_recorder.sample_count();
                    let duration_secs = sample_count as f32 / actual_sample_rate as f32;
                    tracing::debug!("Recording: {:.1}s recorded", duration_secs);
                }

                tui.render_waveform(&audio_recorder.samples())
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
                    .context("failed to render recording waveform")?;
            }
            RecordingCommand::Transcribe => return Ok(true),
            RecordingCommand::Cancel => return Ok(false),
            RecordingCommand::TogglePause => {
                audio_recorder.toggle_pause();
                tui.is_paused = audio_recorder.is_paused();
                tui.render_waveform(&audio_recorder.samples())
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
                    .context("failed to render recording waveform")?;
            }
        }
    }
}

fn pick_action_id_with_recording_tui(
    tui: &mut RecordingTui,
    actions: &[ProcessAction],
) -> anyhow::Result<Option<String>> {
    if actions.is_empty() {
        return Err(anyhow::anyhow!(
            "No process actions configured. Add actions to ~/.config/ostt/ostt.toml"
        ));
    }

    if actions.len() == 1 {
        return Ok(Some(actions[0].id.clone()));
    }

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        match tui
            .render_action_picker(actions, &mut list_state)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .context("failed to render process action picker")?
        {
            Some(PickerResult::Selected(id)) => return Ok(Some(id)),
            Some(PickerResult::Cancelled) => return Ok(None),
            None => continue,
        }
    }
}

async fn run_process_action_with_animation(
    tui: &mut RecordingTui,
    action: ProcessAction,
    text: String,
) -> anyhow::Result<String> {
    let keywords = keywords::load_keywords().context("failed to load keywords")?;
    let mut animation = TranscriptionAnimation::new(80);
    animation.set_status_label("Processing...");

    let task_text = text.clone();
    let task_handle =
        tokio::spawn(async move { process::execute_action(&action, &task_text, &keywords).await });

    loop {
        if let Err(e) = tui.render_transcription_animation(&mut animation) {
            tracing::warn!("Failed to render animation: {}", e);
        }

        if task_handle.is_finished() {
            break;
        }

        if cancel_requested() {
            tracing::info!("Processing cancelled by user");
            task_handle.abort();
            return Ok(text);
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    match task_handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow::anyhow!("Processing task failed: {e}")),
    }
}

fn write_record_output(
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
) -> anyhow::Result<()> {
    if let Some(file_path) = output_file {
        std::fs::write(&file_path, output_text)
            .with_context(|| format!("failed to write output file: {file_path}"))?;
        tracing::info!("Transcription written to file: {}", file_path);
    } else if clipboard {
        crate::clipboard::copy_to_clipboard(output_text)
            .context("failed to copy output to clipboard")?;
        tracing::info!("Transcription copied to clipboard");
    } else {
        println!("{output_text}");
        tracing::debug!("Transcription printed to stdout");
    }

    Ok(())
}

async fn transcribe_recording_with_animation(
    tui: &mut RecordingTui,
    transcription_config: crate::transcription::TranscriptionConfig,
    model_id: &str,
    audio_filename: &str,
) -> anyhow::Result<String> {
    use crate::transcription;

    tracing::debug!(
        "Starting transcription with model '{}' for file '{}'",
        model_id,
        audio_filename
    );

    let mut animation = TranscriptionAnimation::new(80);
    animation.set_status_label("Transcribing...");

    let filename = audio_filename.to_string();
    let transcription_handle = tokio::spawn(async move {
        transcription::transcribe(&transcription_config, filename.as_ref()).await
    });

    let mut cancelled = false;
    loop {
        if let Err(e) = tui.render_transcription_animation(&mut animation) {
            tracing::warn!("Failed to render animation: {}", e);
        }

        if transcription_handle.is_finished() {
            break;
        }

        if cancel_requested() {
            tracing::info!("Transcription cancelled by user");
            transcription_handle.abort();
            cancelled = true;
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    if cancelled {
        return Err(anyhow::anyhow!("Transcription cancelled"));
    }

    match transcription_handle.await {
        Ok(Ok(text)) => {
            let trimmed_text = text.trim().to_string();
            tracing::debug!("Transcription completed: {}", trimmed_text);

            // Return the transcription text to be output after TUI cleanup
            Ok(trimmed_text)
        }
        Ok(Err(e)) => {
            tracing::error!("Transcription failed: {}", e);
            Err(e)
        }
        Err(e) => {
            tracing::error!("Transcription task failed: {}", e);
            Err(anyhow::anyhow!("Transcription task failed: {e}"))
        }
    }
}
