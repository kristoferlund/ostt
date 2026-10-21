//! Keyword management for transcription.
//!
//! Provides storage and management of keywords used to improve transcription accuracy.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::TranscriptionSelectionConfig;

pub mod keywords_view;

pub(crate) fn load_keywords() -> Result<Vec<String>> {
    let keywords_manager = KeywordsManager::new(&crate::app_dirs::config_dir())?;
    keywords_manager.load_keywords()
}

/// Manages the keywords list in `[transcription].keywords` of `ostt.toml`.
pub struct KeywordsManager {
    /// Path to `ostt.toml`
    config_path: PathBuf,
}

impl KeywordsManager {
    /// Creates a new keywords manager for the given config directory.
    ///
    /// # Arguments
    /// * `config_dir` - Directory holding `ostt.toml`
    ///
    /// # Errors
    /// - If the config directory cannot be accessed
    pub fn new(config_dir: &Path) -> Result<Self> {
        let config_path = config_dir.join("ostt.toml");
        Ok(Self { config_path })
    }

    /// Loads the list of keywords from the config file.
    ///
    /// # Errors
    /// - If the file cannot be read or is not valid TOML
    pub fn load_keywords(&self) -> Result<Vec<String>> {
        // The same [transcription] parsing as the full config, so a key the
        // config loader would reject is rejected here too.
        #[derive(Deserialize)]
        struct KeywordsOnly {
            #[serde(default)]
            transcription: TranscriptionSelectionConfig,
        }

        let content = match fs::read_to_string(&self.config_path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let config: KeywordsOnly = toml::from_str(&content).with_context(|| {
            format!(
                "failed to read keywords from {}",
                self.config_path.display()
            )
        })?;
        Ok(clean(config.transcription.keywords))
    }

    /// Saves the list of keywords to the config file.
    ///
    /// # Arguments
    /// * `keywords` - List of keywords to save
    ///
    /// # Errors
    /// - If the file cannot be written
    pub fn save_keywords(&self, keywords: &[String]) -> Result<()> {
        crate::config::edit::edit_config_file_at(&self.config_path, |doc| {
            crate::config::edit::set_keywords(doc, keywords)
        })
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

/// Trimmed, without empty entries, in the user's order.
fn clean(keywords: Vec<String>) -> Vec<String> {
    keywords
        .into_iter()
        .map(|keyword| keyword.trim().to_string())
        .filter(|keyword| !keyword.is_empty())
        .collect()
}

/// Moves keywords from `keywords.txt`, used before 0.0.30, into `ostt.toml`
/// and renames the old file to `keywords.txt.migrated`. If the config already
/// has keywords, it wins and the old file is only renamed.
pub(crate) fn migrate_keywords_file(config_dir: &Path) -> Result<()> {
    let legacy_path = config_dir.join("keywords.txt");
    if !legacy_path.exists() {
        return Ok(());
    }
    let legacy = clean(
        fs::read_to_string(&legacy_path)?
            .lines()
            .map(str::to_string)
            .collect(),
    );
    let manager = KeywordsManager::new(config_dir)?;
    if manager.load_keywords()?.is_empty() {
        manager.save_keywords(&legacy)?;
        tracing::info!(
            "Moved {} keyword(s) from keywords.txt into [transcription].keywords in ostt.toml",
            legacy.len()
        );
    } else if !legacy.is_empty() {
        tracing::warn!("Ignoring keywords.txt: ostt.toml already has [transcription].keywords");
    }
    fs::rename(&legacy_path, config_dir.join("keywords.txt.migrated"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_config_dir(test: impl FnOnce(&Path)) {
        // Clock readings can repeat (macOS has microsecond resolution), so a
        // timestamp cannot isolate tests that run in parallel.
        let dir = tempfile::tempdir().unwrap();
        test(dir.path());
    }

    const CONFIG: &str = "[audio]\ndevice = \"default\" # USB mic\n\n[transcription]\nprovider = \"whisper\"\nmodel = \"base\"\n";

    #[test]
    fn keywords_round_trip_through_ostt_toml_without_touching_other_settings() {
        with_config_dir(|dir| {
            fs::write(dir.join("ostt.toml"), CONFIG).unwrap();
            let mut manager = KeywordsManager::new(dir).unwrap();
            manager.add_keyword("OSTT".to_string()).unwrap();
            manager.add_keyword("Rödeby".to_string()).unwrap();
            manager.add_keyword("OSTT".to_string()).unwrap();

            assert_eq!(manager.load_keywords().unwrap(), ["OSTT", "Rödeby"]);
            let content = fs::read_to_string(dir.join("ostt.toml")).unwrap();
            assert!(content.starts_with(CONFIG), "{content}");
            // The full config loader sees the same keywords.
            let config: crate::config::OsttConfig = toml::from_str(&content).unwrap();
            assert_eq!(config.transcription.keywords, ["OSTT", "Rödeby"]);
        });
    }

    /// Upgrading must keep the keywords users saved in keywords.txt.
    #[test]
    fn keywords_txt_moves_into_the_config_once() {
        with_config_dir(|dir| {
            fs::write(dir.join("ostt.toml"), CONFIG).unwrap();
            fs::write(dir.join("keywords.txt"), "OSTT\n\n  Kubernetes \n").unwrap();

            migrate_keywords_file(dir).unwrap();

            let manager = KeywordsManager::new(dir).unwrap();
            assert_eq!(manager.load_keywords().unwrap(), ["OSTT", "Kubernetes"]);
            assert!(!dir.join("keywords.txt").exists());
            assert!(dir.join("keywords.txt.migrated").exists());
            // A second run finds nothing to move.
            migrate_keywords_file(dir).unwrap();
            assert_eq!(manager.load_keywords().unwrap(), ["OSTT", "Kubernetes"]);
        });
    }

    #[test]
    fn keywords_already_in_the_config_win_over_keywords_txt() {
        with_config_dir(|dir| {
            fs::write(
                dir.join("ostt.toml"),
                format!("{CONFIG}keywords = [\"Rödeby\"]\n"),
            )
            .unwrap();
            fs::write(dir.join("keywords.txt"), "OSTT\n").unwrap();

            migrate_keywords_file(dir).unwrap();

            let manager = KeywordsManager::new(dir).unwrap();
            assert_eq!(manager.load_keywords().unwrap(), ["Rödeby"]);
            assert!(dir.join("keywords.txt.migrated").exists());
        });
    }

    /// A config that cannot be parsed must not lose keywords.txt: the move is
    /// retried once the config is fixed.
    #[test]
    fn a_broken_config_leaves_keywords_txt_in_place() {
        with_config_dir(|dir| {
            fs::write(dir.join("ostt.toml"), "[audio\n").unwrap();
            fs::write(dir.join("keywords.txt"), "OSTT\n").unwrap();

            assert!(migrate_keywords_file(dir).is_err());
            assert!(dir.join("keywords.txt").exists());
        });
    }
}
