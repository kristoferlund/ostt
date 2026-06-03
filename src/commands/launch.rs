//! Launch ostt in a popup terminal window.
//!
//! Spawns a terminal emulator with ostt running inside it. If an ostt instance
//! is already running, sends SIGUSR1 to finish recording instead of spawning
//! a new instance.

use anyhow::{anyhow, Context};
use std::process::{Command, Stdio};

use crate::config::file::PopupConfig;
use crate::recording::active;

/// Shell-quotes a string by wrapping in single quotes and escaping internal single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ─── Terminal detection and spawning ────────────────────────────────────────

/// Supported terminal emulators.
#[derive(Debug, Clone, Copy)]
enum TerminalEmulator {
    Ghostty,
    Kitty,
    Alacritty,
    Foot,
    Konsole,
    GnomeTerminal,
    Xfce4Terminal,
}

impl TerminalEmulator {
    /// Returns the command name for this terminal.
    fn command_name(&self) -> &'static str {
        match self {
            Self::Ghostty => "ghostty",
            Self::Kitty => "kitty",
            Self::Alacritty => "alacritty",
            Self::Foot => "foot",
            Self::Konsole => "konsole",
            Self::GnomeTerminal => "gnome-terminal",
            Self::Xfce4Terminal => "xfce4-terminal",
        }
    }

    /// Try to find this terminal on the system.
    fn find_binary(&self) -> Option<String> {
        // macOS automation contexts such as Shortcuts often use a minimal PATH.
        let app_path = match self {
            Self::Ghostty => Some("/Applications/Ghostty.app/Contents/MacOS/ghostty"),
            Self::Kitty => Some("/Applications/kitty.app/Contents/MacOS/kitty"),
            _ => None,
        };
        if let Some(app_path) = app_path {
            if std::path::Path::new(app_path).exists() {
                return Some(app_path.to_string());
            }
        }

        // Check PATH via `which`
        let output = Command::new("which")
            .arg(self.command_name())
            .output()
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
        None
    }

    /// Parse a terminal name from config string.
    fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "ghostty" => Some(Self::Ghostty),
            "kitty" => Some(Self::Kitty),
            "alacritty" => Some(Self::Alacritty),
            "foot" => Some(Self::Foot),
            "konsole" => Some(Self::Konsole),
            "gnome-terminal" => Some(Self::GnomeTerminal),
            "xfce4-terminal" => Some(Self::Xfce4Terminal),
            _ => None,
        }
    }

    /// Detection order: preferred terminals first, then platform defaults as fallbacks.
    fn detection_order() -> &'static [Self] {
        &[
            // Preferred: modern, feature-rich terminals
            Self::Ghostty,
            Self::Kitty,
            Self::Alacritty,
            // Fallbacks: platform defaults (already installed on their respective DEs)
            Self::Foot,
            Self::Konsole,
            Self::GnomeTerminal,
            Self::Xfce4Terminal,
        ]
    }
}

/// Detects the best available terminal emulator.
fn detect_terminal(config: &PopupConfig) -> anyhow::Result<(TerminalEmulator, String)> {
    // If user specified a terminal in config, use it
    if let Some(ref name) = config.terminal {
        let terminal = TerminalEmulator::from_name(name)
            .ok_or_else(|| anyhow!(
                "Unknown terminal '{}'. Supported: ghostty, kitty, alacritty, foot, konsole, gnome-terminal, xfce4-terminal",
                name
            ))?;
        let binary = terminal.find_binary()
            .ok_or_else(|| anyhow!(
                "Terminal '{}' not found. Install it or choose a different terminal in [popup] config.",
                name
            ))?;
        return Ok((terminal, binary));
    }

    // Auto-detect
    for terminal in TerminalEmulator::detection_order() {
        if let Some(binary) = terminal.find_binary() {
            tracing::debug!(
                "Auto-detected terminal: {} ({})",
                terminal.command_name(),
                binary
            );
            return Ok((*terminal, binary));
        }
    }

    Err(anyhow!(
        "No supported terminal emulator found.\n\
         Install one of: ghostty, kitty, alacritty\n\
         Or set the terminal in ~/.config/ostt/ostt.toml under [popup]."
    ))
}

/// Resolves the ostt binary path (the currently running executable).
fn ostt_binary_path() -> anyhow::Result<String> {
    std::env::current_exe()
        .context("Could not determine ostt binary path")?
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("ostt binary path contains invalid UTF-8"))
}

/// Builds the terminal command arguments for spawning ostt.
fn build_terminal_args(
    terminal: TerminalEmulator,
    binary: &str,
    config: &PopupConfig,
    ostt_bin: &str,
    ostt_args: &[String],
) -> Vec<String> {
    match terminal {
        TerminalEmulator::Ghostty => {
            // Ghostty uses a shell wrapper to source profile for PATH
            // (needed for bash processing actions that invoke external tools)
            let mut ostt_cmd = shell_quote(ostt_bin);
            for arg in ostt_args {
                ostt_cmd.push(' ');
                ostt_cmd.push_str(&shell_quote(arg));
            }

            let shell_cmd = format!(
                "source ~/.bash_profile 2>/dev/null || source ~/.zprofile 2>/dev/null || source ~/.profile 2>/dev/null; clear; exec {}",
                ostt_cmd
            );

            let mut args = vec![
                binary.to_string(),
                "--title=ostt".to_string(),
                format!("--window-position-x={}", config.x),
                format!("--window-position-y={}", config.y),
                format!("--window-width={}", config.width),
                format!("--window-height={}", config.height),
                format!("--font-size={}", config.font_size),
                "--background=#000000".to_string(),
                "--window-padding-x=0".to_string(),
                "--window-padding-y=0".to_string(),
                "--macos-window-shadow=false".to_string(),
            ];
            if config.borderless {
                args.push("--window-decoration=none".to_string());
            }
            args.extend([
                "-e".to_string(),
                "/bin/bash".to_string(),
                "-c".to_string(),
                shell_cmd,
            ]);
            args
        }
        TerminalEmulator::Kitty => {
            let mut args = vec![
                binary.to_string(),
                "--single-instance".to_string(),
                "--instance-group".to_string(),
                "ostt-popup".to_string(),
                "--title".to_string(),
                "ostt".to_string(),
                format!("--position={}x{}", config.x, config.y),
                "-o".to_string(),
                "remember_window_size=no".to_string(),
                "-o".to_string(),
                format!("initial_window_width={}c", config.width),
                "-o".to_string(),
                format!("initial_window_height={}c", config.height),
                "-o".to_string(),
                format!("font_size={}", config.font_size),
                "-o".to_string(),
                "background=#000000".to_string(),
                "-o".to_string(),
                "macos_quit_when_last_window_closed=yes".to_string(),
            ];
            if config.borderless {
                args.extend(["-o".to_string(), "hide_window_decorations=yes".to_string()]);
            }
            args.extend(["-e".to_string(), ostt_bin.to_string()]);
            args.extend(ostt_args.iter().cloned());
            args
        }
        TerminalEmulator::Alacritty => {
            let mut args = vec![
                binary.to_string(),
                "--class".to_string(),
                "ostt-popup".to_string(),
            ];
            args.extend(["-e".to_string(), ostt_bin.to_string()]);
            args.extend(ostt_args.iter().cloned());
            args
        }
        TerminalEmulator::Foot => {
            let mut args = vec![
                binary.to_string(),
                "--app-id".to_string(),
                "ostt-popup".to_string(),
                format!("--window-size-chars={}x{}", config.width, config.height),
            ];
            args.push(ostt_bin.to_string());
            args.extend(ostt_args.iter().cloned());
            args
        }
        TerminalEmulator::Konsole => {
            let mut args = vec![
                binary.to_string(),
                "-p".to_string(),
                format!("TerminalColumns={}", config.width),
                "-p".to_string(),
                format!("TerminalRows={}", config.height),
                "-e".to_string(),
                ostt_bin.to_string(),
            ];
            args.extend(ostt_args.iter().cloned());
            args
        }
        TerminalEmulator::GnomeTerminal => {
            let mut args = vec![
                binary.to_string(),
                format!(
                    "--geometry={}x{}+{}+{}",
                    config.width, config.height, config.x, config.y
                ),
                "--".to_string(),
                ostt_bin.to_string(),
            ];
            args.extend(ostt_args.iter().cloned());
            args
        }
        TerminalEmulator::Xfce4Terminal => {
            let mut args = vec![
                binary.to_string(),
                format!("--geometry={}x{}", config.width, config.height),
                "-e".to_string(),
            ];
            // xfce4-terminal -e takes a single string command
            let mut cmd = shell_quote(ostt_bin);
            for arg in ostt_args {
                cmd.push(' ');
                cmd.push_str(&shell_quote(arg));
            }
            args.push(cmd);
            args
        }
    }
}

// ─── Public handler ─────────────────────────────────────────────────────────

/// Handles the `ostt launch` command.
///
/// If an ostt recorder is already running, sends SIGUSR1
/// to finish recording. Otherwise, spawns a new terminal window with ostt.
pub async fn handle_launch(
    config: &crate::config::OsttConfig,
    args: Vec<String>,
) -> Result<(), anyhow::Error> {
    // Check if there's already a running recorder.
    if let Some(pid) = active::find_running_recorder() {
        tracing::debug!("Found running ostt recorder (PID {}), sending SIGUSR1", pid);
        active::signal_running_recorder(pid)?;
        return Ok(());
    }

    let popup = &config.popup;

    // Detect terminal
    let (terminal, binary) = detect_terminal(popup)?;
    tracing::debug!("Using terminal: {} ({})", terminal.command_name(), binary);

    // Get ostt binary path
    let ostt_bin = ostt_binary_path()?;

    // Build terminal arguments
    let all_args = build_terminal_args(terminal, &binary, popup, &ostt_bin, &args);

    // Spawn the terminal
    let program = &all_args[0];
    let spawn_args = &all_args[1..];

    tracing::debug!("Spawning: {} {:?}", program, spawn_args);

    let child = Command::new(program)
        .args(spawn_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to spawn {}", terminal.command_name()))?;

    tracing::debug!("Terminal spawned with PID {}", child.id());

    // Detach the child process — we don't wait for it.
    drop(child);

    // Exit the process immediately so the caller (hotkey, shell) doesn't block.
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kitty_args_use_supported_flags_and_cell_dimensions() {
        let args = build_terminal_args(
            TerminalEmulator::Kitty,
            "kitty",
            &PopupConfig::default(),
            "ostt",
            &["-c".to_string()],
        );

        assert!(!args.iter().any(|arg| arg == "--class"));
        assert!(args.iter().any(|arg| arg == "--single-instance"));
        assert!(args
            .windows(2)
            .any(|args| args == ["--instance-group", "ostt-popup"]));
        assert!(args.iter().any(|arg| arg == "--position=630x790"));
        assert!(args.iter().any(|arg| arg == "initial_window_width=90c"));
        assert!(args.iter().any(|arg| arg == "initial_window_height=15c"));
        assert!(args
            .iter()
            .any(|arg| arg == "macos_quit_when_last_window_closed=yes"));
        assert_eq!(args.last(), Some(&"-c".to_string()));
    }

    #[test]
    fn ghostty_args_do_not_use_invalid_class_flag() {
        let args = build_terminal_args(
            TerminalEmulator::Ghostty,
            "ghostty",
            &PopupConfig::default(),
            "ostt",
            &["--paste".to_string()],
        );

        assert!(!args.iter().any(|arg| arg == "--class=ostt-popup"));
        assert!(args.iter().any(|arg| arg == "--title=ostt"));
        assert!(args.last().is_some_and(|arg| arg.contains("'--paste'")));
    }
}
