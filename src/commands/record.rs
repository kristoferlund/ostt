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
use ratatui::widgets::ListState;
use signal_hook::consts::SIGUSR1;
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

impl Drop for RecordingPidGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn write_recording_pid_file() -> anyhow::Result<RecordingPidGuard> {
    let path = crate::app_dirs::recording_pid_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, std::process::id().to_string())?;
    Ok(RecordingPidGuard { path })
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
    audio_recorder.start_recording().map_err(|err| {
        tracing::error!("Failed to start recording: {err}");
        anyhow::anyhow!(
            "Recording error: {err}\n\nPlease check your audio configuration and try again."
        )
    })?;
    let actual_sample_rate = audio_recorder.sample_rate();

    let mut tui = RecordingTui::new(config, actual_sample_rate)
        .map_err(|e| anyhow::anyhow!("Failed to initialize UI: {e}"))?;
    let term = Arc::new(AtomicBool::new(false));
    let term_clone = term.clone();

    // Subscribe to SIGUSR1 signal
    signal_hook::flag::register(SIGUSR1, term_clone)
        .inspect_err(|_| {
            tui.cleanup().ok();
        })
        .map_err(|e| anyhow::anyhow!("Failed to register signal handler: {e}"))?;

    // Remember the id of the current process
    let recording_pid_guard = write_recording_pid_file()
        .inspect_err(|_| {
            tui.cleanup().ok();
        })
        .map_err(|e| anyhow::anyhow!("Failed to write recording PID file: {e}"))?;

    let should_transcribe =
        run_recording_loop(&mut tui, &mut audio_recorder, actual_sample_rate, &term).inspect_err(
            |_| {
                tui.cleanup().ok();
            },
        )?;

    // Dropping the pid also deletes file
    drop(recording_pid_guard);

    let filepath = save_recording(&mut audio_recorder, config).inspect_err(|_| {
        tui.cleanup().ok();
    })?;

    // Max 10 recordings are saved
    recording_history::prune_old_recordings();

    // Transcribe or skip if user pressed Esc etc.
    let maybe_transcribed_text = match should_transcribe {
        true => transcribe_with_animation(&mut tui, config, model_override, &filepath)
            .await
            .inspect_err(|_| {
                tui.cleanup().ok();
            })?,
        false => None,
    };

    // Process if there is a transcribed text and a process action.
    let output_text = if let Some(transcribed_text) = maybe_transcribed_text {
        Some(if process.is_some() {
            process_with_animation(&mut tui, config, transcribed_text, process.as_deref())
                .await
                .inspect_err(|_| {
                    tui.cleanup().ok();
                })?
        } else {
            transcribed_text
        })
    } else {
        None
    };

    tui.cleanup()
        .map_err(|e| anyhow::anyhow!("Cleanup failed: {e}"))?;

    if let Some(output_text) = output_text {
        write_record_output(&output_text, output_file, clipboard)?;
    }

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
            anyhow::anyhow!("Input handling error: {e}")
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
        .map_err(|e| anyhow::anyhow!("Render failed: {e}"))
}

fn save_recording(
    audio_recorder: &mut AudioRecorder,
    config: &OsttConfig,
) -> anyhow::Result<PathBuf> {
    tracing::debug!("Stopping recording and saving audio...");

    let extension = recording_file_extension(&config.audio.output_format);
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let filename = format!("ostt-recording-{timestamp}.{extension}");
    let filepath = recordings_dir()?.join(&filename);

    audio_recorder
        .stop_recording(Some(filepath.clone()), &config.audio.output_format)
        .inspect_err(|e| {
            tracing::error!("Failed to save recording: {}", e);
        })?;

    tracing::info!("Recording saved to: {}", filepath.display());
    Ok(filepath)
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
    let transcription_context = common::build_transcription_context(config, model_override)?;
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
    let action = process::select_requested_action(&config.process, process_arg, |actions| {
        pick_action_id_with_recording_tui(tui, actions)
    })?
    .expect("process_recording_with_animation called without a process action");

    run_process_action_with_animation(tui, action, text).await
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
            .map_err(|e| anyhow::anyhow!("{e}"))?
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
    let keywords = common::load_keywords()?;
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
        std::fs::write(&file_path, output_text)?;
        tracing::info!("Transcription written to file: {}", file_path);
    } else if clipboard {
        crate::clipboard::copy_to_clipboard(output_text)?;
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

            common::save_transcription_history(&trimmed_text)?;

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
