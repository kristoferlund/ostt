//! Application directory helpers following XDG Base Directory Specification.
//!
//! All path construction for ostt's data, config, and log directories goes here.
//! Callers should not build these paths inline.

use std::path::PathBuf;

/// Returns `~/.local/share/ostt` (XDG_DATA_HOME/ostt if set).
///
/// This is where model files, recordings, history, and the daemon socket live.
pub(crate) fn data_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("ostt");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".local")
        .join("share")
        .join("ostt")
}

/// Returns `~/.config/ostt` (XDG_CONFIG_HOME/ostt if set).
pub(crate) fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("ostt");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config")
        .join("ostt")
}

/// Returns `~/.config/ostt/ostt.toml` and ensures the parent directory exists.
///
/// # Errors
/// - If the home directory cannot be determined
/// - If the config directory cannot be created
pub(crate) fn config_path() -> Result<PathBuf, std::io::Error> {
    let path = config_dir().join("ostt.toml");
    std::fs::create_dir_all(path.parent().unwrap())?;
    Ok(path)
}

/// Returns `~/.local/state/ostt` (XDG_STATE_HOME/ostt if set) and ensures it exists.
///
/// This is where log files are written.
///
/// # Errors
/// - If the home directory cannot be determined
/// - If the directory cannot be created
pub(crate) fn log_dir() -> Result<PathBuf, anyhow::Error> {
    let dir = if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(xdg).join("ostt")
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".local")
            .join("state")
            .join("ostt")
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
