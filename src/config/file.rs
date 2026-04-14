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
    /// Restrict language detection to specific languages (e.g., ["en", "es"])
    /// When empty, all languages can be detected
    #[serde(default)]
    pub detect_language_codes: Vec<String>,
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
            detect_language_codes: Vec::new(),
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

/// Options for AssemblyAI automatic language detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageDetectionOptions {
    /// List of languages expected in the audio file.
    /// Defaults to ["all"] when unspecified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_languages: Option<Vec<String>>,
    /// Fallback language if detected language is not in expected_languages.
    /// Use "auto" to let the model choose from expected_languages with highest confidence.
    /// Defaults to "auto".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_language: Option<String>,
}

/// AssemblyAI API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyAIConfig {
    /// Apply text formatting (punctuation, casing, numerals)
    #[serde(default = "default_true")]
    pub format_text: bool,
    /// Include disfluencies (uh, um) in transcript
    #[serde(default)]
    pub disfluencies: bool,
    /// Filter profanity from transcript
    #[serde(default)]
    pub filter_profanity: bool,
    /// Enable automatic language detection
    #[serde(default = "default_true")]
    pub language_detection: bool,
    /// Options for automatic language detection
    #[serde(default)]
    pub language_detection_options: LanguageDetectionOptions,
    /// Enable automatic punctuation
    #[serde(default = "default_true")]
    pub punctuate: bool,
}

impl Default for AssemblyAIConfig {
    fn default() -> Self {
        Self {
            format_text: true,
            disfluencies: false,
            filter_profanity: false,
            language_detection: true,
            language_detection_options: LanguageDetectionOptions::default(),
            punctuate: true,
        }
    }
}

/// All provider configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub deepgram: DeepgramConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
    #[serde(default)]
    pub assemblyai: AssemblyAIConfig,
}

/// The role of an input message sent to an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputRole {
    System,
    User,
}

/// A special source that provides dynamic content at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputSource {
    /// The transcribed text from the recording
    Transcription,
    /// The user's configured keyword list
    Keywords,
}

/// The content source for an action input message.
///
/// Uses `#[serde(untagged)]` so that TOML input entries are disambiguated by
/// field name alone — the user writes `source = "transcription"`, `file = "~/prompt.txt"`,
/// or `content = "literal text"` and serde matches the correct variant.
///
/// Variant order defines precedence: if a user specifies multiple fields on the
/// same input (e.g. both `source` and `content`), serde picks the first matching
/// variant silently. Precedence: `source` > `file` > `content`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputContent {
    /// Dynamic source: transcription or keywords
    Source { source: InputSource },
    /// Path to a file whose contents become the message content
    File { file: String },
    /// Literal text content
    Literal { content: String },
}

/// A single input entry for an AI action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInput {
    /// Message role: "system" or "user" — enforced by `InputRole` enum.
    pub role: InputRole,
    /// The content source — exactly one of: literal content, a special source, or a file path.
    /// Enforced at the type level via `InputContent` enum.
    #[serde(flatten)]
    pub input_content: InputContent,
}

/// Supported AI CLI tools for executing AI actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiTool {
    OpenCode,
    ClaudeCode,
    GeminiCli,
    CodexCli,
}

impl AiTool {
    /// Returns the standard binary name for this AI tool.
    pub fn default_binary(&self) -> &'static str {
        match self {
            AiTool::OpenCode => "opencode",
            AiTool::ClaudeCode => "claude",
            AiTool::GeminiCli => "gemini",
            AiTool::CodexCli => "codex",
        }
    }
}

/// Type-specific fields for a processing action.
///
/// Uses `#[serde(tag = "type")]` so the TOML `type` field drives which variant
/// serde expects. Required fields for each variant are enforced at deserialization
/// time — no runtime validation needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ActionDetails {
    /// A bash command that receives transcription via stdin
    Bash {
        /// Shell command to execute
        command: String,
    },
    /// An AI chat completion action
    Ai {
        /// Which CLI tool to invoke
        tool: AiTool,
        /// Provider/model string (e.g. "openai/gpt-4o")
        model: String,
        /// Input messages for the LLM
        inputs: Vec<ActionInput>,
        /// Override the binary path (e.g., "/usr/local/bin/claude" instead of "claude").
        /// Defaults to the standard binary name for the selected tool.
        #[serde(default)]
        tool_binary: Option<String>,
        /// Extra CLI arguments appended after the required ones.
        /// Allows pro users to pass additional flags without modifying OSTT.
        #[serde(default)]
        tool_args: Option<Vec<String>>,
    },
}

/// A single processing action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAction {
    /// Unique identifier for this action (used in CLI: `-p clean`)
    pub id: String,
    /// Human-readable display name (shown in action picker)
    pub name: String,
    /// Action type and its type-specific configuration.
    /// The TOML `type` field ("bash" or "ai") determines which fields are required.
    #[serde(flatten)]
    pub details: ActionDetails,
}

/// Top-level process configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessConfig {
    /// List of configured processing actions
    #[serde(default)]
    pub actions: Vec<ProcessAction>,
}

impl ProcessAction {
    /// Validates this action's configuration.
    ///
    /// Returns an error if an AI action has an empty `inputs` list.
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let ActionDetails::Ai { inputs, .. } = &self.details {
            if inputs.is_empty() {
                return Err(format!("AI action '{}' must have at least one input", self.id).into());
            }
        }
        Ok(())
    }
}

impl ProcessConfig {
    /// Finds an action by its `id` field.
    pub fn get_action(&self, id: &str) -> Option<&ProcessAction> {
        self.actions.iter().find(|a| a.id == id)
    }
}

/// Complete application configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct OsttConfig {
    pub audio: AudioConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub process: ProcessConfig,
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
        for action in &config.process.actions {
            action.validate()?;
        }
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
            process: ProcessConfig::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper: deserialize just a ProcessConfig from a TOML string ──

    fn parse_process_config(toml_str: &str) -> Result<ProcessConfig, toml::de::Error> {
        toml::from_str(toml_str)
    }

    fn parse_action(toml_str: &str) -> Result<ProcessAction, toml::de::Error> {
        toml::from_str(toml_str)
    }

    fn parse_input(toml_str: &str) -> Result<ActionInput, toml::de::Error> {
        toml::from_str(toml_str)
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.1.14 — Valid configurations
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn valid_bash_action() {
        let toml_str = r#"
            id = "copy"
            name = "Copy to clipboard"
            type = "bash"
            command = "xclip -selection clipboard"
        "#;
        let action = parse_action(toml_str).unwrap();
        assert_eq!(action.id, "copy");
        assert_eq!(action.name, "Copy to clipboard");
        assert!(
            matches!(action.details, ActionDetails::Bash { ref command } if command == "xclip -selection clipboard")
        );
    }

    #[test]
    fn valid_ai_action() {
        let toml_str = r#"
            id = "clean"
            name = "Clean transcript"
            type = "ai"
            tool = "open-code"
            model = "openai/gpt-4o"

            [[inputs]]
            role = "system"
            content = "You are a helpful assistant."

            [[inputs]]
            role = "user"
            source = "transcription"
        "#;
        let action = parse_action(toml_str).unwrap();
        assert_eq!(action.id, "clean");
        assert_eq!(action.name, "Clean transcript");
        match &action.details {
            ActionDetails::Ai {
                tool,
                model,
                inputs,
                ..
            } => {
                assert!(matches!(tool, AiTool::OpenCode));
                assert_eq!(model, "openai/gpt-4o");
                assert_eq!(inputs.len(), 2);
            }
            _ => panic!("expected Ai variant"),
        }
    }

    #[test]
    fn valid_mixed_actions() {
        let toml_str = r#"
            [[actions]]
            id = "copy"
            name = "Copy"
            type = "bash"
            command = "xclip"

            [[actions]]
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "claude-code"
            model = "openai/gpt-4o"

            [[actions.inputs]]
            role = "user"
            source = "transcription"
        "#;
        let config = parse_process_config(toml_str).unwrap();
        assert_eq!(config.actions.len(), 2);
        assert!(matches!(
            config.actions[0].details,
            ActionDetails::Bash { .. }
        ));
        assert!(matches!(
            config.actions[1].details,
            ActionDetails::Ai { .. }
        ));
    }

    #[test]
    fn missing_process_section_defaults_to_empty() {
        let toml_str = r#"
            [audio]
            device = "default"
            sample_rate = 16000
        "#;
        let config: OsttConfig = toml::from_str(toml_str).unwrap();
        assert!(config.process.actions.is_empty());
    }

    #[test]
    fn input_role_system_with_content() {
        let toml_str = r#"
            role = "system"
            content = "You are a helpful assistant."
        "#;
        let input = parse_input(toml_str).unwrap();
        assert!(matches!(input.role, InputRole::System));
        assert!(
            matches!(input.input_content, InputContent::Literal { ref content } if content == "You are a helpful assistant.")
        );
    }

    #[test]
    fn input_role_user_with_content() {
        let toml_str = r#"
            role = "user"
            content = "Hello world"
        "#;
        let input = parse_input(toml_str).unwrap();
        assert!(matches!(input.role, InputRole::User));
        assert!(
            matches!(input.input_content, InputContent::Literal { ref content } if content == "Hello world")
        );
    }

    #[test]
    fn input_source_transcription() {
        let toml_str = r#"
            role = "user"
            source = "transcription"
        "#;
        let input = parse_input(toml_str).unwrap();
        assert!(matches!(
            input.input_content,
            InputContent::Source {
                source: InputSource::Transcription
            }
        ));
    }

    #[test]
    fn input_source_keywords() {
        let toml_str = r#"
            role = "user"
            source = "keywords"
        "#;
        let input = parse_input(toml_str).unwrap();
        assert!(matches!(
            input.input_content,
            InputContent::Source {
                source: InputSource::Keywords
            }
        ));
    }

    #[test]
    fn input_file() {
        let toml_str = r#"
            role = "system"
            file = "~/prompts/clean.txt"
        "#;
        let input = parse_input(toml_str).unwrap();
        assert!(matches!(
            input.input_content,
            InputContent::File { ref file } if file == "~/prompts/clean.txt"
        ));
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.1.15 — Invalid ProcessAction configurations
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn invalid_action_missing_type() {
        let toml_str = r#"
            id = "copy"
            name = "Copy"
            command = "xclip"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_action_unknown_type() {
        let toml_str = r#"
            id = "run"
            name = "Run"
            type = "python"
            command = "print('hi')"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_action_missing_id() {
        let toml_str = r#"
            name = "Copy"
            type = "bash"
            command = "xclip"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_action_missing_name() {
        let toml_str = r#"
            id = "copy"
            type = "bash"
            command = "xclip"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_bash_missing_command() {
        let toml_str = r#"
            id = "copy"
            name = "Copy"
            type = "bash"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_ai_missing_model() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "open-code"

            [[inputs]]
            role = "user"
            source = "transcription"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_ai_missing_inputs() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "open-code"
            model = "openai/gpt-4o"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    #[test]
    fn invalid_ai_missing_model_and_inputs() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "open-code"
        "#;
        assert!(parse_action(toml_str).is_err());
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.1.16 — Invalid ActionInput configurations
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn invalid_input_missing_role() {
        let toml_str = r#"
            content = "Hello"
        "#;
        assert!(parse_input(toml_str).is_err());
    }

    #[test]
    fn invalid_input_unknown_role() {
        let toml_str = r#"
            role = "admin"
            content = "Hello"
        "#;
        assert!(parse_input(toml_str).is_err());
    }

    #[test]
    fn invalid_input_no_content_field() {
        let toml_str = r#"
            role = "user"
        "#;
        assert!(parse_input(toml_str).is_err());
    }

    #[test]
    fn invalid_input_unknown_source() {
        let toml_str = r#"
            role = "user"
            source = "bogus"
        "#;
        assert!(parse_input(toml_str).is_err());
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.1.17 — Edge cases
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn empty_inputs_deserializes_but_fails_validate() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "open-code"
            model = "openai/gpt-4o"
            inputs = []
        "#;
        let action = parse_action(toml_str).unwrap();
        assert!(action.validate().is_err());
    }

    #[test]
    fn multiple_content_fields_uses_highest_precedence() {
        // source > file > content — so `source` wins
        let toml_str = r#"
            role = "user"
            source = "transcription"
            content = "ignored"
        "#;
        let input = parse_input(toml_str).unwrap();
        assert!(matches!(
            input.input_content,
            InputContent::Source {
                source: InputSource::Transcription
            }
        ));
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.1.18 — get_action lookup
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn get_action_returns_matching_action() {
        let toml_str = r#"
            [[actions]]
            id = "clean"
            name = "Clean transcript"
            type = "bash"
            command = "sed 's/um//g'"
        "#;
        let config = parse_process_config(toml_str).unwrap();
        let action = config.get_action("clean");
        assert!(action.is_some());
        assert_eq!(action.unwrap().id, "clean");
    }

    #[test]
    fn get_action_returns_none_for_nonexistent() {
        let config = ProcessConfig::default();
        assert!(config.get_action("nonexistent").is_none());
    }
}
