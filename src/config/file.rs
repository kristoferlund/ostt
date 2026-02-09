//! Configuration file management for ostt.
//!
//! This module handles loading and saving application configuration from TOML files.
//! Configuration is stored in the user's config directory.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Visualization type for recording display.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VisualizationType {
    /// Time-domain waveform showing amplitude over time
    Waveform,
    /// Frequency spectrum showing energy distribution across frequencies
    #[default]
    Spectrum,
}

impl std::fmt::Display for VisualizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Waveform => write!(f, "waveform"),
            Self::Spectrum => write!(f, "spectrum"),
        }
    }
}

/// Audio recording and processing configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Audio device to use. Options:
    /// - "default" for system default device
    /// - numeric index (0, 1, 2, etc.) from `ostt list-devices`
    /// - device name from `ostt list-devices`
    pub device: String,
    /// Recording sample rate in Hz (16000 recommended for speech recognition)
    pub sample_rate: u32,
    /// Peak volume threshold for visual indicator (0-100, percentage of reference level)
    #[serde(default = "default_peak_volume_threshold")]
    pub peak_volume_threshold: u8,
    /// Reference level in dBFS for 100% meter display (typical: -20 to -6 dBFS)
    #[serde(default = "default_reference_level_db")]
    pub reference_level_db: i8,
    /// Output audio format string: "codec [ffmpeg_options]" (e.g., "mp3 -ab 16k -ar 12000")
    #[serde(default = "default_output_format")]
    pub output_format: String,
    /// Visualization type: "spectrum" (frequency-based) or "waveform" (time-based amplitude)
    #[serde(default)]
    pub visualization: VisualizationType,
}

fn default_output_format() -> String {
    "mp3 -ab 16k -ar 12000".to_string()
}

fn default_peak_volume_threshold() -> u8 {
    90
}

fn default_reference_level_db() -> i8 {
    -20
}

/// Deepgram API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepgramConfig {
    /// Include filler words in transcript (uh, um, etc.)
    #[serde(default)]
    pub filler_words: bool,
    /// Convert spoken measurements to abbreviations
    #[serde(default)]
    pub measurements: bool,
    /// Convert numbers from written to numerical format
    #[serde(default)]
    pub numerals: bool,
    /// Split audio into paragraphs for readability
    #[serde(default)]
    pub paragraphs: bool,
    /// Apply profanity filtering
    #[serde(default)]
    pub profanity_filter: bool,
    /// Add punctuation and capitalization
    #[serde(default)]
    pub punctuate: bool,
    /// Apply smart formatting to transcript
    #[serde(default)]
    pub smart_format: bool,
    /// Segment speech into meaningful semantic units
    #[serde(default)]
    pub utterances: bool,
    /// Seconds to wait before detecting pause between words
    #[serde(default = "default_utt_split")]
    pub utt_split: f64,
    /// Enable automatic language detection
    #[serde(default = "default_true")]
    pub detect_language: bool,
    /// Opt out from Deepgram Model Improvement Program
    #[serde(default)]
    pub mip_opt_out: bool,
}

fn default_true() -> bool {
    true
}

fn default_utt_split() -> f64 {
    0.8
}

impl Default for DeepgramConfig {
    fn default() -> Self {
        Self {
            filler_words: false,
            measurements: false,
            numerals: false,
            paragraphs: false,
            profanity_filter: false,
            punctuate: false,
            smart_format: false,
            utterances: false,
            utt_split: default_utt_split(),
            detect_language: true,
            mip_opt_out: false,
        }
    }
}

/// OpenAI API configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAiConfig {
    // Currently no additional parameters beyond what's in API
    // Add here as OpenAI features become configurable
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderConfig {
    /// Deepgram provider configuration
    #[serde(rename = "deepgram")]
    Deepgram(DeepgramConfig),
    /// OpenAI provider configuration
    #[serde(rename = "openai")]
    OpenAi(OpenAiConfig),
}

/// All provider configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub deepgram: DeepgramConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
}

/// Complete application configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct OsttConfig {
    pub audio: AudioConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
}

impl OsttConfig {
    /// Loads configuration from the user's config directory.
    ///
    /// # Errors
    /// - If the config directory cannot be determined
    /// - If the config file cannot be read
    /// - If the TOML is malformed
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;
        let config_content = fs::read_to_string(&config_path)?;
        let config: OsttConfig = toml::from_str(&config_content)?;
        Ok(config)
    }

    /// Saves configuration to the user's config directory.
    ///
    /// # Errors
    /// - If the config directory cannot be determined or created
    /// - If the file cannot be written
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = get_config_path()?;
        let config_content = toml::to_string_pretty(self)?;
        fs::write(&config_path, config_content)?;
        tracing::info!("Configuration saved");
        Ok(())
    }

    /// Returns default configuration values.
    #[allow(dead_code)]
    pub(crate) fn default() -> Self {
        OsttConfig {
            audio: AudioConfig {
                device: "default".to_string(),
                sample_rate: 16000,
                peak_volume_threshold: default_peak_volume_threshold(),
                reference_level_db: default_reference_level_db(),
                output_format: default_output_format(),
                visualization: VisualizationType::default(),
            },
            providers: ProvidersConfig::default(),
        }
    }
}

/// Retrieves the path to the config file.
///
/// Assumes the config file exists (created by setup if needed).
///
/// # Errors
/// - If the config directory cannot be determined
/// - If the config directory cannot be created
fn get_config_path() -> Result<PathBuf, std::io::Error> {
    let config_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;
    let config_path = config_dir.join(".config").join("ostt").join("ostt.toml");

    std::fs::create_dir_all(config_path.parent().unwrap())?;

    Ok(config_path)
}

/// Saves the configuration to the config file.
///
/// # Errors
/// - If the config directory cannot be determined or created
/// - If the config file cannot be written
pub fn save_config(config: &OsttConfig) -> anyhow::Result<()> {
    config.save()
}
