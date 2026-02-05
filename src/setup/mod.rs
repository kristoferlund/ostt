//! Setup module for initial application configuration.
//!
//! Handles first-run setup by creating necessary config files and scripts
//! based on the detected environment.

pub mod version;

use anyhow::anyhow;
use std::path::Path;

/// Embedded default configuration template.
const DEFAULT_CONFIG: &str = include_str!("../../environments/ostt.toml");

/// Current application version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Embedded Alacritty configuration for Hyprland floating window.
const ALACRITTY_FLOAT_CONFIG: &str =
    include_str!("../../environments/hyprland/alacritty-float.toml");

/// Embedded shell script for Hyprland integration.
const OSTT_FLOAT_SCRIPT: &str = include_str!("../../environments/hyprland/ostt-float.sh");

/// Runs the setup process if the main config file is missing.
///
/// Creates the config directory and writes default files.
/// On Hyprland + Wayland, also sets up integration files.
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

    // Check for Hyprland environment
    if is_hyprland() {
        setup_hyprland(&config_dir)?;
    }

    Ok(())
}

/// Sets up Hyprland-specific files.
///
/// # Errors
/// Returns an error if file operations fail.
fn setup_hyprland(config_dir: &Path) -> anyhow::Result<()> {
    // Write Alacritty config
    let alacritty_path = config_dir.join("alacritty-float.toml");
    std::fs::write(&alacritty_path, ALACRITTY_FLOAT_CONFIG)?;

    // Create bin directory and write script
    let bin_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".local")
        .join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    let script_path = bin_dir.join("ostt-float");
    std::fs::write(&script_path, OSTT_FLOAT_SCRIPT)?;

    // Make script executable on Unix systems
    #[cfg(unix)]
    make_executable(&script_path)?;

    Ok(())
}

/// Checks if running in a Hyprland + Wayland environment.
fn is_hyprland() -> bool {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() && std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Makes a file executable on Unix systems.
///
/// # Errors
/// Returns an error if permissions cannot be modified.
#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

