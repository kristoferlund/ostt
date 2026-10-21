//! One dictation: record until stopped, transcribe, deliver, exit.
//!
//! The lifecycle is idle → recording → transcribing → idle, or error from
//! either active state. `drive` owns the transitions and is generic over the
//! microphone, the transcription and the delivery so the rules about
//! cancellation and output can be tested without audio hardware or a
//! compositor.

use super::state::{Outcome, Phase, StatusHandle};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

/// Requests a controller can send to the running job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Stop,
    Cancel,
}

/// The microphone side of a job.
pub(crate) trait Capture {
    /// Reports a failed or silent input device while recording.
    fn check(&mut self) -> anyhow::Result<()>;
    /// Stops recording and saves the audio. `None` when nothing was captured.
    fn finish(&mut self) -> anyhow::Result<Option<PathBuf>>;
    /// Stops recording and drops the audio without saving it.
    fn discard(&mut self);
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    /// How often the recorder is checked for device errors.
    pub(crate) tick: Duration,
    /// Recording stops on its own after this long, so a missed key release
    /// cannot leave the microphone open indefinitely.
    pub(crate) max_recording: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            tick: Duration::from_millis(100),
            max_recording: Duration::from_secs(15 * 60),
        }
    }
}

/// How a job ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JobEnd {
    Delivered,
    Cancelled,
    Failed(String),
}

/// Runs one job from an already started recording to its end.
///
/// `produce` turns the saved recording into the text to deliver; it may be
/// abandoned midway by a cancel, so it must not have side effects. `commit`
/// saves history and delivers the text; it runs only once the result can no
/// longer be cancelled. `remove_audio` deletes a recording whose result was
/// cancelled, so a cancelled capture leaves nothing behind.
pub(crate) async fn drive<C, P, Fut, K, R>(
    capture: &mut C,
    mut controls: mpsc::UnboundedReceiver<Control>,
    status: &StatusHandle,
    limits: Limits,
    produce: P,
    commit: K,
    remove_audio: R,
) -> JobEnd
where
    C: Capture,
    P: FnOnce(PathBuf) -> Fut,
    Fut: Future<Output = anyhow::Result<String>>,
    K: FnOnce(&Path, &str) -> anyhow::Result<()>,
    R: FnOnce(&Path),
{
    status.set(|s| {
        s.state = Phase::Recording;
        s.outcome = None;
        s.error = None;
    });

    let recording_deadline = tokio::time::Instant::now() + limits.max_recording;
    let mut tick = tokio::time::interval(limits.tick);
    loop {
        tokio::select! {
            control = controls.recv() => match control {
                Some(Control::Stop) => break,
                Some(Control::Cancel) | None => {
                    capture.discard();
                    return end_cancelled(status);
                }
            },
            _ = tick.tick() => {
                if let Err(err) = capture.check() {
                    capture.discard();
                    return end_failed(status, format!("{err:#}"));
                }
            }
            _ = tokio::time::sleep_until(recording_deadline) => {
                tracing::warn!(
                    "dictation: recording reached {}s, stopping",
                    limits.max_recording.as_secs()
                );
                break;
            }
        }
    }

    let audio = match capture.finish() {
        Ok(Some(audio)) => audio,
        Ok(None) => {
            return end_failed(
                status,
                "No audio was captured. Hold the key a little longer, or check the microphone with `ostt config list-devices`.".to_string(),
            )
        }
        Err(err) => return end_failed(status, format!("Could not save the recording: {err:#}")),
    };

    status.set(|s| s.state = Phase::Transcribing);
    let produced = produce(audio.clone());
    tokio::pin!(produced);
    let text = loop {
        tokio::select! {
            result = &mut produced => break result,
            control = controls.recv() => match control {
                // Stopping an already stopped recording is a no-op.
                Some(Control::Stop) => {}
                Some(Control::Cancel) | None => {
                    remove_audio(&audio);
                    return end_cancelled(status);
                }
            },
        }
    };

    let text = match text {
        Ok(text) => text,
        Err(err) => return end_failed(status, format!("{err:#}")),
    };
    if text.trim().is_empty() {
        return end_failed(
            status,
            "No speech was detected in the recording.".to_string(),
        );
    }

    if let Err(err) = commit(&audio, &text) {
        return end_failed(status, format!("{err:#}"));
    }
    status.set(|s| {
        s.state = Phase::Idle;
        s.outcome = Some(Outcome::Pasted);
        s.error = None;
    });
    JobEnd::Delivered
}

fn end_cancelled(status: &StatusHandle) -> JobEnd {
    status.set(|s| {
        s.state = Phase::Idle;
        s.outcome = Some(Outcome::Cancelled);
        s.error = None;
    });
    JobEnd::Cancelled
}

fn end_failed(status: &StatusHandle, message: String) -> JobEnd {
    tracing::warn!("dictation: {message}");
    status.set(|s| {
        s.state = Phase::Error;
        s.outcome = None;
        s.error = Some(message.clone());
    });
    JobEnd::Failed(message)
}

#[cfg(test)]
mod tests {
    use super::super::state::{Paths, Status};
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct FakeCapture {
        fail_check: Option<String>,
        captured: bool,
        finished: bool,
        discarded: bool,
    }

    impl Capture for FakeCapture {
        fn check(&mut self) -> anyhow::Result<()> {
            match &self.fail_check {
                Some(message) => anyhow::bail!("{message}"),
                None => Ok(()),
            }
        }
        fn finish(&mut self) -> anyhow::Result<Option<PathBuf>> {
            self.finished = true;
            Ok(self.captured.then(|| PathBuf::from("/recordings/a.wav")))
        }
        fn discard(&mut self) {
            self.discarded = true;
        }
    }

    struct Harness {
        _dir: tempfile::TempDir,
        status: StatusHandle,
        delivered: Rc<RefCell<Vec<String>>>,
        removed: Rc<RefCell<Vec<PathBuf>>>,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().to_path_buf());
        Harness {
            status: StatusHandle::new(paths.state(), Status::idle()),
            _dir: dir,
            delivered: Rc::default(),
            removed: Rc::default(),
        }
    }

    fn limits() -> Limits {
        Limits {
            tick: Duration::from_millis(5),
            max_recording: Duration::from_secs(60),
        }
    }

    async fn run(
        h: &Harness,
        capture: &mut FakeCapture,
        controls: mpsc::UnboundedReceiver<Control>,
        produce: impl Future<Output = anyhow::Result<String>>,
    ) -> JobEnd {
        run_with(h, capture, controls, limits(), produce).await
    }

    async fn run_with(
        h: &Harness,
        capture: &mut FakeCapture,
        controls: mpsc::UnboundedReceiver<Control>,
        limits: Limits,
        produce: impl Future<Output = anyhow::Result<String>>,
    ) -> JobEnd {
        let delivered = h.delivered.clone();
        let removed = h.removed.clone();
        drive(
            capture,
            controls,
            &h.status,
            limits,
            |_| produce,
            move |_, text| {
                delivered.borrow_mut().push(text.to_string());
                Ok(())
            },
            move |path| removed.borrow_mut().push(path.to_path_buf()),
        )
        .await
    }

    #[tokio::test]
    async fn stop_transcribes_and_delivers_once() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Stop).unwrap();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };

        let end = run(&h, &mut capture, rx, async { Ok("hello".to_string()) }).await;

        assert_eq!(end, JobEnd::Delivered);
        assert_eq!(*h.delivered.borrow(), vec!["hello"]);
        assert_eq!(h.status.get().state, Phase::Idle);
        assert_eq!(h.status.get().outcome, Some(Outcome::Pasted));
    }

    /// Cancel while recording must not save audio: saving would also prune
    /// the oldest recording from history.
    #[tokio::test]
    async fn cancel_while_recording_discards_without_saving() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Cancel).unwrap();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };

        let end = run(&h, &mut capture, rx, async { Ok("hello".to_string()) }).await;

        assert_eq!(end, JobEnd::Cancelled);
        assert!(capture.discarded);
        assert!(!capture.finished);
        assert!(h.delivered.borrow().is_empty());
        assert_eq!(h.status.get().outcome, Some(Outcome::Cancelled));
    }

    /// A late result must never be pasted after the user cancelled: by then
    /// they may be typing somewhere else.
    #[tokio::test]
    async fn cancel_while_transcribing_drops_the_result_and_the_audio() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Stop).unwrap();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };
        let canceller = tx.clone();
        let slow_transcription = async move {
            canceller.send(Control::Cancel).unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok("too late".to_string())
        };

        let end = run(&h, &mut capture, rx, slow_transcription).await;

        assert_eq!(end, JobEnd::Cancelled);
        assert!(h.delivered.borrow().is_empty());
        assert_eq!(
            *h.removed.borrow(),
            vec![PathBuf::from("/recordings/a.wav")]
        );
    }

    /// A hold key released twice, or stop racing the release, must not
    /// abort a transcription that is already running.
    #[tokio::test]
    async fn stop_while_transcribing_is_a_no_op() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Stop).unwrap();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };
        let second_stop = tx.clone();
        let transcription = async move {
            second_stop.send(Control::Stop).unwrap();
            tokio::task::yield_now().await;
            Ok("hello".to_string())
        };

        let end = run(&h, &mut capture, rx, transcription).await;

        assert_eq!(end, JobEnd::Delivered);
        assert_eq!(*h.delivered.borrow(), vec!["hello"]);
    }

    #[tokio::test]
    async fn an_empty_capture_is_an_actionable_error_without_transcribing() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Stop).unwrap();
        let mut capture = FakeCapture::default();

        let end = run(&h, &mut capture, rx, async {
            panic!("nothing to transcribe");
        })
        .await;

        let JobEnd::Failed(message) = end else {
            panic!("expected failure");
        };
        assert!(message.contains("No audio was captured"));
        assert_eq!(h.status.get().state, Phase::Error);
        assert!(h.delivered.borrow().is_empty());
    }

    #[tokio::test]
    async fn a_device_error_while_recording_ends_the_job() {
        let h = harness();
        let (_tx, rx) = mpsc::unbounded_channel();
        let mut capture = FakeCapture {
            fail_check: Some("device unplugged".to_string()),
            ..Default::default()
        };

        let end = run(&h, &mut capture, rx, async { Ok("x".to_string()) }).await;

        assert_eq!(end, JobEnd::Failed("device unplugged".to_string()));
        assert!(capture.discarded);
        assert_eq!(h.status.get().error.as_deref(), Some("device unplugged"));
    }

    #[tokio::test]
    async fn blank_transcripts_are_not_pasted() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Stop).unwrap();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };

        let end = run(&h, &mut capture, rx, async { Ok("  \n".to_string()) }).await;

        assert!(matches!(end, JobEnd::Failed(m) if m.contains("No speech")));
        assert!(h.delivered.borrow().is_empty());
    }

    #[tokio::test]
    async fn transcription_errors_surface_in_the_status() {
        let h = harness();
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Control::Stop).unwrap();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };

        let end = run(&h, &mut capture, rx, async {
            Err(anyhow::anyhow!("Deepgram returned 401"))
        })
        .await;

        assert!(matches!(end, JobEnd::Failed(_)));
        assert_eq!(
            h.status.get().error.as_deref(),
            Some("Deepgram returned 401")
        );
    }

    #[tokio::test]
    async fn a_forgotten_recording_stops_at_the_limit_and_is_delivered() {
        let h = harness();
        let (_tx, rx) = mpsc::unbounded_channel();
        let mut capture = FakeCapture {
            captured: true,
            ..Default::default()
        };
        let limits = Limits {
            tick: Duration::from_millis(5),
            max_recording: Duration::from_millis(50),
        };

        let end = run_with(&h, &mut capture, rx, limits, async {
            Ok("long".to_string())
        })
        .await;

        assert_eq!(end, JobEnd::Delivered);
        assert!(capture.finished);
    }
}
