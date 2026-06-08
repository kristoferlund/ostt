use anyhow::Context;
use std::io::{self, IsTerminal, Write};
#[cfg(not(target_os = "windows"))]
use std::process::{Command, Stdio};

pub(crate) fn notify_error(title: &str, message: &str) {
    if let Err(err) = try_notify_error(title, message) {
        tracing::debug!("Notification failed: {err}");
        eprintln!("{title}: {message}");
    }
}

pub(crate) fn notify_error_if_popup_context(title: &str, message: &str) -> bool {
    notify_error_if_popup_context_with(title, message, is_popup_context(), notify_error)
}

fn notify_error_if_popup_context_with<F>(
    title: &str,
    message: &str,
    is_popup_context: bool,
    notify: F,
) -> bool
where
    F: FnOnce(&str, &str),
{
    if !is_popup_context {
        return false;
    }
    notify(title, message);
    true
}

fn is_popup_context() -> bool {
    std::env::var("OSTT_POPUP").is_ok_and(|value| value == "1")
}

fn try_notify_error(title: &str, message: &str) -> anyhow::Result<()> {
    match try_desktop_notification(title, message) {
        Ok(()) => Ok(()),
        Err(desktop_err) => try_terminal_notification(title, message)
            .with_context(|| format!("desktop notification failed: {desktop_err}")),
    }
}

#[cfg(target_os = "macos")]
fn try_desktop_notification(title: &str, message: &str) -> anyhow::Result<()> {
    let script = format!(
        "display notification {} with title {}",
        applescript_string(message),
        applescript_string(title)
    );
    run_status(Command::new("osascript").args(["-e", &script]), "osascript")
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn try_desktop_notification(title: &str, message: &str) -> anyhow::Result<()> {
    if !command_exists("notify-send") {
        anyhow::bail!("notify-send not found");
    }
    run_status(
        Command::new("notify-send").args([title, message]),
        "notify-send",
    )
}

#[cfg(target_os = "windows")]
fn try_desktop_notification(_title: &str, _message: &str) -> anyhow::Result<()> {
    anyhow::bail!("desktop notifications are not configured for Windows")
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn try_terminal_notification(title: &str, message: &str) -> anyhow::Result<()> {
    if !io::stderr().is_terminal() {
        anyhow::bail!("stderr is not a terminal");
    }
    let protocol = TerminalNotificationProtocol::detect()
        .ok_or_else(|| anyhow::anyhow!("terminal notification protocol not detected"))?;
    let sequence = terminal_notification_sequence(title, message, protocol);
    let mut stderr = io::stderr().lock();
    stderr
        .write_all(wrap_for_tmux_if_needed(&sequence).as_bytes())
        .context("failed to write terminal notification")?;
    stderr
        .flush()
        .context("failed to flush terminal notification")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalNotificationProtocol {
    ItermOsc9,
    WeztermOsc777,
}

impl TerminalNotificationProtocol {
    fn detect() -> Option<Self> {
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        if term_program.eq_ignore_ascii_case("iTerm.app") {
            return Some(Self::ItermOsc9);
        }
        if term_program.eq_ignore_ascii_case("WezTerm")
            || std::env::var("WEZTERM_EXECUTABLE").is_ok()
        {
            return Some(Self::WeztermOsc777);
        }
        None
    }
}

fn terminal_notification_sequence(
    title: &str,
    message: &str,
    protocol: TerminalNotificationProtocol,
) -> String {
    let title = osc_text(title);
    let message = osc_text(message);
    match protocol {
        TerminalNotificationProtocol::ItermOsc9 => format!("\x1b]9;{title}: {message}\x07"),
        TerminalNotificationProtocol::WeztermOsc777 => {
            format!("\x1b]777;notify;{title};{message}\x07")
        }
    }
}

fn osc_text(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| match ch {
            '\x1b' | '\x07' => None,
            ';' => Some(','),
            '\n' | '\r' | '\t' => Some(' '),
            ch if ch.is_control() => None,
            ch => Some(ch),
        })
        .collect()
}

fn wrap_for_tmux_if_needed(sequence: &str) -> String {
    wrap_for_tmux(sequence, std::env::var("TMUX").is_ok())
}

fn wrap_for_tmux(sequence: &str, in_tmux: bool) -> String {
    if !in_tmux {
        return sequence.to_string();
    }

    format!("\x1bPtmux;{}\x1b\\", sequence.replace('\x1b', "\x1b\x1b"))
}

#[cfg(not(target_os = "windows"))]
fn run_status(command: &mut Command, name: &str) -> anyhow::Result<()> {
    let status = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("Failed to run {name}"))?;
    if !status.success() {
        anyhow::bail!("{name} failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn popup_context_notification_helper_only_notifies_in_popup_context() {
        let calls = RefCell::new(Vec::new());

        let notified = notify_error_if_popup_context_with(
            "Paste Failed",
            "paste failed",
            true,
            |title, message| {
                calls
                    .borrow_mut()
                    .push((title.to_string(), message.to_string()))
            },
        );

        assert!(notified);
        assert_eq!(
            calls.into_inner(),
            vec![("Paste Failed".to_string(), "paste failed".to_string())]
        );

        let skipped =
            notify_error_if_popup_context_with("Paste Failed", "paste failed", false, |_, _| {
                panic!("notification should not run outside popup context")
            });

        assert!(!skipped);
    }

    #[test]
    fn terminal_notification_sequence_uses_known_protocols() {
        assert_eq!(
            terminal_notification_sequence(
                "Build",
                "done",
                TerminalNotificationProtocol::ItermOsc9
            ),
            "\x1b]9;Build: done\x07"
        );
        assert_eq!(
            terminal_notification_sequence(
                "Build",
                "done",
                TerminalNotificationProtocol::WeztermOsc777
            ),
            "\x1b]777;notify;Build;done\x07"
        );
    }

    #[test]
    fn terminal_notification_sequence_removes_osc_control_chars() {
        let sequence = terminal_notification_sequence(
            "Title;\x1b",
            "body\n\x07text",
            TerminalNotificationProtocol::WeztermOsc777,
        );

        assert_eq!(sequence, "\x1b]777;notify;Title,;body text\x07");
    }

    #[test]
    fn tmux_passthrough_wraps_and_escapes_terminal_sequence() {
        let sequence = "\x1b]9;Build done\x07";

        assert_eq!(wrap_for_tmux(sequence, false), sequence);
        assert_eq!(
            wrap_for_tmux(sequence, true),
            "\x1bPtmux;\x1b\x1b]9;Build done\x07\x1b\\"
        );
    }
}
