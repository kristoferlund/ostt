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
    paste: bool,
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

    let mut tui = RecordingTui::new_pending_audio(config)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to initialize recording UI")?;
    run_record_preflight(config, model_override.clone(), param_overrides)
        .context("record preflight failed")
        .map_err(|err| show_recording_error(&mut tui, "Setup Error", err))?;
    let term = Arc::new(AtomicBool::new(false));

    let mut audio_recorder = AudioRecorder::new(config);
    audio_recorder
        .start_recording()
        .context("failed to start audio recording")
        .map_err(|err| show_audio_startup_error(&mut tui, err))?;
    let actual_sample_rate = audio_recorder.sample_rate();
    tui.set_sample_rate(actual_sample_rate);

    // External popup/launcher integrations use SIGUSR1 to finish the active recording.
    let active_recording_guard =
        ActiveRecordingGuard::start(term.clone()).context("failed to mark recorder as active")?;

    // Cancel means discard the in-memory samples; only a transcribe action persists audio.
    if !run_recording_loop(&mut tui, &mut audio_recorder, actual_sample_rate, &term)
        .map_err(|err| show_recording_error(&mut tui, "Recording Error", err))?
    {
        return finish_recording_without_output(&mut tui);
    }

    // Once recording has stopped, external triggers should no longer target this process.
    drop(active_recording_guard);

    let output_format = resolve_recording_output_format(config, model_override.as_ref());
    let Some(filepath) = storage::save_recording(&mut audio_recorder, &output_format)
        .context("failed to save recording")
        .map_err(|err| show_recording_error(&mut tui, "Recording Error", err))?
    else {
        return finish_recording_without_output(&mut tui);
    };

    // Prune only after a real recording was saved so cancellation cannot mutate history.
    recording_history::prune_old_recordings();

    let transcription_context =
        crate::transcription::build_context(config, model_override, param_overrides)
            .context("failed to build transcription context")
            .map_err(|err| show_recording_error(&mut tui, "Transcription Error", err))?;
    let model_id = transcription_context.selected_model.model_id.clone();
    let filepath_str = filepath.to_string_lossy().to_string();

    let maybe_transcribed_text = match transcribe_recording_with_animation(
        &mut tui,
        transcription_context.config,
        &model_id,
        &filepath_str,
    )
    .await
    {
        Ok(text) => {
            let text = crate::text::apply_replace(text.trim(), &config.text.replace)?;
            history::save_transcription(&text).context("failed to save transcription history")?;
            Some(text)
        }
        Err(e) => {
            tracing::warn!("Transcription failed: {}", e);
            let message = format_recording_error(&e);
            if let Err(display_err) = tui.show_error("Transcription Error", &message) {
                tracing::warn!("Failed to show transcription error in TUI: {display_err}");
            }
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
                .context("failed to select process action")
                .map_err(|err| show_recording_error(&mut tui, "Processing Error", err))?
            else {
                return finish_recording_with_output(
                    &mut tui,
                    &transcribed_text,
                    output_file,
                    clipboard,
                    paste,
                    &config.output.paste,
                );
            };

            Some(
                run_process_action_with_animation(&mut tui, action, transcribed_text)
                    .await
                    .context("failed to process transcription")
                    .map_err(|err| show_recording_error(&mut tui, "Processing Error", err))?,
            )
        }
        Some(transcribed_text) => Some(transcribed_text),
        None => None,
    };

    match output_text {
        Some(output_text) => finish_recording_with_output(
            &mut tui,
            &output_text,
            output_file,
            clipboard,
            paste,
            &config.output.paste,
        ),
        None => {
            finish_recording_without_output(&mut tui)?;
            Ok(())
        }
    }
}

fn show_recording_error(
    tui: &mut RecordingTui,
    title: &str,
    error: anyhow::Error,
) -> anyhow::Error {
    let message = format_recording_error(&error);
    show_recording_message(tui, title, &message);
    error
}

fn run_record_preflight(
    config: &OsttConfig,
    model_override: Option<SelectedModel>,
    param_overrides: &[String],
) -> anyhow::Result<()> {
    run_record_preflight_with_ffmpeg_check(
        config,
        model_override,
        param_overrides,
        crate::recording::ffmpeg::find_ffmpeg,
    )
}

fn run_record_preflight_with_ffmpeg_check<F>(
    config: &OsttConfig,
    model_override: Option<SelectedModel>,
    param_overrides: &[String],
    find_ffmpeg: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<std::path::PathBuf>,
{
    let preflight_context =
        crate::transcription::build_preflight_context(config, model_override, param_overrides)?;
    if preflight_context.selected_model.provider_id == "whisper" {
        crate::transcription::local_models::resolve_installed_model_path(
            &preflight_context.selected_model.model_id,
        )
        .map_err(|err| {
            anyhow::anyhow!(
                "local model 'whisper/{}' is unavailable: {err}",
                preflight_context.selected_model.model_id
            )
        })?;
    }

    let output_format =
        resolve_recording_output_format(config, Some(&preflight_context.selected_model));
    find_ffmpeg().map_err(|err| {
        anyhow::anyhow!("ffmpeg is required to save recordings as '{output_format}': {err}")
    })?;

    Ok(())
}

fn show_audio_startup_error(tui: &mut RecordingTui, error: anyhow::Error) -> anyhow::Error {
    let message = format_recording_error(&error);
    show_recording_message(tui, "Audio Device Error", &message);
    crate::notifier::notify_error(
        "Audio Device Error",
        &audio_startup_notification_body(&error),
    );
    error
}

fn audio_startup_notification_body(error: &anyhow::Error) -> String {
    let searchable = error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    audio_startup_next_step(&searchable)
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string())
}

fn show_recording_message(tui: &mut RecordingTui, title: &str, message: &str) {
    if let Err(display_err) = tui.show_error(title, message) {
        tracing::warn!("Failed to show recording error in TUI: {display_err}");
    }
}

fn format_recording_error(error: &anyhow::Error) -> String {
    let primary = error.to_string();
    let details = error
        .chain()
        .skip(1)
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let mut message = format!("Primary error: {primary}");

    if !details.is_empty() {
        message.push_str("\n\nCaused by: ");
        message.push_str(&details.join(" -> "));

        if details.len() > 1 {
            if let Some(root_cause) = details.last() {
                message.push_str("\nRoot cause: ");
                message.push_str(root_cause);
            }
        }
    }

    let searchable = if details.is_empty() {
        primary
    } else {
        format!("{primary}\n{}", details.join("\n"))
    };

    if let Some(next_step) = record_error_next_step(&searchable) {
        message.push_str("\n\nNext step: ");
        message.push_str(next_step);
    }

    message
}

fn record_error_next_step(error_text: &str) -> Option<&'static str> {
    audio_startup_next_step(error_text)
}

fn audio_startup_next_step(error_text: &str) -> Option<&'static str> {
    let normalized = error_text.to_ascii_lowercase();

    if cfg!(target_os = "macos")
        && (normalized.contains("permission")
            || normalized.contains("access denied")
            || normalized.contains("permission denied")
            || normalized.contains("not permitted"))
    {
        return Some(
            "Grant microphone access to your terminal app in System Settings > Privacy & Security > Microphone, then retry.",
        );
    }

    if normalized.contains("no longer available") {
        return Some("The default audio device may not support input. Run 'ostt config list-devices' to find your input device, then set it with 'ostt config'.");
    }

    if normalized.contains("no audio input device") {
        return Some("Connect or enable a microphone, then retry.");
    }

    if normalized.contains("audio input device") && normalized.contains("not found") {
        return Some("Run 'ostt config list-devices' and update [audio].device, then retry.");
    }

    if normalized.contains("input device configuration") {
        return Some("Select a working input device with 'ostt config list-devices', then retry.");
    }

    if normalized.contains("audio input stream") {
        return Some(
            "Check that your microphone is available and not blocked by another app, then retry.",
        );
    }

    None
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
    paste: bool,
    paste_config: &crate::config::PasteConfig,
) -> anyhow::Result<()> {
    tui.cleanup()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to clean up recording UI")?;

    write_record_output(output_text, output_file, clipboard, paste, paste_config)
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
    paste: bool,
    paste_config: &crate::config::PasteConfig,
) -> anyhow::Result<()> {
    if paste {
        crate::paste::spawn_detached_paste_helper(output_text).inspect_err(|err| {
            crate::notifier::notify_error_if_popup_context("Paste Failed", &err.to_string());
        })?;
        tracing::debug!("Transcription sent to detached paste helper");
        return Ok(());
    }

    super::output::write_text(
        output_text,
        output_file,
        clipboard,
        paste,
        paste_config,
        "Transcription",
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderConfig, ProviderModelConfig, ProviderSettings};
    use indexmap::IndexMap;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, ffi::OsString, fs, path::PathBuf};

    struct EnvGuard {
        home: Option<OsString>,
        xdg_config_home: Option<OsString>,
        xdg_data_home: Option<OsString>,
        dir: PathBuf,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            crate::transcription::local_models::set_test_models_dir(None);
            restore_env("HOME", self.home.clone());
            restore_env("XDG_CONFIG_HOME", self.xdg_config_home.clone());
            restore_env("XDG_DATA_HOME", self.xdg_data_home.clone());
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn restore_env(key: &str, value: Option<OsString>) {
        if let Some(value) = value {
            env::set_var(key, value);
        } else {
            env::remove_var(key);
        }
    }

    fn isolated_env() -> EnvGuard {
        let home = env::var_os("HOME");
        let xdg_config_home = env::var_os("XDG_CONFIG_HOME");
        let xdg_data_home = env::var_os("XDG_DATA_HOME");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ostt-record-preflight-test-{unique}"));
        crate::transcription::local_models::set_test_models_dir(Some(dir.join("models")));
        env::set_var("HOME", &dir);
        env::set_var("XDG_CONFIG_HOME", dir.join(".config"));
        env::set_var("XDG_DATA_HOME", dir.join(".local").join("share"));

        EnvGuard {
            home,
            xdg_config_home,
            xdg_data_home,
            dir,
        }
    }

    fn ffmpeg_ok() -> anyhow::Result<PathBuf> {
        Ok(PathBuf::from("/usr/bin/ffmpeg"))
    }

    fn command_config() -> OsttConfig {
        let mut config = OsttConfig::default();
        config.transcription.provider = Some("command".to_string());
        config.transcription.model = Some("test-profile".to_string());
        config.provider_configs.insert(
            "command".to_string(),
            ProviderConfig {
                models: IndexMap::from([(
                    "test-profile".to_string(),
                    ProviderModelConfig {
                        settings: ProviderSettings {
                            command: Some(
                                "definitely-not-an-installed-ostt-test-command {audio_path}"
                                    .to_string(),
                            ),
                            ..ProviderSettings::default()
                        },
                        ..ProviderModelConfig::default()
                    },
                )]),
                ..ProviderConfig::default()
            },
        );
        config
    }

    fn http_config() -> OsttConfig {
        let mut config = OsttConfig::default();
        config.transcription.provider = Some("http".to_string());
        config.transcription.model = Some("test-profile".to_string());
        config.provider_configs.insert(
            "http".to_string(),
            ProviderConfig {
                models: IndexMap::from([(
                    "test-profile".to_string(),
                    ProviderModelConfig {
                        settings: ProviderSettings {
                            endpoint: Some("http://127.0.0.1:9/transcribe".to_string()),
                            ..ProviderSettings::default()
                        },
                        ..ProviderModelConfig::default()
                    },
                )]),
                ..ProviderConfig::default()
            },
        );
        config
    }

    #[test]
    fn audio_startup_error_includes_primary_details_and_next_step() {
        let error = anyhow::anyhow!("No audio input device available")
            .context("failed to start audio recording");

        let message = format_recording_error(&error);

        assert!(message.contains("Primary error: failed to start audio recording"));
        assert!(message.contains("Caused by: No audio input device available"));
        assert!(message.contains("Next step: Connect or enable a microphone, then retry."));
    }

    #[test]
    fn recording_error_includes_cause_chain_and_root_cause() {
        let error = anyhow::anyhow!("ffmpeg exited with code 1")
            .context("audio encoding failed")
            .context("failed to stop recorder and encode audio")
            .context("failed to save recording");

        let message = format_recording_error(&error);

        assert!(message.contains("Primary error: failed to save recording"));
        assert!(message.contains(
            "Caused by: failed to stop recorder and encode audio -> audio encoding failed -> ffmpeg exited with code 1"
        ));
        assert!(message.contains("Root cause: ffmpeg exited with code 1"));
    }

    #[test]
    fn audio_startup_error_guides_configured_device_not_found() {
        let error = anyhow::anyhow!(
            "Audio input device 'Missing Mic' not found. Use 'ostt config list-devices' to see available devices."
        )
        .context("failed to start audio recording");

        let message = format_recording_error(&error);

        assert!(message.contains("Run 'ostt config list-devices' and update [audio].device"));
    }

    #[test]
    fn audio_startup_error_mentions_macos_microphone_permission_when_likely() {
        let error = anyhow::anyhow!("permission denied")
            .context("Failed to create audio input stream")
            .context("failed to start audio recording");

        let message = format_recording_error(&error);

        if cfg!(target_os = "macos") {
            assert!(message.contains(
                "Grant microphone access to your terminal app in System Settings > Privacy & Security > Microphone, then retry."
            ));
        } else {
            assert!(!message.contains("Privacy & Security > Microphone"));
        }
    }

    #[test]
    fn record_preflight_reports_no_selected_model_before_ffmpeg_check() {
        let config = OsttConfig::default();

        let err = run_record_preflight_with_ffmpeg_check(&config, None, &[], ffmpeg_ok)
            .unwrap_err()
            .to_string();

        assert!(err.contains("No transcription model selected"));
        assert!(err.contains("ostt auth"));
        assert!(err.contains("ostt model"));
    }

    #[test]
    fn record_preflight_reports_missing_cloud_api_key() {
        let _guard = crate::transcription::local_models::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _env = isolated_env();
        let mut config = OsttConfig::default();
        config.transcription.provider = Some("openai".to_string());
        config.transcription.model = Some("gpt-4o-transcribe".to_string());

        let err = run_record_preflight_with_ffmpeg_check(&config, None, &[], ffmpeg_ok)
            .unwrap_err()
            .to_string();

        assert!(err.contains("No API key for OpenAI"));
        assert!(err.contains("ostt auth"));
        assert!(err.contains("ostt model"));
    }

    #[test]
    fn record_preflight_reports_missing_local_model_file() {
        let _guard = crate::transcription::local_models::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _env = isolated_env();
        let mut config = OsttConfig::default();
        config.transcription.provider = Some("whisper".to_string());
        config.transcription.model = Some("missing-local-model".to_string());

        let err = run_record_preflight_with_ffmpeg_check(&config, None, &[], ffmpeg_ok)
            .unwrap_err()
            .to_string();

        assert!(err.contains("local model 'whisper/missing-local-model' is unavailable"));
        assert!(err.contains("not downloaded"));
    }

    #[test]
    fn record_preflight_does_not_probe_custom_provider_runtime_targets() {
        let command_config = command_config();
        run_record_preflight_with_ffmpeg_check(&command_config, None, &[], ffmpeg_ok)
            .expect("command provider should not probe executable availability");

        let http_config = http_config();
        run_record_preflight_with_ffmpeg_check(&http_config, None, &[], ffmpeg_ok)
            .expect("http provider should not probe endpoint reachability");
    }

    #[test]
    fn record_preflight_reports_missing_ffmpeg() {
        let config = command_config();

        let err = run_record_preflight_with_ffmpeg_check(&config, None, &[], || {
            Err(anyhow::anyhow!("ffmpeg not found. Please install ffmpeg."))
        })
        .unwrap_err()
        .to_string();

        assert!(err.contains("ffmpeg is required to save recordings as"));
        assert!(err.contains("ffmpeg not found"));
    }
}
