//! Application directory helpers following XDG Base Directory Specification.
//!
//! All path construction for ostt's data, config, log, and runtime directories goes here.
//! Callers should not build these paths inline.

use std::path::{Path, PathBuf};

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

/// Returns `~/.local/share/ostt` (XDG_DATA_HOME/ostt if set).
///
/// This is where model files, recordings, history, and the daemon socket live.
pub(crate) fn data_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("ostt");
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".local")
        .join("share")
        .join("ostt")
}

/// Creates `dir` so that only this user can enter it, or makes an existing
/// one so. ostt keeps dictations, recordings, credentials and logs in such
/// directories: another local account must not read them, whatever the
/// umask, even through files whose own mode would let it.
///
/// # Errors
/// - If the directory cannot be created or its mode changed
/// - If it, or a symlink to it, belongs to another user, who could change it
pub(crate) fn private_dir(dir: &Path) -> std::io::Result<()> {
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        match std::fs::DirBuilder::new().mode(0o700).create(dir) {
            Err(err) if err.kind() != std::io::ErrorKind::AlreadyExists => return Err(err),
            _ => {}
        }
        // SAFETY: geteuid has no preconditions and cannot fail.
        let euid = unsafe { libc::geteuid() };
        // A symlink, such as one from a dotfiles repository, must be this
        // user's as well as the directory it leads to: in the temporary
        // folder another user could otherwise point it anywhere.
        let link = std::fs::symlink_metadata(dir)?;
        let metadata = std::fs::metadata(dir)?;
        if link.uid() != euid || metadata.uid() != euid || !metadata.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("{} is not a directory of your own", dir.display()),
            ));
        }
        if metadata.mode() & 0o077 != 0 {
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// Returns the data directory and ensures it exists, private to this user.
pub(crate) fn private_data_dir() -> std::io::Result<PathBuf> {
    let dir = data_dir();
    private_dir(&dir)?;
    Ok(dir)
}

/// Returns the recordings directory and ensures it exists, inside the
/// private data directory.
pub(crate) fn recordings_dir() -> Result<PathBuf, anyhow::Error> {
    let dir = private_data_dir()?.join("recordings");
    private_dir(&dir)?;
    Ok(dir)
}

/// Returns `~/.config/ostt` (XDG_CONFIG_HOME/ostt if set).
pub(crate) fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("ostt");
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config")
        .join("ostt")
}

/// Returns `~/.config/ostt/ostt.toml` and ensures the parent directory
/// exists, private to this user: the config can hold API keys and keywords.
///
/// # Errors
/// - If the home directory cannot be determined
/// - If the config directory cannot be created or made private
pub(crate) fn config_path() -> Result<PathBuf, std::io::Error> {
    let dir = config_dir();
    private_dir(&dir)?;
    Ok(dir.join("ostt.toml"))
}

/// Returns `~/.local/state/ostt` (XDG_STATE_HOME/ostt if set) and ensures it
/// exists, private to this user.
///
/// This is where log files are written. At debug level they hold transcripts.
///
/// # Errors
/// - If the home directory cannot be determined
/// - If the directory cannot be created or made private
pub(crate) fn log_dir() -> Result<PathBuf, anyhow::Error> {
    let dir = if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(xdg).join("ostt")
    } else {
        home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".local")
            .join("state")
            .join("ostt")
    };
    private_dir(&dir)?;
    Ok(dir)
}

/// Returns the per-user runtime directory for transient files.
///
/// On Linux this follows XDG_RUNTIME_DIR. On platforms without XDG runtime
/// directories, it falls back to the system temp directory with a per-user
/// name; [`private_dir`] refuses it if another user created it first.
pub(crate) fn runtime_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(xdg).join("ostt");
    }

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| {
            home_dir()
                .and_then(|home| {
                    home.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| "unknown".to_string())
        });

    std::env::temp_dir().join(format!("ostt-{user}"))
}

/// Returns the active recorder PID file path.
pub(crate) fn recording_pid_path() -> PathBuf {
    runtime_dir().join("recording.pid")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn private_dirs_are_created_and_tightened_for_their_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("share").join("ostt");
        private_dir(&dir).unwrap();
        assert_eq!(mode(&dir), 0o700);

        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        private_dir(&dir).unwrap();
        assert_eq!(mode(&dir), 0o700);
    }
}
