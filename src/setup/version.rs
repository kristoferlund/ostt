//! Version comparison and migration logic.
//!
//! Handles checking if setup is needed by comparing embedded version with config file version.

use anyhow::anyhow;
use regex::Regex;
use std::cmp::Ordering;
use std::fmt;
use std::path::Path;

use crate::transcription::TranscriptionModel;

/// Current application version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Represents a semantic version (major.minor.patch)
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SemanticVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemanticVersion {
    /// Parse a version string like "0.0.5" into a SemanticVersion
    fn parse(version_str: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = version_str.trim().split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!(
                "Invalid version format: '{}'. Expected 'major.minor.patch'",
                version_str
            ));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid major version: '{}'", parts[0]))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid minor version: '{}'", parts[1]))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid patch version: '{}'", parts[2]))?;

        Ok(SemanticVersion {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Reads the config version from the first line of the config file using regex.
///
/// Expects the first line to match: `config_version = "X.Y.Z"`
/// The line must start with optional whitespace followed by `config_version` (not a comment).
///
/// # Errors
/// Returns an error if the file can't be read or version parsing fails.
fn read_config_version_from_file(config_path: &Path) -> anyhow::Result<Option<String>> {
    if !config_path.exists() {
        return Ok(None);
    }

    // Read only the first line
    let first_line = std::fs::read_to_string(config_path).and_then(|content| {
        content
            .lines()
            .next()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "config file is empty")
            })
            .map(|s| s.to_string())
    })?;

    // Parse version with regex: ^config_version = "X.Y.Z"
    // Must start with optional whitespace, then 'config_version', not a comment
    let regex = Regex::new(r#"^\s*config_version\s*=\s*"([^"]+)""#)?;
    if let Some(caps) = regex.captures(&first_line) {
        return Ok(Some(caps[1].to_string()));
    }

    Ok(None)
}

/// Determines if setup/version maintenance is needed.
///
/// Maintenance is needed if:
/// 1. Config file doesn't exist, OR
/// 2. Config file exists but has no version (legacy config), OR
/// 3. Config file version is older than current version
///
/// Returns the version that the config file was at. Callers decide whether to
/// create the default config, run migrations, or only update `config_version`.
pub fn check_setup_needed(config_path: &Path) -> anyhow::Result<Option<String>> {
    if !config_path.exists() {
        // Config doesn't exist — setup is needed (fresh install)
        return Ok(Some("none (fresh install)".to_string()));
    }

    let config_version_opt = read_config_version_from_file(config_path)?;
    match config_version_opt {
        Some(config_version) => {
            let config_parsed = SemanticVersion::parse(&config_version)?;
            let current_parsed = SemanticVersion::parse(CURRENT_VERSION)?;

            match config_parsed.cmp(&current_parsed) {
                Ordering::Less => {
                    // Config is older, setup needed
                    Ok(Some(config_version))
                }
                Ordering::Equal => {
                    // Versions match, no setup needed
                    Ok(None)
                }
                Ordering::Greater => {
                    // Config is newer than binary (shouldn't happen in practice)
                    // Log a warning but continue - don't block startup
                    tracing::warn!(
                        "Config version {} is newer than app version {}",
                        config_version,
                        CURRENT_VERSION
                    );
                    Ok(None)
                }
            }
        }
        None => {
            // Config exists but has no version (legacy config)
            Ok(Some("unknown (legacy config)".to_string()))
        }
    }
}

/// Adds or updates the config_version line as the first line of the config file.
///
/// This preserves all existing content by reading the full file, removing any
/// existing config_version line, and prepending the new version line.
pub fn update_config_version(config_path: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config_path)?;

    // Remove existing config_version line if present
    let lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("config_version")
        })
        .collect();

    // Create new content with version as first line
    let version_line = format!(r#"config_version = "{}""#, CURRENT_VERSION);
    let new_content = if lines.is_empty() {
        version_line
    } else {
        format!("{}\n{}", version_line, lines.join("\n"))
    };

    std::fs::write(config_path, new_content)?;
    Ok(())
}

pub fn run_config_migrations(config_path: &Path) -> anyhow::Result<()> {
    if !config_path.exists() {
        return Ok(());
    }

    migrate_selected_model_to_transcription_config(config_path)
}

fn migrate_selected_model_to_transcription_config(config_path: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config_path)?;
    let mut config: toml::Value = toml::from_str(&content)?;

    let has_selection = config
        .get("transcription")
        .and_then(|section| section.as_table())
        .is_some_and(|section| section.get("provider").is_some() && section.get("model").is_some());
    if has_selection {
        return Ok(());
    }

    let Some(selected) = read_legacy_selected_model()? else {
        return Ok(());
    };

    let table = config
        .as_table_mut()
        .ok_or_else(|| anyhow!("config root must be a TOML table"))?;
    let transcription = table
        .entry("transcription")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let transcription = transcription
        .as_table_mut()
        .ok_or_else(|| anyhow!("[transcription] must be a TOML table"))?;
    transcription.insert("provider".to_string(), toml::Value::String(selected.0));
    transcription.insert("model".to_string(), toml::Value::String(selected.1));

    std::fs::write(config_path, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn read_legacy_selected_model() -> anyhow::Result<Option<(String, String)>> {
    let Some(home) = dirs::home_dir() else {
        return Ok(None);
    };
    let model_file = home.join(".local").join("share").join("ostt").join("model");
    if !model_file.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(model_file)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Ok(selected) = serde_json::from_str::<crate::config::SelectedModel>(trimmed) {
        return Ok(Some((selected.provider_id, selected.model_id)));
    }

    let provider_id = TranscriptionModel::from_id(trimmed)
        .map(|model| model.provider().id().to_string())
        .unwrap_or_else(|| "local".to_string());
    Ok(Some((provider_id, trimmed.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::local_models::TEST_ENV_LOCK;

    #[test]
    fn test_semantic_version_parse() {
        let v = SemanticVersion::parse("0.0.5").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 5);
    }

    #[test]
    fn test_semantic_version_comparison() {
        let v1 = SemanticVersion::parse("0.0.4").unwrap();
        let v2 = SemanticVersion::parse("0.0.5").unwrap();
        let v3 = SemanticVersion::parse("0.1.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert_eq!(v1, v1.clone());
    }

    #[test]
    fn test_invalid_version_format() {
        assert!(SemanticVersion::parse("0.0").is_err());
        assert!(SemanticVersion::parse("0.0.5.1").is_err());
        assert!(SemanticVersion::parse("invalid").is_err());
    }

    #[test]
    fn update_config_version_preserves_existing_config() {
        let path = std::env::temp_dir().join(format!(
            "ostt-config-version-test-{}.toml",
            std::process::id()
        ));
        let original = r#"config_version = "0.0.9"
[process.actions.caveman]
name = "Caveman speak"
type = "ai"
"#;

        std::fs::write(&path, original).unwrap();
        update_config_version(&path).unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert!(updated.starts_with(&format!(r#"config_version = "{}""#, CURRENT_VERSION)));
        assert!(updated.contains("[process.actions.caveman]"));
        assert!(updated.contains(r#"name = "Caveman speak""#));
    }

    #[test]
    fn run_config_migrations_moves_legacy_model_file_to_transcription_section() {
        let _guard = TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_home = std::env::var_os("HOME");
        let dir =
            std::env::temp_dir().join(format!("ostt-config-migration-test-{}", std::process::id()));
        let config_path = dir.join(".config").join("ostt").join("ostt.toml");
        let model_path = dir.join(".local").join("share").join("ostt").join("model");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(model_path.parent().unwrap()).unwrap();
        std::fs::write(
            &config_path,
            r#"config_version = "0.0.10"

[audio]
device = "default"
sample_rate = 16000
"#,
        )
        .unwrap();
        std::fs::write(&model_path, "nova-3").unwrap();
        std::env::set_var("HOME", &dir);

        run_config_migrations(&config_path).unwrap();

        let migrated = std::fs::read_to_string(&config_path).unwrap();
        assert!(migrated.contains("[transcription]"));
        assert!(migrated.contains(r#"provider = "deepgram""#));
        assert!(migrated.contains(r#"model = "nova-3""#));

        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
