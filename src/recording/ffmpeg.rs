//! FFmpeg locator utility.
//!
//! Provides ffmpeg binary discovery. Checks standard installation locations
//! before falling back to PATH search. This ensures ffmpeg can be found
//! even when running in environments with limited PATH setup (e.g., iTerm commands).

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Locates the ffmpeg binary on the system.
///
/// Checks in this order:
/// 1. macOS homebrew locations: `/opt/homebrew/bin/ffmpeg`, `/usr/local/bin/ffmpeg`
/// 2. Linux standard locations: `/usr/bin/ffmpeg`, `/usr/local/bin/ffmpeg`
/// 3. Falls back to PATH search via `which`
///
/// # Returns
/// The path to the ffmpeg binary, or an error if not found.
pub fn find_ffmpeg() -> Result<PathBuf> {
    // Check common installation locations by platform
    let candidates = if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/opt/homebrew/bin/ffmpeg"), // Apple Silicon Homebrew
            PathBuf::from("/usr/local/bin/ffmpeg"),    // Intel Homebrew or manual install
            PathBuf::from("/usr/bin/ffmpeg"),          // Direct system install
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            PathBuf::from("/usr/bin/ffmpeg"),       // Standard Linux
            PathBuf::from("/usr/local/bin/ffmpeg"), // Manual install
            PathBuf::from("/snap/bin/ffmpeg"),      // Snap installation
        ]
    } else {
        vec![] // For other platforms, rely on PATH search
    };

    // Check each candidate location
    for path in candidates {
        if path.exists() {
            tracing::debug!("Found ffmpeg at: {}", path.display());
            return Ok(path);
        }
    }

    // Fall back to PATH search using system commands
    let ffmpeg_path = find_in_path("ffmpeg")?;
    tracing::debug!("Found ffmpeg in PATH at: {}", ffmpeg_path.display());
    Ok(ffmpeg_path)
}

/// Searches for a binary in the system PATH.
fn find_in_path(binary_name: &str) -> Result<PathBuf> {
    let output = std::process::Command::new("which")
        .arg(binary_name)
        .output()
        .map_err(|e| anyhow!("failed to search PATH for {binary_name}: {e}"))?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(path_str.trim());
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }

    Err(anyhow!(missing_ffmpeg_message()))
}

fn missing_ffmpeg_message() -> String {
    missing_ffmpeg_message_for(current_os(), homebrew_likely_available())
}

fn current_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    }
}

fn homebrew_likely_available() -> bool {
    Path::new("/opt/homebrew/bin/brew").exists()
        || Path::new("/usr/local/bin/brew").exists()
        || command_exists("brew")
}

fn command_exists(binary_name: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary_name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn missing_ffmpeg_message_for(os: &str, homebrew_available: bool) -> String {
    match os {
        "macos" if homebrew_available => {
            "ffmpeg not found. Install it with Homebrew: brew install ffmpeg.".to_string()
        }
        "macos" => "ffmpeg not found. Install Homebrew from https://brew.sh, then run: brew install ffmpeg.".to_string(),
        "linux" => "ffmpeg not found. Install it with your package manager, for example: sudo apt install ffmpeg, sudo dnf install ffmpeg, or sudo pacman -S ffmpeg.".to_string(),
        _ => "ffmpeg not found. Install ffmpeg from https://ffmpeg.org/download.html, then retry.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_ffmpeg() {
        // This test will succeed if ffmpeg is installed
        match find_ffmpeg() {
            Ok(path) => println!("Found ffmpeg at: {}", path.display()),
            Err(e) => println!("ffmpeg not found (expected on CI): {e}"),
        }
    }

    #[test]
    fn macos_ffmpeg_message_uses_homebrew_when_available() {
        let message = missing_ffmpeg_message_for("macos", true);

        assert!(message.contains("brew install ffmpeg"));
        assert!(!message.contains("https://brew.sh"));
    }

    #[test]
    fn macos_ffmpeg_message_guides_homebrew_install_when_unavailable() {
        let message = missing_ffmpeg_message_for("macos", false);

        assert!(message.contains("https://brew.sh"));
        assert!(message.contains("brew install ffmpeg"));
    }

    #[test]
    fn linux_ffmpeg_message_includes_package_manager_examples() {
        let message = missing_ffmpeg_message_for("linux", false);

        assert!(message.contains("sudo apt install ffmpeg"));
        assert!(message.contains("sudo dnf install ffmpeg"));
        assert!(message.contains("sudo pacman -S ffmpeg"));
    }
}
