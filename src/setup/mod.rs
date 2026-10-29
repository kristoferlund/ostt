//! Setup module for initial application configuration.
//!
//! Handles first-run setup by creating necessary config files.

/// Embedded default configuration template.
const DEFAULT_CONFIG: &str = include_str!("default-config.toml");

/// Runs the setup process if the main config file is missing.
///
/// Creates the config directory and writes default files.
///
/// # Errors
/// Returns an error if any file operations fail.
pub fn run_setup() -> anyhow::Result<()> {
    // Create config directory
    let config_dir = crate::app_dirs::config_dir();
    crate::app_dirs::private_dir(&config_dir)?;

    // The config can come to hold API keys and keywords.
    crate::app_dirs::write_private(&config_dir.join("ostt.toml"), DEFAULT_CONFIG.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OsttConfig;

    #[test]
    fn embedded_default_config_is_valid_toml() {
        toml::from_str::<OsttConfig>(DEFAULT_CONFIG).unwrap();
    }
}
