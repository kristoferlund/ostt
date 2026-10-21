//! Dictation status shared between the job, its controllers and desktop widgets.
//!
//! The job is the only writer. It replaces `state.json` atomically on every
//! transition, so a reader never sees a partial file. A job that dies without
//! writing its final state leaves a stale "recording" or "transcribing" file
//! behind; readers detect that through the job lock and report an error
//! instead of a recording that is not happening.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Idle,
    Recording,
    Transcribing,
    Error,
}

impl Phase {
    pub(crate) fn is_active(self) -> bool {
        matches!(self, Self::Recording | Self::Transcribing)
    }
}

/// How the last job ended, shown next to an idle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Pasted,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Status {
    pub(crate) state: Phase,
    /// Identifies the job that wrote this status. Controllers wait for their
    /// own job id, so they never mistake an earlier job's state for theirs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) job: Option<String>,
    /// When the current state began (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) since: Option<String>,
    /// The `provider/model` the job transcribes with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) outcome: Option<Outcome>,
    /// Actionable error text. Never contains transcript content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

impl Status {
    pub(crate) fn idle() -> Self {
        Self {
            state: Phase::Idle,
            job: None,
            since: None,
            model: None,
            outcome: None,
            error: None,
        }
    }
}

/// Files of the dictation runtime directory.
#[derive(Clone, Debug)]
pub(crate) struct Paths {
    pub(crate) dir: PathBuf,
}

impl Paths {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub(crate) fn default_location() -> Self {
        Self::new(crate::app_dirs::runtime_dir().join("dictation"))
    }

    pub(crate) fn state(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    /// Held by the job for its whole life: one job at a time.
    pub(crate) fn job_lock(&self) -> PathBuf {
        self.dir.join("job.lock")
    }

    /// Held by start/stop/toggle/cancel while they act, so racing callers
    /// (a hold key's press and release) run one after another.
    pub(crate) fn control_lock(&self) -> PathBuf {
        self.dir.join("control.lock")
    }

    pub(crate) fn socket(&self) -> PathBuf {
        self.dir.join("control.sock")
    }

    /// Creates the directory, private to the user: it holds the control socket.
    pub(crate) fn ensure_dir(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("failed to create {}", self.dir.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o700))
                .with_context(|| format!("failed to restrict {}", self.dir.display()))?;
        }
        Ok(())
    }
}

pub(crate) fn open_lock_file(path: &Path) -> anyhow::Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))
}

/// True while some job holds the job lock.
pub(crate) fn job_alive(paths: &Paths) -> bool {
    let Ok(file) = open_lock_file(&paths.job_lock()) else {
        return false;
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(std::fs::TryLockError::WouldBlock) => true,
        Err(std::fs::TryLockError::Error(err)) => {
            tracing::warn!("dictation: could not check job lock: {err}");
            false
        }
    }
}

pub(crate) fn read_status_file(paths: &Paths) -> Option<Status> {
    let raw = std::fs::read_to_string(paths.state()).ok()?;
    serde_json::from_str(&raw).ok()
}

/// The status as a reader should see it, with stale job states resolved.
pub(crate) fn current_status(paths: &Paths) -> Status {
    let status = read_status_file(paths).unwrap_or_else(Status::idle);
    resolve_stale(status, job_alive(paths))
}

fn resolve_stale(status: Status, alive: bool) -> Status {
    if status.state.is_active() && !alive {
        return Status {
            state: Phase::Error,
            outcome: None,
            error: Some(
                "The dictation job stopped unexpectedly. See `ostt logs` for details.".to_string(),
            ),
            ..status
        };
    }
    status
}

/// The job's handle on its status: keeps it in memory for the control socket
/// and mirrors every change to `state.json`.
#[derive(Clone)]
pub(crate) struct StatusHandle {
    path: PathBuf,
    current: Arc<Mutex<Status>>,
}

impl StatusHandle {
    pub(crate) fn new(path: PathBuf, initial: Status) -> Self {
        Self {
            path,
            current: Arc::new(Mutex::new(initial)),
        }
    }

    pub(crate) fn get(&self) -> Status {
        self.current.lock().unwrap().clone()
    }

    pub(crate) fn set(&self, update: impl FnOnce(&mut Status)) {
        let snapshot = {
            let mut status = self.current.lock().unwrap();
            update(&mut status);
            status.since = Some(chrono::Local::now().to_rfc3339());
            status.clone()
        };
        if let Err(err) = write_atomically(&self.path, &snapshot) {
            tracing::warn!("dictation: failed to write status: {err:#}");
        }
    }
}

fn write_atomically(path: &Path, status: &Status) -> anyhow::Result<()> {
    let json = serde_json::to_vec(status)?;
    let temporary = path.with_extension(format!("json.{}", std::process::id()));
    {
        let mut file = File::create(&temporary)
            .with_context(|| format!("failed to create {}", temporary.display()))?;
        file.write_all(&json)?;
        file.write_all(b"\n")?;
    }
    std::fs::rename(&temporary, path)
        .with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> (tempfile::TempDir, Paths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("dictation"));
        paths.ensure_dir().unwrap();
        (dir, paths)
    }

    fn recording(job: &str) -> Status {
        Status {
            state: Phase::Recording,
            job: Some(job.to_string()),
            ..Status::idle()
        }
    }

    /// A widget must not show "recording" forever after the job crashed.
    #[test]
    fn an_active_state_without_a_live_job_reads_as_an_error() {
        let resolved = resolve_stale(recording("a"), false);
        assert_eq!(resolved.state, Phase::Error);
        assert_eq!(resolved.job.as_deref(), Some("a"));
        assert!(resolved.error.unwrap().contains("ostt logs"));

        assert_eq!(resolve_stale(recording("a"), true), recording("a"));
        assert_eq!(resolve_stale(Status::idle(), false), Status::idle());
    }

    #[test]
    fn status_survives_a_round_trip_through_the_state_file() {
        let (_dir, paths) = paths();
        let handle = StatusHandle::new(paths.state(), Status::idle());
        handle.set(|status| *status = recording("job-1"));

        let read = read_status_file(&paths).unwrap();
        assert_eq!(read.state, Phase::Recording);
        assert_eq!(read.job.as_deref(), Some("job-1"));
        assert!(read.since.is_some());
    }

    #[test]
    fn no_state_file_means_idle() {
        let (_dir, paths) = paths();
        assert_eq!(current_status(&paths), Status::idle());
    }

    #[test]
    fn a_held_job_lock_marks_the_job_alive() {
        let (_dir, paths) = paths();
        assert!(!job_alive(&paths));

        let lock = open_lock_file(&paths.job_lock()).unwrap();
        lock.lock().unwrap();
        assert!(job_alive(&paths));

        drop(lock);
        assert!(!job_alive(&paths));
    }
}
