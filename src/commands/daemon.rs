//! Handlers for `ostt daemon <subcommand>`.
//!
//! The daemon keeps a local Whisper model loaded in memory between transcriptions,
//! eliminating the model-load cost on every call. It always serves the currently
//! active local model (as configured by `ostt model`).
//!
//! # Service management
//! `install`/`uninstall` write platform-specific service files and register them
//! with the system service manager (launchd on macOS, systemd --user on Linux).
//! When installed, the daemon starts automatically at login and restarts on failure.
//!
//! # Logs
//! Daemon activity is written to the regular OSTT logs. Use `ostt logs` to inspect it.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context};

use crate::config::OsttConfig;
use crate::transcription::daemon_client::{ensure_daemon, probe_daemon, shutdown_daemon};
use crate::transcription::local_models::{daemon_pid_path, daemon_socket_path};

// ── Internal run command ──────────────────────────────────────────────────────

/// [Internal] Run the daemon process for the currently active local model.
///
/// This is called by the service manager and by `daemon start`. It reads the
/// active model from config, loads it, and serves requests until stopped.
pub async fn handle_daemon_run(
    model_id: Option<String>,
    idle_timeout_secs: Option<u64>,
) -> anyhow::Result<()> {
    // model_id may be overridden by the caller (e.g. spawn_daemon_process passes it),
    // but falls back to the config active model for service-managed invocations.
    let model_id = model_id
        .or_else(active_local_model)
        .ok_or_else(|| anyhow!("No active local model configured. Run 'ostt model' first."))?;
    crate::transcription::daemon::run(&model_id, idle_timeout_secs).await
}

// ── User-facing subcommands ───────────────────────────────────────────────────

/// Start the daemon for the currently active local model.
pub async fn handle_daemon_start(config: &OsttConfig) -> anyhow::Result<()> {
    let model_id = require_active_model(config)?;

    if let Some(info) = probe_daemon().await {
        if info.model_id == model_id {
            println!("Daemon is running (model: {model_id}).");
            return Ok(());
        }
        println!("Stopping daemon (currently loaded: {})…", info.model_id);
        shutdown_daemon().await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("Starting daemon for model '{model_id}'…");
    ensure_daemon(&model_id, None).await?;
    println!("Daemon started.");
    Ok(())
}

/// Stop the running daemon.
pub async fn handle_daemon_stop() -> anyhow::Result<()> {
    if probe_daemon().await.is_none() {
        println!("Daemon is not running.");
        return Ok(());
    }
    shutdown_daemon().await?;
    // Brief wait for the process to exit and remove its socket/PID files.
    tokio::time::sleep(Duration::from_millis(300)).await;
    println!("Daemon stopped.");
    Ok(())
}

/// Restart the daemon with the currently active local model.
pub async fn handle_daemon_restart(config: &OsttConfig) -> anyhow::Result<()> {
    let model_id = require_active_model(config)?;
    let _ = shutdown_daemon().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("Starting daemon for model '{model_id}'…");
    ensure_daemon(&model_id, None).await?;
    println!("Daemon restarted.");
    Ok(())
}

/// Print daemon status.
pub async fn handle_daemon_status(config: &OsttConfig) -> anyhow::Result<()> {
    let active = active_local_model_from_config(config);
    let info = probe_daemon().await;
    let pid = read_pid_file();

    match &info {
        Some(d) => {
            println!("Status:  running");
            if let Some(p) = pid {
                println!("PID:     {p}");
            }
            println!("Socket:  {}", daemon_socket_path().display());
            let mismatch = active.as_deref().is_some_and(|m| m != d.model_id);
            if mismatch {
                println!(
                    "Model:   {} (active: {})",
                    d.model_id,
                    active.as_deref().unwrap_or("none")
                );
                println!("Warning: run 'ostt daemon restart' to load the active model");
            } else {
                println!("Model:   {}", d.model_id);
            }
        }
        None => {
            println!("Status:  stopped");
            println!("Model:   {}", active.as_deref().unwrap_or("(none)"));
        }
    }

    let service_path = service_file_path();
    let installed = service_path.exists();
    println!(
        "Service: {}",
        if installed {
            format!("installed ({})", service_path.display())
        } else {
            "not installed".to_string()
        }
    );

    println!("Log:     ostt logs");

    Ok(())
}

/// Install the daemon as a system service (launchd on macOS, systemd on Linux).
pub fn handle_daemon_install() -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("could not determine ostt executable path")?;

    let service_path = service_file_path();
    std::fs::create_dir_all(
        service_path
            .parent()
            .unwrap_or(std::path::Path::new("/tmp")),
    )?;

    #[cfg(target_os = "macos")]
    {
        let plist = macos_plist_content(&exe);
        std::fs::write(&service_path, &plist)
            .with_context(|| format!("could not write plist to {}", service_path.display()))?;
        let status = std::process::Command::new("launchctl")
            .args(["load", &service_path.to_string_lossy()])
            .status()
            .context("launchctl load failed")?;
        anyhow::ensure!(status.success(), "launchctl load returned non-zero");
        println!("Service installed and loaded: {}", service_path.display());
        println!("The daemon will start automatically at login.");
    }

    #[cfg(target_os = "linux")]
    {
        let unit = linux_systemd_content(&exe);
        std::fs::write(&service_path, &unit)
            .with_context(|| format!("could not write unit to {}", service_path.display()))?;
        for args in [
            vec!["--user", "daemon-reload"],
            vec!["--user", "enable", "ostt-daemon"],
            vec!["--user", "start", "ostt-daemon"],
        ] {
            let status = std::process::Command::new("systemctl")
                .args(&args)
                .status()
                .with_context(|| format!("systemctl {:?} failed", args))?;
            anyhow::ensure!(status.success(), "systemctl {:?} returned non-zero", args);
        }
        println!("Service installed and started: {}", service_path.display());
        println!("The daemon will start automatically at login.");
        println!("View logs: journalctl --user -u ostt-daemon -f");
    }

    Ok(())
}

/// Remove the daemon system service.
pub fn handle_daemon_uninstall() -> anyhow::Result<()> {
    let service_path = service_file_path();
    if !service_path.exists() {
        println!("Service is not installed.");
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        // Ignore errors from unload (service may already be stopped).
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &service_path.to_string_lossy()])
            .status();
        std::fs::remove_file(&service_path)
            .with_context(|| format!("could not remove {}", service_path.display()))?;
        println!("Service uninstalled.");
    }

    #[cfg(target_os = "linux")]
    {
        for args in [
            vec!["--user", "stop", "ostt-daemon"],
            vec!["--user", "disable", "ostt-daemon"],
        ] {
            // Ignore errors (service may not be running).
            let _ = std::process::Command::new("systemctl").args(&args).status();
        }
        std::fs::remove_file(&service_path)
            .with_context(|| format!("could not remove {}", service_path.display()))?;
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        println!("Service uninstalled.");
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn active_local_model() -> Option<String> {
    crate::config::get_selected_model_entry()
        .ok()
        .flatten()
        .filter(|m| m.provider_id == "whisper")
        .map(|m| m.model_id)
}

fn active_local_model_from_config(config: &OsttConfig) -> Option<String> {
    match (
        config.transcription.provider.as_deref(),
        config.transcription.model.as_deref(),
    ) {
        (Some("whisper") | Some("local"), Some(model_id)) => Some(model_id.to_string()),
        _ => None,
    }
}

fn require_active_model(config: &OsttConfig) -> anyhow::Result<String> {
    active_local_model_from_config(config).ok_or_else(|| {
        anyhow!("No local model is active. Run 'ostt model' to download and activate one.")
    })
}

fn read_pid_file() -> Option<u32> {
    std::fs::read_to_string(daemon_pid_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn service_file_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library")
            .join("LaunchAgents")
            .join("ai.ostt.daemon.plist")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".config")
            .join("systemd")
            .join("user")
            .join("ostt-daemon.service")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("/tmp/ostt-daemon.service")
    }
}

#[cfg(target_os = "macos")]
fn macos_plist_content(exe: &std::path::Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.ostt.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>/dev/null</string>
    <key>StandardErrorPath</key>
    <string>/dev/null</string>
</dict>
</plist>
"#,
        exe = exe.display(),
    )
}

#[cfg(target_os = "linux")]
fn linux_systemd_content(exe: &std::path::Path) -> String {
    format!(
        "[Unit]\n\
         Description=OSTT local transcription model daemon\n\
         After=default.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={exe} daemon run\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         StandardOutput=null\n\
         StandardError=null\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe = exe.display(),
    )
}
