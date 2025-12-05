//! Structured logging for ostt using the tracing crate.
//!
//! Configures a rolling file logger that writes to daily-rotated log files.
//! Follows the XDG Base Directory Specification for log file placement.
//! Does not output to terminal to avoid interfering with the TUI.

use dirs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing_appender::rolling;
use tracing_subscriber::prelude::*;

/// Global non-blocking guard holder to keep the appender alive for the program lifetime.
static APPENDER_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Initializes the logging system with file-based output.
///
/// Sets up a non-blocking rolling file appender that rotates daily.
/// Log level is controlled by the RUST_LOG environment variable (defaults to "info").
///
/// # Errors
/// - If the log directory cannot be determined or created
/// - If the subscriber initialization fails
pub fn init_logging() -> Result<(), anyhow::Error> {
    let log_dir = get_log_dir()?;

    let file_appender = rolling::daily(&log_dir, "ostt.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Store the guard in a static to keep it alive for the program lifetime
    APPENDER_GUARD
        .set(guard)
        .map_err(|_| anyhow::anyhow!("Logging already initialized"))?;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(true)
                .with_ansi(false),
        )
        .init();

    tracing::info!("Logging initialized. Log file: {}", log_dir.display());
    Ok(())
}

/// Determines the log directory, following XDG Base Directory Specification.
///
/// Prefers XDG_STATE_HOME if set, otherwise uses ~/.local/state/ostt.
///
/// # Errors
/// - If home directory cannot be determined
/// - If log directory cannot be created
fn get_log_dir() -> Result<PathBuf, anyhow::Error> {
    let log_dir = if let Ok(xdg_state) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(xdg_state).join("ostt")
    } else {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        home.join(".local/state/ostt")
    };

    std::fs::create_dir_all(&log_dir)?;

    Ok(log_dir)
}
