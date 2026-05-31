//! Audio recording and transcription.
//!
//! Handles audio recording with real-time waveform visualization, optional transcription,
//! and history management. Supports external triggers via SIGUSR1 signal.

use super::common;
use crate::app_dirs::recordings_dir;
use crate::config::{OsttConfig, ProcessAction, SelectedModel};
use crate::process;
use crate::recording::{
    recording_history, AudioRecorder, PickerEvent, RecordingCommand, RecordingTui,
};
use crate::transcription::TranscriptionAnimation;
use anyhow::Context;
use ratatui::widgets::ListState;
use signal_hook::consts::SIGUSR1;
use signal_hook::low_level::unregister;
use signal_hook::SigId;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct RecordingPidGuard {
    path: PathBuf,
}

struct SignalGuard {
    id: SigId,
}

impl Drop for RecordingPidGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        unregister(self.id);
    }
}

fn write_recording_pid_file() -> anyhow::Result<RecordingPidGuard> {
    let path = crate::app_dirs::recording_pid_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create recording PID directory: {}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&path, std::process::id().to_string())
        .with_context(|| format!("Failed to write PID to {}", path.display()))?;
    Ok(RecordingPidGuard { path })
}

fn register_transcription_signal(term: Arc<AtomicBool>) -> anyhow::Result<SignalGuard> {
    let id = signal_hook::flag::register(SIGUSR1, term)?;
    Ok(SignalGuard { id })
}

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
) -> Result<(), anyhow::Error> {
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
        .context("Failed to start audio recording")?;
    let actual_sample_rate = audio_recorder.sample_rate();

    // The UI depends on the device's actual sample rate for timing and spectrum analysis.
    let mut tui = RecordingTui::new(config, actual_sample_rate)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to initialize recording UI")?;
    let term = Arc::new(AtomicBool::new(false));

    // External popup/launcher integrations use SIGUSR1 to finish the active recording.
    let signal_guard = register_transcription_signal(term.clone())
        .context("Failed to register recording signal handler")?;

    // Other OSTT commands need this PID to trigger transcription of the active recorder.
    let recording_pid_guard =
        write_recording_pid_file().context("Failed to write recording PID file")?;

    // Cancel means discard the in-memory samples; only a transcribe action persists audio.
    if !run_recording_loop(&mut tui, &mut audio_recorder, actual_sample_rate, &term)
        .context("Recording loop failed")?
    {
        return finish_recording_without_output(&mut tui);
    }

    // Once recording has stopped, external triggers should no longer target this process.
    drop(signal_guard);
    drop(recording_pid_guard);

    let Some(filepath) =
        save_recording(&mut audio_recorder, config).context("Failed to save recording")?
    else {
        return finish_recording_without_output(&mut tui);
    };

    // Prune only after a real recording was saved so cancellation cannot mutate history.
    recording_history::prune_old_recordings();

    let maybe_transcribed_text =
        transcribe_with_animation(&mut tui, config, model_override, &filepath)
            .await
            .context("Failed to transcribe recording")?;

    // A transcription failure is non-fatal for record mode; it still leaves the audio in history.
    let output_text = match maybe_transcribed_text {
        Some(transcribed_text) if process.is_some() => Some(
            process_with_animation(&mut tui, config, transcribed_text, process.as_deref())
                .await
                .context("Failed to process transcription")?,
        ),
        Some(transcribed_text) => Some(transcribed_text),
        None => None,
    };

    tui.cleanup()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to clean up recording UI")?;

    if let Some(output_text) = output_text {
        write_record_output(&output_text, output_file, clipboard)
            .context("Failed to write recording output")?;
    }

    tracing::info!("=== ostt Audio Recorder Exited Successfully ===");
    Ok(())
}

fn finish_recording_without_output(tui: &mut RecordingTui) -> Result<(), anyhow::Error> {
    tui.cleanup()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to clean up recording UI")?;
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

                render_recording_waveform(tui, audio_recorder)?;
            }
            RecordingCommand::Transcribe => return Ok(true),
            RecordingCommand::Cancel => return Ok(false),
            RecordingCommand::TogglePause => {
                audio_recorder.toggle_pause();
                tui.is_paused = audio_recorder.is_paused();
                render_recording_waveform(tui, audio_recorder)?;
            }
        }
    }
}

fn render_recording_waveform(
    tui: &mut RecordingTui,
    audio_recorder: &AudioRecorder,
) -> anyhow::Result<()> {
    tui.render_waveform(&audio_recorder.samples())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to render recording waveform")
}

fn save_recording(
    audio_recorder: &mut AudioRecorder,
    config: &OsttConfig,
) -> anyhow::Result<Option<PathBuf>> {
    tracing::debug!("Stopping recording and saving audio...");

    let extension = recording_file_extension(&config.audio.output_format);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let filename = format!("ostt-recording-{timestamp}.{extension}");
    let filepath = recordings_dir()
        .context("Failed to get recordings directory")?
        .join(&filename);

    audio_recorder
        .stop_recording(Some(filepath.clone()), &config.audio.output_format)
        .inspect_err(|e| {
            tracing::error!("Failed to save recording: {}", e);
        })
        .context("Failed to stop recorder and encode audio")?;

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

async fn transcribe_with_animation(
    tui: &mut RecordingTui,
    config: &OsttConfig,
    model_override: Option<SelectedModel>,
    filepath: &std::path::Path,
) -> anyhow::Result<Option<String>> {
    let transcription_context = common::build_transcription_context(config, model_override)
        .context("Failed to build transcription context")?;
    let model_id = transcription_context.selected_model.model_id.clone();
    let filepath_str = filepath.to_string_lossy().to_string();

    match transcribe_recording_with_animation(
        tui,
        transcription_context.config,
        &model_id,
        &filepath_str,
    )
    .await
    {
        Ok(text) => Ok(Some(text)),
        Err(e) => {
            tracing::warn!("Transcription failed: {}", e);
            eprintln!("Warning: Transcription failed: {e}");
            Ok(None)
        }
    }
}

async fn process_with_animation(
    tui: &mut RecordingTui,
    config: &OsttConfig,
    text: String,
    process_arg: Option<&str>,
) -> anyhow::Result<String> {
    let Some(action) = process::select_requested_action(&config.process, process_arg, |actions| {
        pick_action_id_with_recording_tui(tui, actions)
    })
    .context("Failed to select process action")?
    else {
        return Ok(text);
    };

    run_process_action_with_animation(tui, action, text)
        .await
        .context("Failed to run process action")
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
            .context("Failed to render process action picker")?
        {
            Some(PickerEvent::Selected(id)) => return Ok(Some(id)),
            Some(PickerEvent::Cancelled) => return Ok(None),
            None => continue,
        }
    }
}

async fn run_process_action_with_animation(
    tui: &mut RecordingTui,
    action: ProcessAction,
    text: String,
) -> anyhow::Result<String> {
    let keywords = common::load_keywords().context("Failed to load keywords")?;
    let mut animation = TranscriptionAnimation::new(80);
    animation.set_status_label("Processing...");

    let action_clone = action.clone();
    let text_clone = text.clone();
    let keywords_clone = keywords.clone();
    let task_handle = tokio::spawn(async move {
        process::execute_action(&action_clone, &text_clone, &keywords_clone).await
    });

    loop {
        if let Err(e) = tui.render_transcription_animation(&mut animation) {
            tracing::warn!("Failed to render animation: {}", e);
        }

        if task_handle.is_finished() {
            break;
        }

        if processing_cancel_requested() {
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

fn processing_cancel_requested() -> bool {
    if !crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        return false;
    }

    let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() else {
        return false;
    };

    match key.code {
        crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => true,
        crossterm::event::KeyCode::Char('c') => key
            .modifiers
            .contains(crossterm::event::KeyModifiers::CONTROL),
        _ => false,
    }
}

fn write_record_output(
    output_text: &str,
    output_file: Option<String>,
    clipboard: bool,
) -> anyhow::Result<()> {
    if let Some(file_path) = output_file {
        std::fs::write(&file_path, output_text)
            .with_context(|| format!("Failed to write output file: {file_path}"))?;
        tracing::info!("Transcription written to file: {}", file_path);
    } else if clipboard {
        crate::clipboard::copy_to_clipboard(output_text)
            .context("Failed to copy output to clipboard")?;
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

        // Check for cancel input (Escape, 'q', or Ctrl+C)
        if crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                match key.code {
                    crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => {
                        tracing::info!("Transcription cancelled by user");
                        transcription_handle.abort();
                        cancelled = true;
                        break;
                    }
                    crossterm::event::KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        tracing::info!("Transcription cancelled by user (Ctrl+C)");
                        transcription_handle.abort();
                        cancelled = true;
                        break;
                    }
                    _ => {}
                }
            }
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

            common::save_transcription_history(&trimmed_text)
                .context("Failed to save transcription history")?;

            // Return the transcription text to be output after TUI cleanup
            Ok(text)
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
