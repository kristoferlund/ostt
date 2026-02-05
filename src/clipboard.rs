//! Clipboard utilities for ostt.
//!
//! Handles copying transcribed text to system clipboard using pbcopy (macOS), wl-copy (Wayland), or xclip (X11).

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
        if let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
        {
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
