//! `ostt dictate start|stop|toggle|cancel|status`: the commands desktop
//! integrations call. They return promptly; the job does the slow work.
//!
//! Every command that changes state first takes the control lock, so callers
//! that race (a hold key's press and release, a key and the bar widget) act
//! one after another. `start` keeps the lock until the job is recording, so a
//! `stop` that arrives during start-up stops that recording instead of
//! finding nothing to stop.

use super::run::send_control;
use super::state::{current_status, job_alive, open_lock_file, Paths, Phase, Status};
use anyhow::Context;
use std::fs::File;
use std::time::{Duration, Instant};

/// How long `start` waits for the microphone to open before giving up.
const START_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a controller waits for another controller to finish.
const CONTROL_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// Starts the background job.
pub(crate) trait Launcher: Sync {
    fn launch(&self, job_id: &str, process: Option<&str>) -> anyhow::Result<Box<dyn LaunchedJob>>;
}

pub(crate) trait LaunchedJob: Send {
    /// Describes how the job exited, once it has.
    fn exited(&mut self) -> Option<String>;
    fn kill(&mut self);
}

/// Launches `ostt dictate run` as a detached process.
pub(crate) struct ProcessLauncher;

impl Launcher for ProcessLauncher {
    fn launch(&self, job_id: &str, process: Option<&str>) -> anyhow::Result<Box<dyn LaunchedJob>> {
        let exe = std::env::current_exe().context("could not determine the ostt executable")?;
        let mut command = std::process::Command::new(exe);
        command.args(["dictate", "run", "--job", job_id]);
        if let Some(action) = process {
            command.args(["--process", action]);
        }
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        // Own process group: the job outlives the key binding that started it.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command
            .spawn()
            .context("failed to start the dictation job")?;
        Ok(Box::new(child))
    }
}

impl LaunchedJob for std::process::Child {
    fn exited(&mut self) -> Option<String> {
        self.try_wait()
            .ok()
            .flatten()
            .map(|status| status.to_string())
    }

    fn kill(&mut self) {
        let _ = std::process::Child::kill(self);
    }
}

/// The job reported this failure itself, and has already notified the user.
#[derive(Debug)]
pub(crate) struct JobFailed(pub(crate) String);

impl std::fmt::Display for JobFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JobFailed {}

pub(crate) async fn start(
    paths: &Paths,
    launcher: &dyn Launcher,
    process: Option<&str>,
) -> anyhow::Result<Status> {
    paths.ensure_dir()?;
    let _control = lock_control(paths)?;
    start_locked(paths, launcher, process).await
}

pub(crate) async fn stop(paths: &Paths) -> anyhow::Result<Status> {
    send_if_running(paths, "stop").await
}

pub(crate) async fn cancel(paths: &Paths) -> anyhow::Result<Status> {
    send_if_running(paths, "cancel").await
}

pub(crate) async fn toggle(
    paths: &Paths,
    launcher: &dyn Launcher,
    process: Option<&str>,
) -> anyhow::Result<Status> {
    paths.ensure_dir()?;
    let _control = lock_control(paths)?;
    if job_alive(paths) {
        let status = current_status(paths);
        if status.state == Phase::Recording {
            return send_control(paths, "stop").await;
        }
        return Err(busy(&status));
    }
    start_locked(paths, launcher, process).await
}

pub(crate) fn status(paths: &Paths) -> Status {
    current_status(paths)
}

async fn start_locked(
    paths: &Paths,
    launcher: &dyn Launcher,
    process: Option<&str>,
) -> anyhow::Result<Status> {
    if job_alive(paths) {
        let status = current_status(paths);
        // A second press while recording keeps the one recording going.
        if status.state == Phase::Recording {
            return Ok(status);
        }
        return Err(busy(&status));
    }

    let job_id = new_job_id();
    let mut job = launcher.launch(&job_id, process)?;
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Some(status) = super::state::read_status_file(paths) {
            if status.job.as_deref() == Some(job_id.as_str()) {
                match status.state {
                    Phase::Recording | Phase::Transcribing | Phase::Idle => return Ok(status),
                    Phase::Error => {
                        return Err(JobFailed(
                            status.error.unwrap_or_else(|| "dictation failed".into()),
                        )
                        .into())
                    }
                }
            }
        }
        if let Some(exit) = job.exited() {
            anyhow::bail!("the dictation job exited ({exit}) before recording; see `ostt logs`");
        }
        if Instant::now() >= deadline {
            job.kill();
            anyhow::bail!(
                "the microphone did not open within {}s; see `ostt logs`",
                START_TIMEOUT.as_secs()
            );
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Stop and cancel succeed when no job runs: releasing a hold key after a
/// cancel, or switching away from OSTT, must not report a failure.
async fn send_if_running(paths: &Paths, request: &str) -> anyhow::Result<Status> {
    if !paths.dir.exists() {
        return Ok(Status::idle());
    }
    let _control = lock_control(paths)?;
    if !job_alive(paths) {
        return Ok(current_status(paths));
    }
    match send_control(paths, request).await {
        Ok(status) => Ok(status),
        // The job finished between the liveness check and the request.
        Err(_) if !job_alive(paths) => Ok(current_status(paths)),
        Err(err) => Err(err),
    }
}

/// Overlapping work is rejected rather than queued.
fn busy(status: &Status) -> anyhow::Error {
    match status.state {
        Phase::Transcribing => anyhow::anyhow!("Still transcribing the previous dictation."),
        _ => anyhow::anyhow!("Another dictation is starting; try again."),
    }
}

fn lock_control(paths: &Paths) -> anyhow::Result<File> {
    let file = open_lock_file(&paths.control_lock())?;
    let deadline = Instant::now() + CONTROL_LOCK_TIMEOUT;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                anyhow::bail!("Dictation is busy; try again.")
            }
            Err(std::fs::TryLockError::Error(err)) => {
                return Err(err).context("failed to lock dictation control")
            }
        }
    }
}

fn new_job_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!(
        "{}-{}-{}",
        chrono::Local::now().format("%Y%m%d%H%M%S%3f"),
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    //! These run real jobs (lock, socket, status file) on threads, with a
    //! fake microphone and transcription, so the races and leftovers the
    //! commands must survive are exercised end to end.

    use super::super::job::{self, Capture, Limits};
    use super::super::run::host_job;
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::Notify;

    struct FakeMic {
        captured: bool,
    }

    impl Capture for FakeMic {
        fn check(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        fn finish(&mut self) -> anyhow::Result<Option<PathBuf>> {
            Ok(self.captured.then(|| PathBuf::from("/recordings/fake.wav")))
        }
        fn discard(&mut self) {}
    }

    #[derive(Clone)]
    struct FakeLauncher {
        paths: Paths,
        launches: Arc<AtomicUsize>,
        delivered: Arc<Mutex<Vec<String>>>,
        /// When set, transcription waits until the test releases it.
        gate: Option<Arc<Notify>>,
        setup_error: Option<String>,
    }

    impl FakeLauncher {
        fn new(paths: &Paths) -> Self {
            Self {
                paths: paths.clone(),
                launches: Arc::default(),
                delivered: Arc::default(),
                gate: None,
                setup_error: None,
            }
        }
    }

    struct ThreadJob(Option<std::thread::JoinHandle<()>>);

    impl LaunchedJob for ThreadJob {
        fn exited(&mut self) -> Option<String> {
            self.0
                .as_ref()
                .is_some_and(|handle| handle.is_finished())
                .then(|| "exit status: 1".to_string())
        }
        fn kill(&mut self) {}
    }

    impl Launcher for FakeLauncher {
        fn launch(&self, job_id: &str, _: Option<&str>) -> anyhow::Result<Box<dyn LaunchedJob>> {
            self.launches.fetch_add(1, Ordering::SeqCst);
            let fake = self.clone();
            let job_id = job_id.to_string();
            let handle = std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let _ = runtime.block_on(host_job(
                    &fake.paths.clone(),
                    job_id,
                    None,
                    |_, _| {},
                    |controls, status| async move {
                        if let Some(message) = fake.setup_error.clone() {
                            anyhow::bail!(message);
                        }
                        let gate = fake.gate.clone();
                        let delivered = fake.delivered.clone();
                        let mut mic = FakeMic { captured: true };
                        Ok(job::drive(
                            &mut mic,
                            controls,
                            &status,
                            Limits {
                                tick: Duration::from_millis(5),
                                max_recording: Duration::from_secs(30),
                            },
                            move |_| async move {
                                if let Some(gate) = gate {
                                    gate.notified().await;
                                }
                                Ok("dictated text".to_string())
                            },
                            move |_, text| {
                                delivered.lock().unwrap().push(text.to_string());
                                Ok(())
                            },
                            |_| {},
                        )
                        .await)
                    },
                ));
            });
            Ok(Box::new(ThreadJob(Some(handle))))
        }
    }

    fn control_held(paths: &Paths) -> bool {
        let Ok(file) = open_lock_file(&paths.control_lock()) else {
            return false;
        };
        match file.try_lock() {
            Ok(()) => false,
            Err(_) => true,
        }
    }

    fn paths() -> (tempfile::TempDir, Paths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("dictation"));
        (dir, paths)
    }

    async fn wait_for(paths: &Paths, phase: Phase) -> Status {
        for _ in 0..500 {
            let status = current_status(paths);
            if status.state == phase && !(phase == Phase::Idle && job_alive(paths)) {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!(
            "dictation never reached {phase:?}: {:?}",
            current_status(paths)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn idle_stop_and_cancel_succeed_without_starting_anything() {
        let (_dir, paths) = paths();

        assert_eq!(stop(&paths).await.unwrap().state, Phase::Idle);
        assert_eq!(cancel(&paths).await.unwrap().state, Phase::Idle);
        assert!(!paths.dir.exists(), "stop must not create runtime state");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn start_then_stop_pastes_exactly_once() {
        let (_dir, paths) = paths();
        let launcher = FakeLauncher::new(&paths);

        assert_eq!(
            start(&paths, &launcher, None).await.unwrap().state,
            Phase::Recording
        );
        stop(&paths).await.unwrap();
        wait_for(&paths, Phase::Idle).await;

        assert_eq!(*launcher.delivered.lock().unwrap(), vec!["dictated text"]);
    }

    /// Pressing start again while recording must not open a second recorder.
    #[tokio::test(flavor = "multi_thread")]
    async fn repeated_start_keeps_one_recording() {
        let (_dir, paths) = paths();
        let launcher = FakeLauncher::new(&paths);

        start(&paths, &launcher, None).await.unwrap();
        start(&paths, &launcher, None).await.unwrap();

        assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
        cancel(&paths).await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn racing_starts_launch_one_job() {
        let (_dir, paths) = paths();
        let launcher = FakeLauncher::new(&paths);

        let callers: Vec<_> = (0..4)
            .map(|_| {
                let paths = paths.clone();
                let launcher = launcher.clone();
                tokio::spawn(async move { start(&paths, &launcher, None).await })
            })
            .collect();
        for caller in callers {
            caller.await.unwrap().unwrap();
        }

        assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
        cancel(&paths).await.unwrap();
    }

    /// A hold key's release can run while its press is still opening the
    /// microphone. The release must stop that recording, not find nothing.
    #[tokio::test(flavor = "multi_thread")]
    async fn stop_racing_start_still_stops_the_recording() {
        let (_dir, paths) = paths();
        let launcher = FakeLauncher::new(&paths);

        let press = {
            let paths = paths.clone();
            let launcher = launcher.clone();
            tokio::spawn(async move { start(&paths, &launcher, None).await })
        };
        // Let the press take the control lock first, as Omarchy's
        // dispatcher orders press before release.
        while !control_held(&paths) {
            tokio::task::yield_now().await;
        }
        let release = {
            let paths = paths.clone();
            tokio::spawn(async move { stop(&paths).await })
        };
        press.await.unwrap().unwrap();
        release.await.unwrap().unwrap();
        wait_for(&paths, Phase::Idle).await;

        assert_eq!(*launcher.delivered.lock().unwrap(), vec!["dictated text"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn toggle_starts_then_stops() {
        let (_dir, paths) = paths();
        let launcher = FakeLauncher::new(&paths);

        assert_eq!(
            toggle(&paths, &launcher, None).await.unwrap().state,
            Phase::Recording
        );
        toggle(&paths, &launcher, None).await.unwrap();
        wait_for(&paths, Phase::Idle).await;

        assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
        assert_eq!(launcher.delivered.lock().unwrap().len(), 1);
    }

    /// New work while transcribing is refused, not queued: a queued second
    /// dictation would paste somewhere the user has since moved on from.
    #[tokio::test(flavor = "multi_thread")]
    async fn starting_while_transcribing_is_refused() {
        let (_dir, paths) = paths();
        let gate = Arc::new(Notify::new());
        let launcher = FakeLauncher {
            gate: Some(gate.clone()),
            ..FakeLauncher::new(&paths)
        };

        start(&paths, &launcher, None).await.unwrap();
        stop(&paths).await.unwrap();
        wait_for(&paths, Phase::Transcribing).await;

        let err = start(&paths, &launcher, None).await.unwrap_err();
        assert!(err.to_string().contains("Still transcribing"));
        let err = toggle(&paths, &launcher, None).await.unwrap_err();
        assert!(err.to_string().contains("Still transcribing"));
        assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);

        gate.notify_one();
        wait_for(&paths, Phase::Idle).await;
        assert_eq!(launcher.delivered.lock().unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancel_while_transcribing_never_pastes() {
        let (_dir, paths) = paths();
        let gate = Arc::new(Notify::new());
        let launcher = FakeLauncher {
            gate: Some(gate.clone()),
            ..FakeLauncher::new(&paths)
        };

        start(&paths, &launcher, None).await.unwrap();
        stop(&paths).await.unwrap();
        wait_for(&paths, Phase::Transcribing).await;
        cancel(&paths).await.unwrap();
        let status = wait_for(&paths, Phase::Idle).await;
        gate.notify_one();

        assert_eq!(
            status.outcome,
            Some(super::super::state::Outcome::Cancelled)
        );
        assert!(launcher.delivered.lock().unwrap().is_empty());
    }

    /// A missing model or credential must fail the key press itself.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_setup_failure_is_returned_by_start() {
        let (_dir, paths) = paths();
        let launcher = FakeLauncher {
            setup_error: Some("No transcription model selected. Run `ostt model`.".to_string()),
            ..FakeLauncher::new(&paths)
        };

        let err = start(&paths, &launcher, None).await.unwrap_err();

        assert!(err.downcast_ref::<JobFailed>().is_some());
        assert!(err.to_string().contains("ostt model"));
        assert_eq!(current_status(&paths).state, Phase::Error);
    }

    /// State left by a job that crashed must not block the next dictation.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_stale_recording_state_does_not_block_a_new_job() {
        let (_dir, paths) = paths();
        paths.ensure_dir().unwrap();
        std::fs::write(paths.state(), r#"{"state":"recording","job":"crashed"}"#).unwrap();
        std::fs::write(paths.socket(), "").unwrap();
        let launcher = FakeLauncher::new(&paths);

        assert_eq!(stop(&paths).await.unwrap().state, Phase::Error);
        assert_eq!(
            start(&paths, &launcher, None).await.unwrap().state,
            Phase::Recording
        );
        stop(&paths).await.unwrap();
        wait_for(&paths, Phase::Idle).await;
        assert_eq!(launcher.delivered.lock().unwrap().len(), 1);
    }
}
