use anyhow::{anyhow, Context};
use signal_hook::consts::SIGUSR1;
use signal_hook::low_level::unregister;
use signal_hook::SigId;
use std::process::Command;
use std::sync::{atomic::AtomicBool, Arc};

pub(crate) struct ActiveRecordingGuard {
    _signal_guard: SignalGuard,
    _pid_guard: RecordingPidGuard,
}

struct RecordingPidGuard {
    path: std::path::PathBuf,
}

struct SignalGuard {
    id: SigId,
}

impl ActiveRecordingGuard {
    pub(crate) fn start(term: Arc<AtomicBool>) -> anyhow::Result<Self> {
        Ok(Self {
            _signal_guard: register_transcription_signal(term)
                .context("failed to register recording signal handler")?,
            _pid_guard: write_recording_pid_file().context("failed to write recording PID file")?,
        })
    }
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

pub(crate) fn find_running_recorder() -> Option<u32> {
    let pid_path = crate::app_dirs::recording_pid_path();
    let pid = std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())?;

    if is_recording_process(pid) {
        Some(pid)
    } else {
        tracing::debug!("Removing stale recording PID file: {}", pid_path.display());
        let _ = std::fs::remove_file(pid_path);
        None
    }
}

pub(crate) fn signal_running_recorder(pid: u32) -> anyhow::Result<()> {
    tracing::debug!("Sending SIGUSR1 to ostt PID {}", pid);

    let status = Command::new("kill")
        .args(["-USR1", &pid.to_string()])
        .status()
        .context("failed to send SIGUSR1")?;

    if !status.success() {
        return Err(anyhow!("failed to send SIGUSR1 to PID {}", pid));
    }

    Ok(())
}

fn write_recording_pid_file() -> anyhow::Result<RecordingPidGuard> {
    let path = crate::app_dirs::recording_pid_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create recording PID directory: {}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&path, std::process::id().to_string())
        .with_context(|| format!("failed to write PID to {}", path.display()))?;
    Ok(RecordingPidGuard { path })
}

fn register_transcription_signal(term: Arc<AtomicBool>) -> anyhow::Result<SignalGuard> {
    let id = signal_hook::flag::register(SIGUSR1, term)?;
    Ok(SignalGuard { id })
}

fn is_recording_process(pid: u32) -> bool {
    pid != std::process::id() && process_exists(pid)
}

fn process_exists(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}
