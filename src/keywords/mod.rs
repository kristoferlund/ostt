//! Keyword management for transcription.
//!
//! Provides storage and management of keywords used to improve transcription accuracy.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub mod ui;

pub use ui::KeywordsViewer;

/// Manages the keywords list stored in the config directory.
pub struct KeywordsManager {
    /// Path to the keywords file
    file_path: PathBuf,
}

impl KeywordsManager {
    /// Creates a new keywords manager for the given config directory.
    ///
    /// # Arguments
    /// * `config_dir` - Directory where the keywords file will be stored
    ///
    /// # Errors
    /// - If the config directory cannot be accessed
    pub fn new(config_dir: &Path) -> Result<Self> {
        let file_path = config_dir.join("keywords.txt");
        Ok(Self { file_path })
    }

    /// Loads the list of keywords from the file.
    ///
    /// # Errors
    /// - If the file cannot be read
    pub fn load_keywords(&self) -> Result<Vec<String>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.file_path)?;
        let keywords = content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        Ok(keywords)
    }

    /// Saves the list of keywords to the file.
    ///
    /// # Arguments
    /// * `keywords` - List of keywords to save
    ///
    /// # Errors
    /// - If the file cannot be written
    pub fn save_keywords(&self, keywords: &[String]) -> Result<()> {
        let content = keywords.join("\n");
        fs::write(&self.file_path, content)?;
        Ok(())
    }

    /// Adds a new keyword to the list.
    ///
    /// # Arguments
    /// * `keyword` - The keyword to add
    ///
    /// # Errors
    /// - If loading or saving fails
    pub fn add_keyword(&mut self, keyword: String) -> Result<()> {
        let mut keywords = self.load_keywords()?;
        if !keywords.contains(&keyword) {
            keywords.push(keyword);
            self.save_keywords(&keywords)?;
        }
        Ok(())
    }

    /// Removes a keyword from the list.
    ///
    /// # Arguments
    /// * `index` - Index of the keyword to remove
    ///
    /// # Errors
    /// - If loading or saving fails
    pub fn remove_keyword(&mut self, index: usize) -> Result<()> {
        let mut keywords = self.load_keywords()?;
        if index < keywords.len() {
            keywords.remove(index);
            self.save_keywords(&keywords)?;
        }
        Ok(())
    }
}
