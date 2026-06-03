use crate::clipboard::{read_clipboard, set_clipboard};
use crate::config::PasteConfig;
use anyhow::Context;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(not(target_os = "macos"))]
const POPUP_TITLE: &str = "ostt";

pub(crate) fn wait_for_focus_after_popup(config: &PasteConfig) {
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

pub(crate) fn paste_text(text: &str, config: &PasteConfig) -> anyhow::Result<()> {
    tracing::debug!(
        "Paste mode: paste_key='{}', restore_clipboard={}, restore_delay_ms={}",
        config.paste_key,
        config.restore_clipboard,
        config.restore_delay_ms
    );

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
        eprintln!(
            "Warning: Failed to send paste key '{}'. Text was copied to the clipboard.",
            config.paste_key
        );
        return Ok(());
    }

    thread::sleep(Duration::from_millis(config.restore_delay_ms));

    if let Some(previous_clipboard) = previous_clipboard {
        if let Err(err) = set_clipboard(&previous_clipboard) {
            tracing::warn!("Failed to restore clipboard after paste: {err}");
            eprintln!("Warning: Failed to restore previous clipboard contents after paste.");
        }
    }

    Ok(())
}

pub(crate) fn spawn_detached_paste_helper(text: &str) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("failed to resolve ostt executable path")?;
    let mut command = Command::new(exe);
    command
        .arg("__paste")
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
    paste_text(&text, &config.output.paste)
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
        return send_macos_key(paste_key);
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
