//! Configuration file editor command.
//!
//! Opens the ostt configuration file in the user's preferred editor.

use std::process::Command;

/// Opens the ostt configuration file in the user's preferred editor.
///
/// Tries editors in this order:
/// 1. $EDITOR environment variable
/// 2. nano (most user-friendly fallback)
/// 3. vi (ultimate fallback, always available)
///
/// # Errors
/// - If no editor can be found or executed
pub fn handle_config() -> anyhow::Result<()> {
    let config_path = get_config_path()?;

    tracing::info!("Opening config file: {}", config_path.display());

    let editor = find_editor()?;
    tracing::debug!("Using editor: {}", editor);

    let status = Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to open editor '{editor}': {e}. Make sure the editor is installed and accessible."
            )
        })?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "Editor exited with error code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    tracing::info!("Config file edited successfully");
    Ok(())
}

/// Finds the best available editor to use.
///
/// Tries in order: $EDITOR, nano, vi
fn find_editor() -> anyhow::Result<String> {
    // Try $EDITOR environment variable
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.is_empty() {
            return Ok(editor);
        }
    }

    // Try common editors in order of user-friendliness
    for editor in &["nano", "vi"] {
        if is_editor_available(editor) {
            return Ok(editor.to_string());
        }
    }

    Err(anyhow::anyhow!(
        "No editor found. Please set the $EDITOR environment variable."
    ))
}

/// Checks if an editor is available in the system PATH.
fn is_editor_available(editor: &str) -> bool {
    Command::new("which")
        .arg(editor)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Retrieves the path to the ostt configuration file.
///
/// # Errors
/// - If the home directory cannot be determined
fn get_config_path() -> anyhow::Result<std::path::PathBuf> {
    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("ostt");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create config directory: {e}"))?;

    Ok(config_dir.join("ostt.toml"))
}
