//! Launch ostt in a popup terminal window.
//!
//! Spawns a terminal emulator with ostt running inside it. If an ostt instance
//! is already running (tracked via PID file), sends SIGUSR1 to finish recording
//! instead of spawning a new instance.

use anyhow::{anyhow, Context};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::file::PopupConfig;
use crate::config::OsttConfig;

// ─── PID file management ───────────────────────────────────────────────────

/// Returns the path to the PID file: ~/.local/share/ostt/launch.pid
fn pid_file_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let dir = home.join(".local").join("share").join("ostt");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("launch.pid"))
}

/// Reads the PID from the PID file. Returns None if file doesn't exist or is invalid.
fn read_pid() -> Option<u32> {
    let path = pid_file_path().ok()?;
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse().ok()
}

/// Writes a PID to the PID file.
fn write_pid(pid: u32) -> anyhow::Result<()> {
    let path = pid_file_path()?;
    fs::write(&path, pid.to_string())?;
    Ok(())
}

/// Removes the PID file.
fn remove_pid_file() -> anyhow::Result<()> {
    let path = pid_file_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Checks if a process with the given PID is alive.
fn is_process_alive(pid: u32) -> bool {
    // kill -0 checks if process exists without sending a signal
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Walk the process tree from a given PID to find the leaf (deepest child).
/// This handles the Ghostty -> login -> ostt chain on macOS.
fn find_leaf_pid(pid: u32) -> u32 {
    let mut current = pid;
    for _ in 0..5 {
        let output = Command::new("pgrep")
            .args(["-P", &current.to_string()])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                match stdout.trim().lines().next().and_then(|l| l.trim().parse::<u32>().ok()) {
                    Some(child) => current = child,
                    None => break,
                }
            }
            _ => break,
        }
    }
    current
}

/// Sends SIGUSR1 to finish recording on a running ostt instance.
/// Walks the process tree to find the actual ostt process.
fn signal_running_ostt(terminal_pid: u32) -> anyhow::Result<()> {
    let ostt_pid = find_leaf_pid(terminal_pid);
    tracing::info!("Sending SIGUSR1 to PID {} (leaf of {})", ostt_pid, terminal_pid);

    let status = Command::new("kill")
        .args(["-USR1", &ostt_pid.to_string()])
        .status()
        .context("Failed to send SIGUSR1")?;

    if !status.success() {
        return Err(anyhow!("Failed to send SIGUSR1 to PID {}", ostt_pid));
    }
    Ok(())
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
        // On macOS, Ghostty might be in /Applications
        if matches!(self, Self::Ghostty) {
            let app_path = "/Applications/Ghostty.app/Contents/MacOS/ghostty";
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
            // Preferred: modern, feature-rich, cross-platform
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
            tracing::info!("Auto-detected terminal: {} ({})", terminal.command_name(), binary);
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
            let mut ostt_cmd = ostt_bin.to_string();
            for arg in ostt_args {
                ostt_cmd.push(' ');
                ostt_cmd.push_str(arg);
            }

            let shell_cmd = format!(
                "source ~/.bash_profile 2>/dev/null || source ~/.zprofile 2>/dev/null || source ~/.profile 2>/dev/null; clear; exec {}",
                ostt_cmd
            );

            let mut args = vec![
                binary.to_string(),
                format!("--window-position-x={}", config.x),
                format!("--window-position-y={}", config.y),
                format!("--window-width={}", config.width),
                format!("--window-height={}", config.height),
                format!("--font-size={}", config.font_size),
                "--background=#000000".to_string(),
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
                "--class".to_string(),
                "ostt-popup".to_string(),
                "-o".to_string(),
                "remember_window_size=no".to_string(),
                "-o".to_string(),
                format!("initial_window_width={}", config.width),
                "-o".to_string(),
                format!("initial_window_height={}", config.height),
                "-o".to_string(),
                format!("font_size={}", config.font_size),
                "-o".to_string(),
                "background=#000000".to_string(),
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
                format!("--geometry={}x{}+{}+{}", config.width, config.height, config.x, config.y),
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
            let mut cmd = ostt_bin.to_string();
            for arg in ostt_args {
                cmd.push(' ');
                cmd.push_str(arg);
            }
            args.push(cmd);
            args
        }
    }
}

// ─── Public handler ─────────────────────────────────────────────────────────

/// Handles the `ostt launch` command.
///
/// If an ostt instance is already running (tracked via PID file), sends SIGUSR1
/// to finish recording. Otherwise, spawns a new terminal window with ostt.
pub async fn handle_launch(args: Vec<String>) -> Result<(), anyhow::Error> {
    // Check if there's already a running instance
    if let Some(pid) = read_pid() {
        if is_process_alive(pid) {
            tracing::info!("Found running ostt instance (terminal PID {}), sending SIGUSR1", pid);
            signal_running_ostt(pid)?;
            return Ok(());
        }
        // Stale PID file, clean up
        tracing::debug!("Stale PID file (PID {} no longer running), cleaning up", pid);
        remove_pid_file()?;
    }

    // Load config for popup settings
    let config = OsttConfig::load().map_err(|e| anyhow!("Failed to load config: {e}"))?;
    let popup = &config.popup;

    // Detect terminal
    let (terminal, binary) = detect_terminal(popup)?;
    tracing::info!("Using terminal: {} ({})", terminal.command_name(), binary);

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
        .spawn()
        .with_context(|| format!("Failed to spawn {}", terminal.command_name()))?;

    let terminal_pid = child.id();
    tracing::info!("Terminal spawned with PID {}", terminal_pid);

    // Write PID file for toggle support
    write_pid(terminal_pid)?;

    // Detach the child process — we don't wait for it.
    // The PID file is cleaned up on the next invocation if the process has exited
    // (stale PID detection above), or when ostt launch is used as a toggle.
    drop(child);

    // Exit the process immediately so the caller (hotkey, shell) doesn't block.
    std::process::exit(0);
}
