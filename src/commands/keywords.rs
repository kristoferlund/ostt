//! Keyword management command handler.
//!
//! Orchestrates the keywords management UI and storage.

use crate::keywords::{KeywordsManager, KeywordsView};
use anyhow::Result;

/// Handles the keywords management command.
///
/// Shows a TUI for viewing, adding, and removing keywords.
pub async fn handle_keywords() -> Result<()> {
    let config_dir = crate::app_dirs::config_dir();

    let mut manager = KeywordsManager::new(&config_dir)?;

    let mut view = KeywordsView::new(manager.load_keywords()?)?;
    view.run(&mut manager)?;

    Ok(())
}
