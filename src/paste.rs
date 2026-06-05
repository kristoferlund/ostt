use crate::clipboard::{read_clipboard, set_clipboard};
use crate::config::PasteConfig;
use anyhow::Context;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const MACOS_ACCESSIBILITY_REMEDIATION: &str = "Grant accessibility permissions to your terminal app or OSTT launcher in System Settings > Privacy & Security > Accessibility.";
const POPUP_TITLE: &str = "ostt";

#[cfg(target_os = "macos")]
const MACOS_POPUP_APPS: &[&str] = &["Ghostty", "kitty", "Alacritty"];

pub(crate) fn notify_no_popup_error(title: &str, message: &str) {
    if let Err(err) = try_notify_no_popup_error(title, message) {
        tracing::debug!("No-popup notification failed: {err}");
        eprintln!("{title}: {message}");
    }
}

pub(crate) fn notify_no_popup_error_if_popup_context(title: &str, message: &str) -> bool {
    notify_no_popup_error_if_popup_context_with(
        title,
        message,
        is_popup_context(),
        notify_no_popup_error,
    )
}

fn notify_no_popup_error_if_popup_context_with<F>(
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

#[cfg(target_os = "macos")]
fn try_notify_no_popup_error(title: &str, message: &str) -> anyhow::Result<()> {
    let script = format!(
        "display alert {} message {} as critical",
        applescript_string(title),
        applescript_string(message)
    );
    run_status(Command::new("osascript").args(["-e", &script]), "osascript")
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(not(target_os = "macos"))]
fn try_notify_no_popup_error(title: &str, message: &str) -> anyhow::Result<()> {
    if !command_exists("notify-send") {
        anyhow::bail!("notify-send not found");
    }
    run_status(
        Command::new("notify-send").args([title, message]),
        "notify-send",
    )
}

#[cfg(not(target_os = "macos"))]
fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub(crate) fn wait_for_focus_after_popup(config: &PasteConfig) {
    #[cfg(target_os = "macos")]
    {
        if wait_for_macos_focus_after_popup(config) {
            return;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        use std::time::Instant;

        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            let deadline = Instant::now() + Duration::from_millis(config.post_popup_delay_ms);
            while Instant::now() < deadline {
                match active_hyprland_window_title() {
                    Some(title) if title == POPUP_TITLE => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Some(title) => {
                        tracing::debug!("Paste mode: focus returned to window title '{title}'");
                        return;
                    }
                    None => break,
                }
            }

            tracing::debug!(
                "Paste mode: focus settle timeout reached after {}ms",
                config.post_popup_delay_ms
            );
            return;
        }
    }

    thread::sleep(Duration::from_millis(config.post_popup_delay_ms));
}

#[cfg(target_os = "macos")]
fn wait_for_macos_focus_after_popup(config: &PasteConfig) -> bool {
    use std::time::Instant;

    let deadline = Instant::now() + Duration::from_millis(config.post_popup_delay_ms);
    while Instant::now() < deadline {
        let Some((app_name, window_title)) = active_macos_app_window() else {
            return false;
        };

        if !is_macos_popup_window(&app_name, &window_title) {
            tracing::debug!(
                "Paste mode: focus returned to macOS app '{app_name}' window '{window_title}'"
            );
            return true;
        }

        thread::sleep(Duration::from_millis(25));
    }

    tracing::debug!(
        "Paste mode: macOS focus settle timeout reached after {}ms",
        config.post_popup_delay_ms
    );
    true
}

#[cfg(target_os = "macos")]
fn active_macos_app_window() -> Option<(String, String)> {
    let script = r#"
tell application "System Events"
    set frontApp to first application process whose frontmost is true
    set appName to name of frontApp
    set windowTitle to ""
    try
        set windowTitle to name of front window of frontApp
    end try
    return appName & tab & windowTitle
end tell
"#;
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let (app_name, window_title) = text.trim_end().split_once('\t')?;
    Some((app_name.to_string(), window_title.to_string()))
}

#[cfg(target_os = "macos")]
fn is_macos_popup_window(app_name: &str, window_title: &str) -> bool {
    MACOS_POPUP_APPS
        .iter()
        .any(|popup_app| app_name.eq_ignore_ascii_case(popup_app))
        && window_title == POPUP_TITLE
}

pub(crate) fn paste_text(text: &str, config: &PasteConfig) -> anyhow::Result<()> {
    tracing::debug!(
        "Paste mode: paste_key='{}', restore_clipboard={}, restore_delay_ms={}",
        config.paste_key,
        config.restore_clipboard,
        config.restore_delay_ms
    );

    paste_text_with_handlers(
        text,
        config,
        read_clipboard,
        set_clipboard,
        send_paste_key,
        thread::sleep,
    )
}

fn paste_text_with_handlers<R, S, K, L>(
    text: &str,
    config: &PasteConfig,
    mut read_clipboard: R,
    mut set_clipboard: S,
    mut send_paste_key: K,
    mut sleep: L,
) -> anyhow::Result<()>
where
    R: FnMut() -> anyhow::Result<String>,
    S: FnMut(&str) -> anyhow::Result<()>,
    K: FnMut(&str) -> anyhow::Result<()>,
    L: FnMut(Duration),
{
    let previous_clipboard = if config.restore_clipboard {
        match read_clipboard() {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!("Failed to read clipboard before paste: {err}");
                None
            }
        }
    } else {
        None
    };

    set_clipboard(text).context("failed to copy text to clipboard for paste")?;
    tracing::debug!("Paste mode: copied {} bytes to clipboard", text.len());

    #[cfg(not(target_os = "macos"))]
    log_active_window("before paste key");
    if let Err(err) = send_paste_key(&config.paste_key) {
        tracing::warn!("Failed to send paste key '{}': {err}", config.paste_key);
        return Err(err.context(paste_key_failure_message(&config.paste_key)));
    }

    sleep(Duration::from_millis(config.restore_delay_ms));

    if let Some(previous_clipboard) = previous_clipboard {
        if let Err(err) = set_clipboard(&previous_clipboard) {
            tracing::warn!("Failed to restore clipboard after paste: {err}");
            eprintln!("Warning: Failed to restore previous clipboard contents after paste.");
        }
    }

    Ok(())
}

fn paste_key_failure_message(paste_key: &str) -> String {
    paste_key_failure_message_for_os(paste_key, cfg!(target_os = "macos"))
}

fn paste_key_failure_message_for_os(paste_key: &str, is_macos: bool) -> String {
    let mut message =
        format!("Failed to send paste key '{paste_key}'. Text was copied to the clipboard.");
    if is_macos {
        message.push_str("\nNext step: ");
        message.push_str(MACOS_ACCESSIBILITY_REMEDIATION);
    }
    message
}

pub(crate) fn spawn_detached_paste_helper(text: &str) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("failed to resolve ostt executable path")?;
    let mut command = Command::new(exe);
    command
        .arg("paste-helper")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach_from_terminal_process_group(&mut command);

    let mut child = command.spawn().context("failed to spawn paste helper")?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to open paste helper stdin"))?;
    stdin
        .write_all(text.as_bytes())
        .context("failed to write text to paste helper")?;
    drop(stdin);
    drop(child);

    Ok(())
}

#[cfg(unix)]
fn detach_from_terminal_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(unix))]
fn detach_from_terminal_process_group(_command: &mut Command) {}

pub(crate) fn handle_paste_helper(config: &crate::config::OsttConfig) -> anyhow::Result<()> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("failed to read paste helper input")?;
    wait_for_focus_after_popup(&config.output.paste);
    paste_text(&text, &config.output.paste).inspect_err(|err| {
        notify_no_popup_error_if_popup_context("Paste Failed", &err.to_string());
    })
}

#[cfg(not(target_os = "macos"))]
fn log_active_window(label: &str) {
    if let Ok(output) = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            tracing::debug!("Paste mode: active window {label}: {}", text.trim());
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn active_hyprland_window_title() -> Option<String> {
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    value
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn send_paste_key(paste_key: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        send_macos_key(paste_key)
    }

    #[cfg(not(target_os = "macos"))]
    {
        send_linux_key(paste_key)
    }
}

#[cfg(target_os = "macos")]
fn send_macos_key(paste_key: &str) -> anyhow::Result<()> {
    let (modifiers, key) = parse_paste_key(paste_key)?;
    let mut using_parts = Vec::new();
    for modifier in modifiers {
        using_parts.push(match modifier.as_str() {
            "cmd" => "command down",
            "ctrl" => "control down",
            "shift" => "shift down",
            "alt" => "option down",
            "super" => "command down",
            _ => anyhow::bail!("Unsupported macOS paste modifier '{modifier}'"),
        });
    }

    let script = if using_parts.is_empty() {
        format!("tell application \"System Events\" to keystroke \"{key}\"")
    } else {
        format!(
            "tell application \"System Events\" to keystroke \"{key}\" using {{{}}}",
            using_parts.join(", ")
        )
    };

    run_status(Command::new("osascript").args(["-e", &script]), "osascript")
}

#[cfg(not(target_os = "macos"))]
fn send_linux_key(paste_key: &str) -> anyhow::Result<()> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        if let Ok(()) = send_wtype_key(paste_key) {
            return Ok(());
        }
    }

    send_xdotool_key(paste_key)
}

#[cfg(not(target_os = "macos"))]
fn send_wtype_key(paste_key: &str) -> anyhow::Result<()> {
    let (modifiers, key) = parse_paste_key(paste_key)?;
    let key = linux_key_name(&key);
    let mut args = Vec::new();
    for modifier in &modifiers {
        args.push("-M".to_string());
        args.push(wtype_modifier(modifier)?.to_string());
    }
    args.push("-P".to_string());
    args.push(key.clone());
    args.push("-p".to_string());
    args.push(key);
    for modifier in modifiers.iter().rev() {
        args.push("-m".to_string());
        args.push(wtype_modifier(modifier)?.to_string());
    }

    tracing::debug!("Paste mode: running wtype {:?}", args);
    run_status(Command::new("wtype").args(args), "wtype")
}

#[cfg(not(target_os = "macos"))]
fn send_xdotool_key(paste_key: &str) -> anyhow::Result<()> {
    let (modifiers, key) = parse_paste_key(paste_key)?;
    let mut parts: Vec<String> = modifiers
        .iter()
        .map(|modifier| xdotool_modifier(modifier).map(str::to_string))
        .collect::<anyhow::Result<_>>()?;
    parts.push(linux_key_name(&key));
    tracing::debug!("Paste mode: running xdotool key {}", parts.join("+"));
    run_status(
        Command::new("xdotool").args(["key", &parts.join("+")]),
        "xdotool",
    )
}

fn parse_paste_key(paste_key: &str) -> anyhow::Result<(Vec<String>, String)> {
    let parts: Vec<_> = paste_key
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect();
    if parts.len() < 2 {
        anyhow::bail!("paste_key must be a key combination like 'ctrl+v'");
    }
    Ok((
        parts[..parts.len() - 1].to_vec(),
        parts[parts.len() - 1].clone(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn linux_key_name(key: &str) -> String {
    if key == "insert" {
        "Insert".to_string()
    } else {
        key.to_string()
    }
}

#[cfg(not(target_os = "macos"))]
fn wtype_modifier(modifier: &str) -> anyhow::Result<&'static str> {
    match modifier {
        "ctrl" => Ok("ctrl"),
        "shift" => Ok("shift"),
        "alt" => Ok("alt"),
        "super" => Ok("logo"),
        "cmd" => Ok("logo"),
        _ => anyhow::bail!("Unsupported paste_key modifier '{modifier}'"),
    }
}

#[cfg(not(target_os = "macos"))]
fn xdotool_modifier(modifier: &str) -> anyhow::Result<&'static str> {
    match modifier {
        "ctrl" => Ok("ctrl"),
        "shift" => Ok("shift"),
        "alt" => Ok("alt"),
        "super" => Ok("Super_L"),
        "cmd" => Ok("Super_L"),
        _ => anyhow::bail!("Unsupported paste_key modifier '{modifier}'"),
    }
}

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
    fn paste_key_failure_returns_error_and_leaves_text_in_clipboard() {
        let config = PasteConfig {
            restore_clipboard: true,
            restore_delay_ms: 0,
            ..PasteConfig::default()
        };
        let clipboard_writes = RefCell::new(Vec::new());

        let err = paste_text_with_handlers(
            "new text",
            &config,
            || Ok("old text".to_string()),
            |value| {
                clipboard_writes.borrow_mut().push(value.to_string());
                Ok(())
            },
            |_| Err(anyhow::anyhow!("xdotool failed")),
            |_| {},
        )
        .unwrap_err();

        assert!(err.to_string().contains("Text was copied to the clipboard"));
        assert_eq!(clipboard_writes.into_inner(), vec!["new text"]);
    }

    #[test]
    fn macos_paste_failure_message_mentions_accessibility() {
        let message = paste_key_failure_message_for_os("cmd+v", true);

        assert!(message.contains("Privacy & Security > Accessibility"));
        assert!(message.contains("OSTT launcher"));
    }

    #[test]
    fn non_macos_paste_failure_message_omits_accessibility() {
        let message = paste_key_failure_message_for_os("ctrl+v", false);

        assert!(!message.contains("Privacy & Security > Accessibility"));
    }

    #[test]
    fn popup_context_notification_helper_only_notifies_in_popup_context() {
        let calls = RefCell::new(Vec::new());

        let notified = notify_no_popup_error_if_popup_context_with(
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

        let skipped = notify_no_popup_error_if_popup_context_with(
            "Paste Failed",
            "paste failed",
            false,
            |_, _| panic!("notification should not run outside popup context"),
        );

        assert!(!skipped);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_focus_detection_matches_only_popup_terminal_window() {
        assert!(is_macos_popup_window("Ghostty", "ostt"));
        assert!(is_macos_popup_window("kitty", "ostt"));
        assert!(!is_macos_popup_window("Ghostty", "notes"));
        assert!(!is_macos_popup_window("Safari", "ostt"));
    }
}
