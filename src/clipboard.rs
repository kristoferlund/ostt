//! Clipboard utilities for ostt.
//!
//! Handles copying transcribed text to system clipboard using pbcopy (macOS), wl-copy (Wayland), or xclip (X11).

use anyhow::Context;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Copies text to system clipboard using pbcopy (macOS), wl-copy (Wayland/Hyprland), or xclip (X11).
///
/// Attempts pbcopy first on macOS, wl-copy for Wayland environments, then falls back to xclip for X11.
/// Does not fail if clipboard is unavailable, allowing transcription to succeed regardless.
///
/// # Errors
/// - If no clipboard tool is available (warning only, not an error)
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                match write!(stdin, "{text}") {
                    Ok(_) => {
                        drop(stdin);
                        thread::sleep(Duration::from_millis(100));
                        tracing::debug!("Transcribed text copied to clipboard via pbcopy");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to write to pbcopy stdin: {e}");
                    }
                }
            }
        } else {
            tracing::debug!("pbcopy not found or not executable");
        }
    }
    // Only try wl-copy on Wayland sessions — it spawns successfully on X11
    // but silently fails to copy since there's no Wayland compositor.
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    if is_wayland {
        if let Ok(mut child) = Command::new("wl-copy")
            .args(["--type", "text/plain", "--trim-newline"])
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                match write!(stdin, "{text}") {
                    Ok(_) => {
                        drop(stdin);
                        thread::sleep(Duration::from_millis(100));
                        tracing::debug!("Transcribed text copied to clipboard via wl-copy");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to write to wl-copy stdin: {}", e);
                    }
                }
            }
        } else {
            tracing::debug!("wl-copy not found or not executable");
        }
    }

    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard", "-in", "-quiet"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            match write!(stdin, "{text}") {
                Ok(_) => {
                    drop(stdin);
                    thread::sleep(Duration::from_millis(100));
                    tracing::debug!("Transcribed text copied to clipboard via xclip");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Failed to write to xclip stdin: {}", e);
                }
            }
        }
    } else {
        tracing::debug!("xclip not found or not executable");
    }

    #[cfg(target_os = "macos")]
    tracing::warn!("No clipboard tool available (pbcopy not found)");
    #[cfg(not(target_os = "macos"))]
    tracing::warn!("No clipboard tool available (wl-copy or xclip not found)");
    Ok(())
}

pub(crate) fn read_clipboard() -> anyhow::Result<String> {
    #[cfg(target_os = "macos")]
    {
        return read_command("pbpaste", &[]);
    }

    #[cfg(not(target_os = "macos"))]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            if let Ok(text) = read_command("wl-paste", &["--no-newline"]) {
                return Ok(text);
            }
        }

        read_command("xclip", &["-selection", "clipboard", "-out"])
    }
}

pub(crate) fn set_clipboard(text: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return write_command("pbcopy", &[], text);
    }

    #[cfg(not(target_os = "macos"))]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            if write_command("wl-copy", &["--type", "text/plain", "--trim-newline"], text).is_ok() {
                return Ok(());
            }
        }

        write_command("xclip", &["-selection", "clipboard", "-in", "-quiet"], text)
    }
}

fn read_command(program: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run {program}"))?;
    if !output.status.success() {
        anyhow::bail!("{program} failed with status {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn write_command(program: &str, args: &[&str], text: &str) -> anyhow::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to run {program}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to open {program} stdin"))?;
    stdin
        .write_all(text.as_bytes())
        .with_context(|| format!("Failed to write to {program}"))?;
    drop(stdin);

    let status = child
        .wait()
        .with_context(|| format!("Failed to wait for {program}"))?;
    if !status.success() {
        anyhow::bail!("{program} failed with status {status}");
    }
    Ok(())
}
