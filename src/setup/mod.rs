//! Setup module for initial application configuration.
//!
//! Handles first-run setup by creating necessary config files.

pub mod version;

use anyhow::anyhow;

/// Embedded default configuration template.
const DEFAULT_CONFIG: &str = include_str!("../../environments/ostt.toml");

/// Current application version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runs the setup process if the main config file is missing.
///
/// Creates the config directory and writes default files.
///
/// # Errors
/// Returns an error if any file operations fail.
pub fn run_setup() -> anyhow::Result<()> {
    // Create config directory
    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("ostt");
    std::fs::create_dir_all(&config_dir)?;

    // Write main config file with version prefix
    let config_path = config_dir.join("ostt.toml");
    let config_with_version = format!(r#"config_version = "{}""#, CURRENT_VERSION);
    let full_config = format!("{}\n{}", config_with_version, DEFAULT_CONFIG);
    std::fs::write(&config_path, full_config)?;

    Ok(())
}
