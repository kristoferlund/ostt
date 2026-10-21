//! `ostt dictate`: record without a terminal for desktop dictation keys.

use crate::dictation::{control, state::Paths, state::Status};

pub(crate) enum DictateAction<'a> {
    Start { process: Option<&'a str> },
    Stop,
    Toggle { process: Option<&'a str> },
    Cancel,
    Status { json: bool, follow: bool },
}

pub(crate) async fn handle_dictate(action: DictateAction<'_>) -> anyhow::Result<()> {
    let paths = Paths::default_location();
    let launcher = control::ProcessLauncher;
    let result = match action {
        DictateAction::Start { process } => control::start(&paths, &launcher, process).await,
        DictateAction::Stop => control::stop(&paths).await,
        DictateAction::Toggle { process } => control::toggle(&paths, &launcher, process).await,
        DictateAction::Cancel => control::cancel(&paths).await,
        DictateAction::Status {
            json,
            follow: false,
        } => {
            print_status(&control::status(&paths), json);
            return Ok(());
        }
        DictateAction::Status { json, follow: true } => return follow_status(&paths, json).await,
    };
    // These commands usually run from a key binding, where stderr goes
    // nowhere. Failures the job reported were already shown by the job.
    if let Err(err) = &result {
        if err.downcast_ref::<control::JobFailed>().is_none() {
            crate::notifier::notify_error("Dictation", &format!("{err:#}"));
        }
    }
    result.map(drop)
}

/// Prints the status each time it changes, for widgets that read a stream.
async fn follow_status(paths: &Paths, json: bool) -> anyhow::Result<()> {
    let mut last = None;
    loop {
        let status = control::status(paths);
        if last.as_ref() != Some(&status) {
            print_status(&status, json);
            last = Some(status);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

fn print_status(status: &Status, json: bool) {
    use std::io::Write;
    let line = if json {
        serde_json::to_string(status).unwrap_or_default()
    } else {
        let state = serde_json::to_value(status.state)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        match &status.error {
            Some(error) => format!("{state}: {error}"),
            None => state,
        }
    };
    // A widget that closes the pipe ends the follower.
    if writeln!(std::io::stdout(), "{line}").is_err() || std::io::stdout().flush().is_err() {
        std::process::exit(0);
    }
}
