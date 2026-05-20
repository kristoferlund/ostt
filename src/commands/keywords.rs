//! Keyword management command handler.
//!
//! Orchestrates the keywords management UI and storage.

use crate::keywords::{KeywordsManager, KeywordsView};
use anyhow::Result;
use dirs;

/// Handles the keywords management command.
///
/// Shows a TUI for viewing, adding, and removing keywords.
pub async fn handle_keywords() -> Result<()> {
    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("ostt");

    let mut manager = KeywordsManager::new(&config_dir)?;

    let mut view = KeywordsView::new(manager.load_keywords()?)?;
    view.run(&mut manager)?;

    Ok(())
}
