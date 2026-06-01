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

pub fn handle_keyword_list(json: bool) -> Result<()> {
    let manager = KeywordsManager::new(&crate::app_dirs::config_dir())?;
    let keywords = manager.load_keywords()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&keywords)?);
        return Ok(());
    }

    for keyword in keywords {
        println!("{keyword}");
    }

    Ok(())
}

pub fn handle_keyword_add(keywords: Vec<String>) -> Result<()> {
    let mut manager = KeywordsManager::new(&crate::app_dirs::config_dir())?;
    for keyword in keywords {
        manager.add_keyword(keyword)?;
    }
    Ok(())
}

pub fn handle_keyword_remove(keywords_to_remove: Vec<String>) -> Result<()> {
    let manager = KeywordsManager::new(&crate::app_dirs::config_dir())?;
    let mut keywords = manager.load_keywords()?;

    for keyword in &keywords_to_remove {
        let Some(index) = keywords.iter().position(|candidate| candidate == keyword) else {
            anyhow::bail!("Keyword not found: {keyword}");
        };
        keywords.remove(index);
    }

    manager.save_keywords(&keywords)?;
    Ok(())
}
