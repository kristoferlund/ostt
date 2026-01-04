//! FFmpeg locator utility.
//!
//! Provides cross-platform ffmpeg binary discovery. Checks standard installation
//! locations before falling back to PATH search. This ensures ffmpeg can be found
//! even when running in environments with limited PATH setup (e.g., iTerm commands).

use std::path::PathBuf;
use anyhow::{anyhow, Result};

/// Locates the ffmpeg binary on the system.
///
/// Checks in this order:
/// 1. macOS homebrew locations: `/opt/homebrew/bin/ffmpeg`, `/usr/local/bin/ffmpeg`
/// 2. Linux standard locations: `/usr/bin/ffmpeg`, `/usr/local/bin/ffmpeg`
/// 3. Windows standard locations: `C:\ffmpeg\bin\ffmpeg.exe`
/// 4. Falls back to PATH search via `which` or `where` command
///
/// # Returns
/// The path to the ffmpeg binary, or an error if not found.
pub fn find_ffmpeg() -> Result<PathBuf> {
    // Check common installation locations by platform
    let candidates = if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/opt/homebrew/bin/ffmpeg"),      // Apple Silicon Homebrew
            PathBuf::from("/usr/local/bin/ffmpeg"),         // Intel Homebrew or manual install
            PathBuf::from("/usr/bin/ffmpeg"),               // Direct system install
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            PathBuf::from("/usr/bin/ffmpeg"),               // Standard Linux
            PathBuf::from("/usr/local/bin/ffmpeg"),         // Manual install
            PathBuf::from("/snap/bin/ffmpeg"),              // Snap installation
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe"),
            PathBuf::from("C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe"),
            PathBuf::from("C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe"),
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
///
/// Uses `which` on Unix systems and `where` on Windows.
fn find_in_path(binary_name: &str) -> Result<PathBuf> {
    let search_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    let output = std::process::Command::new(search_cmd)
        .arg(binary_name)
        .output()
        .map_err(|e| anyhow!("Failed to search PATH for {binary_name}: {e}"))?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(path_str.trim());
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "ffmpeg not found. Please install ffmpeg:\n\
         macOS: brew install ffmpeg\n\
         Linux: apt install ffmpeg (Debian/Ubuntu) or dnf install ffmpeg (Fedora)\n\
         Windows: Download from https://ffmpeg.org/download.html"
    ))
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
}
