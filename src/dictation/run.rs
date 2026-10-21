//! The background job process started by `ostt dictate start`.
//!
//! It records from the configured microphone without a terminal, listens on a
//! private Unix socket for stop and cancel, then hands the recording to the
//! regular transcription pipeline and pastes the result.

use super::job::{self, Capture, Control, JobEnd, Limits};
use super::state::{open_lock_file, Paths, Phase, Status, StatusHandle};
use crate::commands::record;
use crate::config::OsttConfig;
use crate::recording::{recording_history, storage, AudioRecorder};
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// Runs the real job: the configured microphone and the regular pipeline.
pub(crate) async fn run_job(
    config: &OsttConfig,
    paths: &Paths,
    job_id: String,
    process: Option<String>,
) -> anyhow::Result<()> {
    host_job(
        paths,
        job_id,
        selected_model_label(config),
        crate::notifier::notify_error,
        |controls, status| record_and_deliver(config, controls, status, process),
    )
    .await
}

/// Hosts one job: takes the job lock, serves the control socket, records the
/// outcome in the status, and raises a notification when the job fails.
/// `body` does the recording and delivery.
pub(crate) async fn host_job<B, Fut>(
    paths: &Paths,
    job_id: String,
    model: Option<String>,
    notify: fn(&str, &str),
    body: B,
) -> anyhow::Result<()>
where
    B: FnOnce(mpsc::UnboundedReceiver<Control>, StatusHandle) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<JobEnd>>,
{
    paths.ensure_dir()?;
    let job_lock = open_lock_file(&paths.job_lock())?;
    match job_lock.try_lock() {
        Ok(()) => {}
        // Leave the state file alone: it belongs to the job that holds the lock.
        Err(std::fs::TryLockError::WouldBlock) => anyhow::bail!("another dictation is active"),
        Err(std::fs::TryLockError::Error(err)) => {
            return Err(err).context("failed to lock the dictation job")
        }
    }
    tracing::info!("dictation: job {job_id} started");

    let status = StatusHandle::new(
        paths.state(),
        Status {
            job: Some(job_id),
            model,
            ..Status::idle()
        },
    );

    let _ = std::fs::remove_file(paths.socket());
    let result = match UnixListener::bind(paths.socket())
        .context("failed to open the dictation control socket")
    {
        Ok(listener) => {
            let (controls_tx, controls_rx) = mpsc::unbounded_channel();
            let server = tokio::spawn(serve_controls(listener, controls_tx, status.clone()));
            let result = body(controls_rx, status.clone()).await;
            server.abort();
            result
        }
        Err(err) => Err(err),
    };
    let _ = std::fs::remove_file(paths.socket());

    let failure = match &result {
        Ok(JobEnd::Failed(message)) => Some(message.clone()),
        Ok(_) => None,
        Err(err) => {
            let message = format!("{err:#}");
            tracing::warn!("dictation: {message}");
            status.set(|s| {
                s.state = Phase::Error;
                s.outcome = None;
                s.error = Some(message.clone());
            });
            Some(message)
        }
    };
    if let Some(message) = failure {
        notify("Dictation failed", &message);
    }
    // The final state is written before the lock is released, so a reader
    // never sees a finished job's active state as current.
    drop(job_lock);
    result.map(drop)
}

async fn record_and_deliver(
    config: &OsttConfig,
    controls: mpsc::UnboundedReceiver<Control>,
    status: StatusHandle,
    process: Option<String>,
) -> anyhow::Result<JobEnd> {
    // Everything that can be checked before recording is checked here, so a
    // missing model or credential fails at the key press, not after speaking.
    record::run_record_preflight(config, None, &[])?;
    let action = match process.as_deref() {
        Some(action_id) => {
            crate::process::select_requested_action(&config.process, Some(action_id), |_| Ok(None))?
        }
        None => None,
    };
    let context = crate::transcription::build_context(config, None, &[])?;
    let output_format = record::resolve_recording_output_format(config, None);

    let mut recorder = AudioRecorder::new(config);
    recorder
        .start_recording()
        .map_err(|err| anyhow::anyhow!(record::audio_startup_notification_body(&err)))
        .context("could not start the microphone")?;
    let mut capture = Microphone {
        recorder,
        output_format,
        started: Instant::now(),
        signal_seen: false,
    };

    let replace_rules = config.text.replace.clone();
    let paste_config = config.output.paste.clone();
    Ok(job::drive(
        &mut capture,
        controls,
        &status,
        Limits::default(),
        move |audio| async move {
            let text = crate::transcription::transcribe(&context.config, &audio)
                .await
                .context("transcription failed")?;
            let text = crate::text::apply_replace(text.trim(), &replace_rules)?;
            match action {
                Some(action) => crate::process::execute_action(&action, &text, &context.keywords)
                    .await
                    .context("processing action failed"),
                None => Ok(text),
            }
        },
        move |_audio, text| {
            crate::history::save_transcription(text).context("failed to save history")?;
            // Prune only once the recording is kept, as the terminal recorder does.
            recording_history::prune_old_recordings();
            if let Some(backend) = omarchy_switched_away() {
                anyhow::bail!(
                    "Not pasted: Omarchy dictation switched to {backend}. The text is in `ostt history`."
                );
            }
            crate::commands::output::write_text(text, None, false, true, &paste_config, "Dictation")
                .context("could not paste the transcript")
        },
        remove_recording,
    )
    .await)
}

/// For a job started by Omarchy's dictation keys: the backend the user
/// switched to since, if it is no longer OSTT. Its result must not be pasted
/// into whatever the user is doing with the other backend.
fn omarchy_switched_away() -> Option<String> {
    std::env::var_os("OSTT_OMARCHY_DICTATION")?;
    let selected = std::fs::read_to_string(crate::commands::omarchy::selection_path()).ok();
    switched_away(selected.as_deref())
}

fn switched_away(selected: Option<&str>) -> Option<String> {
    match selected.map(str::trim) {
        Some("ostt") => None,
        Some("") | None => Some("no backend".to_string()),
        Some(other) => Some(other.to_string()),
    }
}

fn remove_recording(path: &Path) {
    if let Err(err) = std::fs::remove_file(path) {
        tracing::warn!("dictation: failed to remove cancelled recording: {err}");
    }
}

fn selected_model_label(config: &OsttConfig) -> Option<String> {
    match (&config.transcription.provider, &config.transcription.model) {
        (Some(provider), Some(model)) => Some(format!("{provider}/{model}")),
        _ => None,
    }
}

struct Microphone {
    recorder: AudioRecorder,
    output_format: String,
    started: Instant,
    signal_seen: bool,
}

impl Capture for Microphone {
    fn check(&mut self) -> anyhow::Result<()> {
        if let Some(err) = self.recorder.take_stream_error() {
            anyhow::bail!(
                "audio input from '{}' failed: {err}",
                self.recorder.device()
            );
        }
        if !self.signal_seen {
            if self.recorder.first_buffer_frames().is_some() {
                self.signal_seen = true;
            } else if self.started.elapsed() >= record::NO_SIGNAL_TIMEOUT {
                return Err(record::no_signal_error(self.recorder.device()));
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> anyhow::Result<Option<PathBuf>> {
        storage::save_recording(&mut self.recorder, &self.output_format)
    }

    fn discard(&mut self) {
        if let Err(err) = self.recorder.stop_recording(None, &self.output_format) {
            tracing::warn!("dictation: failed to stop the recorder: {err}");
        }
    }
}

/// Answers controllers until the job exits. Each connection sends one
/// request line (`stop`, `cancel` or `status`) and gets the status as JSON.
async fn serve_controls(
    listener: UnixListener,
    controls: mpsc::UnboundedSender<Control>,
    status: StatusHandle,
) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        if let Err(err) = answer(stream, &controls, &status).await {
            tracing::debug!("dictation: control connection failed: {err}");
        }
    }
}

async fn answer(
    stream: UnixStream,
    controls: &mpsc::UnboundedSender<Control>,
    status: &StatusHandle,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut line = String::new();
    tokio::time::timeout(
        Duration::from_secs(2),
        BufReader::new(reader).read_line(&mut line),
    )
    .await
    .context("control request timed out")??;
    let control = match line.trim() {
        "stop" => Some(Control::Stop),
        "cancel" => Some(Control::Cancel),
        "status" => None,
        other => anyhow::bail!("unknown control request '{other}'"),
    };
    // Report the state the request was made in; the job acts on it next.
    let reply = serde_json::to_string(&status.get())?;
    if let Some(control) = control {
        let _ = controls.send(control);
    }
    writer.write_all(reply.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

/// Sends one control request to the running job and returns its status.
pub(crate) async fn send_control(paths: &Paths, request: &str) -> anyhow::Result<Status> {
    let stream = UnixStream::connect(paths.socket())
        .await
        .context("failed to reach the dictation job")?;
    let (reader, mut writer) = stream.into_split();
    writer.write_all(format!("{request}\n").as_bytes()).await?;
    let mut line = String::new();
    tokio::time::timeout(
        Duration::from_secs(2),
        BufReader::new(reader).read_line(&mut line),
    )
    .await
    .context("the dictation job did not answer")??;
    serde_json::from_str(&line).context("invalid reply from the dictation job")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_selection_other_than_ostt_blocks_the_paste() {
        assert_eq!(switched_away(Some("ostt\n")), None);
        assert_eq!(switched_away(Some("voxtype\n")).as_deref(), Some("voxtype"));
        // Removing the selection (OSTT's remover does) also means "not OSTT".
        assert_eq!(switched_away(None).as_deref(), Some("no backend"));
    }
}
