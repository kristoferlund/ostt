//! API credentials storage for ostt.
//!
//! This module handles secure storage of API credentials with restricted file permissions.
//! Credentials are stored in the user's local data directory (~/.local/share/ostt).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Returns the path to the secrets directory (~/.local/share/ostt).
///
/// Creates the directory if it doesn't exist.
///
/// # Errors
/// - If the local data directory cannot be determined
/// - If the secrets directory cannot be created
fn get_secrets_dir() -> anyhow::Result<PathBuf> {
    let secrets_dir = crate::app_dirs::data_dir();
    fs::create_dir_all(&secrets_dir)?;
    Ok(secrets_dir)
}

/// Writes the credentials file readable only by its owner. A new file is
/// created with mode 0600, and an existing one is restricted before the keys
/// are written, so they are never readable by others, whatever the umask.
fn write_private(path: &Path, content: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.set_len(0)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Saves an API key for the specified provider.
///
/// Stores credentials in ~/.local/share/ostt/credentials with restricted permissions (0600).
///
/// # Errors
/// - If the secrets directory cannot be determined or created
/// - If the credentials file cannot be read or written
/// - If the TOML cannot be serialized
pub fn save_api_key(provider_id: &str, api_key: &str) -> anyhow::Result<()> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    let mut credentials: HashMap<String, String> = if credentials_file.exists() {
        let content = fs::read_to_string(&credentials_file)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    credentials.insert(provider_id.to_string(), api_key.to_string());

    let content = toml::to_string(&credentials)?;
    write_private(&credentials_file, &content)?;

    tracing::info!("API key saved for provider: {}", provider_id);
    Ok(())
}

/// Retrieves the API key for the specified provider.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the credentials file cannot be read
/// - If the TOML cannot be parsed
pub fn get_api_key(provider_id: &str) -> anyhow::Result<Option<String>> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    if !credentials_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&credentials_file)?;
    let credentials: HashMap<String, String> = toml::from_str(&content).unwrap_or_default();

    Ok(credentials.get(provider_id).cloned())
}

/// Returns all available provider IDs that have API keys saved.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the credentials file cannot be read
/// - If the TOML cannot be parsed
pub fn get_authorized_providers() -> anyhow::Result<Vec<String>> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    if !credentials_file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&credentials_file)?;
    let credentials: HashMap<String, String> = toml::from_str(&content).unwrap_or_default();

    Ok(credentials.keys().cloned().collect())
}

/// Clears the API key for the specified provider.
///
/// # Errors
/// - If the secrets directory cannot be determined
/// - If the credentials file cannot be read or written
/// - If the TOML cannot be serialized
pub fn clear_api_key(provider_id: &str) -> anyhow::Result<()> {
    let secrets_dir = get_secrets_dir()?;
    let credentials_file = secrets_dir.join("credentials");

    if !credentials_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&credentials_file)?;
    let mut credentials: HashMap<String, String> = toml::from_str(&content).unwrap_or_default();

    if credentials.remove(provider_id).is_some() {
        let content = toml::to_string(&credentials)?;
        write_private(&credentials_file, &content)?;
        tracing::info!("API key cleared for provider: {}", provider_id);
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    /// API keys must never be readable by other local users, not even for a
    /// moment while the file is written, so the file is 0600 before any key
    /// is in it.
    #[test]
    fn new_credentials_file_is_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials");
        write_private(&path, "openai = \"sk-1\"\n").unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(fs::read_to_string(&path).unwrap(), "openai = \"sk-1\"\n");
    }

    /// A credentials file left readable by an older ostt or by hand is
    /// restricted before the keys are written again, and shorter content
    /// leaves nothing of the old behind.
    #[test]
    fn existing_readable_file_is_restricted_and_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials");
        fs::write(&path, "openai = \"sk-long-old-key\"\ngroq = \"gsk\"\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&path, "groq = \"gsk\"\n").unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(fs::read_to_string(&path).unwrap(), "groq = \"gsk\"\n");
    }
}
