//! Configuration file management for ostt.
//!
//! This module handles loading and saving application configuration from TOML files.
//! Configuration is stored in the user's config directory.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::transcription::{api, model};

fn current_config_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

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
    /// - numeric index (0, 1, 2, etc.) from `ostt config list-devices`
    /// - device name from `ostt config list-devices`
    pub device: String,
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

const LOCAL_TRANSCRIPTION_OUTPUT_FORMAT: &str = "pcm_s16le -ar 16000";

pub fn is_local_transcription_audio_compatible(audio: &AudioConfig) -> bool {
    audio.output_format == LOCAL_TRANSCRIPTION_OUTPUT_FORMAT
}

fn default_peak_volume_threshold() -> u8 {
    90
}

fn default_reference_level_db() -> i8 {
    -20
}

fn default_true() -> bool {
    true
}

/// Local transcription provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalTranscriptionConfig {
    /// Language hint: "auto" or ISO code (e.g. "sv", "en")
    pub language: String,
    /// Suppress timestamp output
    pub no_timestamps: bool,
    /// Suppress text context from previous segments
    pub no_context: bool,
    /// Sampling temperature (0.0 = greedy/deterministic)
    pub temperature: f32,
    /// Entropy threshold for fallback (2.4 = default)
    pub entropy_thold: f32,
    /// No-speech probability threshold (0.6 = default)
    pub no_speech_thold: f32,
}

impl Default for LocalTranscriptionConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            no_timestamps: true,
            no_context: true,
            temperature: 0.0,
            entropy_thold: 2.4,
            no_speech_thold: 0.6,
        }
    }
}

impl LocalTranscriptionConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        validate_local_values(self.temperature, self.entropy_thold, self.no_speech_thold)?;

        Ok(())
    }
}

fn validate_local_values(
    temperature: f32,
    entropy_thold: f32,
    no_speech_thold: f32,
) -> anyhow::Result<()> {
    if !(0.0..=1.0).contains(&temperature) {
        anyhow::bail!("temperature must be between 0.0 and 1.0");
    }
    if entropy_thold < 0.0 {
        anyhow::bail!("entropy_thold must be >= 0.0");
    }
    if !(0.0..=1.0).contains(&no_speech_thold) {
        anyhow::bail!("no_speech_thold must be between 0.0 and 1.0");
    }
    Ok(())
}

/// All provider configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub local: LocalTranscriptionConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelOptionValue {
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    StringList(Vec<String>),
}

pub type ModelOptionsConfig = IndexMap<String, IndexMap<String, ModelOptionValue>>;

/// Popup window configuration for the `launch` subcommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupConfig {
    /// Terminal emulator to use. If not set, auto-detects from available terminals.
    /// Preferred: "ghostty", "kitty", "alacritty"
    /// Fallbacks: "foot", "konsole", "gnome-terminal", "xfce4-terminal"
    #[serde(default)]
    pub terminal: Option<String>,
    /// Window x position in pixels
    #[serde(default = "default_popup_x")]
    pub x: u32,
    /// Window y position in pixels
    #[serde(default = "default_popup_y")]
    pub y: u32,
    /// Window width in terminal columns
    #[serde(default = "default_popup_width")]
    pub width: u32,
    /// Window height in terminal rows
    #[serde(default = "default_popup_height")]
    pub height: u32,
    /// Font size
    #[serde(default = "default_popup_font_size")]
    pub font_size: u32,
    /// Hide window decorations (titlebar, borders)
    #[serde(default = "default_true")]
    pub borderless: bool,
}

fn default_popup_x() -> u32 {
    630
}
fn default_popup_y() -> u32 {
    790
}
fn default_popup_width() -> u32 {
    90
}
fn default_popup_height() -> u32 {
    15
}
fn default_popup_font_size() -> u32 {
    6
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            terminal: None,
            x: default_popup_x(),
            y: default_popup_y(),
            width: default_popup_width(),
            height: default_popup_height(),
            font_size: default_popup_font_size(),
            borderless: true,
        }
    }
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
    #[serde(rename = "opencode")]
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

    /// Returns the tool-specific required CLI arguments given a model and system prompt.
    ///
    /// - OpenCode: `["run", "--model", model]`
    /// - Claude Code: `["-p", "--system-prompt", system, "--model", model]`
    /// - Gemini CLI: `["-p", system, "-m", model]`
    /// - Codex CLI: `["exec", system, "--model", model]`
    pub fn build_required_args(&self, model: &str) -> Vec<String> {
        match self {
            AiTool::OpenCode => {
                vec![
                    "--pure".to_string(),
                    "run".to_string(),
                    "--model".to_string(),
                    model.to_string(),
                ]
            }
            AiTool::ClaudeCode => {
                vec![
                    "-p".to_string(),
                    "--model".to_string(),
                    model.to_string(),
                    "--no-session-persistence".to_string(),
                    "--mcp-config".to_string(),
                    r#"{"mcpServers":{}}"#.to_string(),
                    "--strict-mcp-config".to_string(),
                    "--allowedTools".to_string(),
                    String::new(),
                ]
            }
            AiTool::GeminiCli => {
                vec!["-p".to_string(), "-m".to_string(), model.to_string()]
            }
            AiTool::CodexCli => {
                vec!["exec".to_string(), "--model".to_string(), model.to_string()]
            }
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool: Option<AiTool>,
        /// Provider/model string (e.g. "openai/gpt-4o")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
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
#[derive(Debug, Clone, Default)]
pub struct ProcessConfig {
    /// Default AI CLI tool for AI actions that omit `tool`.
    pub default_tool: Option<AiTool>,
    /// Default model for AI actions that omit `model`.
    pub default_model: Option<String>,
    /// List of configured processing actions
    pub actions: Vec<ProcessAction>,
}

#[derive(Deserialize)]
struct ProcessActionConfig {
    /// Human-readable display name (shown in action picker)
    name: String,
    /// Action type and its type-specific configuration.
    #[serde(flatten)]
    details: ActionDetails,
}

#[derive(Serialize)]
struct ProcessActionConfigRef<'a> {
    /// Human-readable display name (shown in action picker)
    name: &'a str,
    /// Action type and its type-specific configuration.
    #[serde(flatten)]
    details: &'a ActionDetails,
}

impl<'de> Deserialize<'de> for ProcessConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawProcessConfig {
            #[serde(default)]
            default_tool: Option<AiTool>,
            #[serde(default)]
            default_model: Option<String>,
            #[serde(default)]
            actions: IndexMap<String, ProcessActionConfig>,
        }

        let raw = RawProcessConfig::deserialize(deserializer)?;
        let actions = raw
            .actions
            .into_iter()
            .map(|(id, action)| {
                let mut details = action.details;
                if let ActionDetails::Ai { tool, model, .. } = &mut details {
                    if tool.is_none() {
                        *tool = raw.default_tool.clone();
                    }
                    if model.is_none() {
                        *model = raw.default_model.clone();
                    }
                }

                ProcessAction {
                    id,
                    name: action.name,
                    details,
                }
            })
            .collect();

        Ok(Self {
            default_tool: raw.default_tool,
            default_model: raw.default_model,
            actions,
        })
    }
}

impl Serialize for ProcessConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct RawProcessConfig<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            default_tool: &'a Option<AiTool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            default_model: &'a Option<String>,
            actions: IndexMap<String, ProcessActionConfigRef<'a>>,
        }

        let actions = self
            .actions
            .iter()
            .map(|action| {
                (
                    action.id.clone(),
                    ProcessActionConfigRef {
                        name: action.name.as_str(),
                        details: &action.details,
                    },
                )
            })
            .collect();

        RawProcessConfig {
            default_tool: &self.default_tool,
            default_model: &self.default_model,
            actions,
        }
        .serialize(serializer)
    }
}

impl ProcessAction {
    /// Validates this action's configuration.
    ///
    /// Returns an error if an AI action has an empty `inputs` list.
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let ActionDetails::Ai {
            tool,
            model,
            inputs,
            ..
        } = &self.details
        {
            if tool.is_none() {
                return Err(format!(
                    "AI action '{}' must set tool or inherit process.default_tool",
                    self.id
                )
                .into());
            }
            if model.is_none() {
                return Err(format!(
                    "AI action '{}' must set model or inherit process.default_model",
                    self.id
                )
                .into());
            }
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

/// Active transcription provider/model selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptionSelectionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Complete application configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct OsttConfig {
    #[serde(default = "current_config_version")]
    pub config_version: String,
    pub audio: AudioConfig,
    #[serde(default)]
    pub transcription: TranscriptionSelectionConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub model_options: ModelOptionsConfig,
    #[serde(default)]
    pub process: ProcessConfig,
    #[serde(default)]
    pub popup: PopupConfig,
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
        config.providers.local.validate()?;
        validate_model_options(&config.model_options)?;
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
            config_version: current_config_version(),
            audio: AudioConfig {
                device: "default".to_string(),
                peak_volume_threshold: default_peak_volume_threshold(),
                reference_level_db: default_reference_level_db(),
                output_format: default_output_format(),
                visualization: VisualizationType::default(),
            },
            transcription: TranscriptionSelectionConfig::default(),
            providers: ProvidersConfig::default(),
            model_options: ModelOptionsConfig::default(),
            process: ProcessConfig::default(),
            popup: PopupConfig::default(),
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
pub(crate) fn get_config_path() -> Result<PathBuf, std::io::Error> {
    crate::app_dirs::config_path()
}

/// Saves the configuration to the config file.
///
/// # Errors
/// - If the config directory cannot be determined or created
/// - If the config file cannot be written
pub fn save_config(config: &OsttConfig) -> anyhow::Result<()> {
    config.save()
}

pub fn validate_model_options(model_options: &ModelOptionsConfig) -> anyhow::Result<()> {
    for (full_model_id, options) in model_options {
        let (provider_id, model_id) = full_model_id.split_once('/').ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid model_options key '{}'. Use 'provider/model'.",
                full_model_id
            )
        })?;

        if provider_id != "local" && model::find_model(provider_id, model_id).is_none() {
            anyhow::bail!(
                "Invalid model_options key '{}'. Unknown provider/model.",
                full_model_id
            );
        }

        if provider_id == "local" && !crate::transcription::local_models::is_safe_model_id(model_id)
        {
            anyhow::bail!(
                "Invalid model_options key '{}'. Local model id must contain only lowercase letters, digits, '.', '_' or '-'.",
                full_model_id
            );
        }

        let schema = api::option_schema(provider_id, model_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid model_options key '{}'. No options are supported for this model.",
                full_model_id
            )
        })?;

        for (option_name, value) in options {
            let Some(spec) = schema.option(option_name) else {
                anyhow::bail!(
                    "Invalid option '{}' for '{}'. Supported options: {}.",
                    option_name,
                    full_model_id,
                    schema.option_names().join(", ")
                );
            };
            validate_model_option_type(full_model_id, option_name, value, spec.kind)?;
        }

        api::validate_model_options(provider_id, full_model_id, options)?;
    }

    Ok(())
}

fn validate_model_option_type(
    full_model_id: &str,
    option_name: &str,
    value: &ModelOptionValue,
    expected: model::ModelOptionKind,
) -> anyhow::Result<()> {
    let matches = matches!(
        (expected, value),
        (model::ModelOptionKind::Bool, ModelOptionValue::Bool(_))
            | (
                model::ModelOptionKind::BoolOrString,
                ModelOptionValue::Bool(_)
            )
            | (
                model::ModelOptionKind::BoolOrString,
                ModelOptionValue::String(_)
            )
            | (
                model::ModelOptionKind::BoolOrStringList,
                ModelOptionValue::Bool(_),
            )
            | (
                model::ModelOptionKind::BoolOrStringList,
                ModelOptionValue::StringList(_),
            )
            | (
                model::ModelOptionKind::Integer,
                ModelOptionValue::Integer(_)
            )
            | (model::ModelOptionKind::Number, ModelOptionValue::Number(_))
            | (model::ModelOptionKind::Number, ModelOptionValue::Integer(_))
            | (model::ModelOptionKind::String, ModelOptionValue::String(_))
            | (
                model::ModelOptionKind::StringOrStringList,
                ModelOptionValue::String(_),
            )
            | (
                model::ModelOptionKind::StringOrStringList,
                ModelOptionValue::StringList(_),
            )
            | (
                model::ModelOptionKind::StringList,
                ModelOptionValue::StringList(_)
            )
    );

    if !matches {
        anyhow::bail!(
            "Invalid value for option '{}' in '{}'. Expected {}.",
            option_name,
            full_model_id,
            expected.name()
        );
    }

    if matches_empty_string(expected, value) {
        anyhow::bail!(
            "Invalid value for option '{}' in '{}'. Value must not be empty.",
            option_name,
            full_model_id,
        );
    }

    Ok(())
}

fn matches_empty_string(expected: model::ModelOptionKind, value: &ModelOptionValue) -> bool {
    match (expected, value) {
        (model::ModelOptionKind::String, ModelOptionValue::String(s))
        | (model::ModelOptionKind::BoolOrString, ModelOptionValue::String(s)) => s.is_empty(),
        (model::ModelOptionKind::StringOrStringList, ModelOptionValue::String(s)) => s.is_empty(),
        _ => false,
    }
}

pub fn ensure_local_transcription_audio_config() -> anyhow::Result<()> {
    let config_path = get_config_path()?;
    let content = fs::read_to_string(&config_path)?;
    let updated = ensure_local_transcription_audio_config_content(&content);
    fs::write(config_path, updated)?;
    Ok(())
}

fn ensure_local_transcription_audio_config_content(content: &str) -> String {
    let mut output = Vec::new();
    let mut in_audio = false;
    let mut saw_audio = false;
    let mut wrote_output_format = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[audio]" {
            in_audio = true;
            saw_audio = true;
            wrote_output_format = false;
            output.push(line.to_string());
            continue;
        }

        if in_audio && trimmed.starts_with('[') && trimmed.ends_with(']') {
            if !wrote_output_format {
                output.push(format!(
                    "output_format = \"{LOCAL_TRANSCRIPTION_OUTPUT_FORMAT}\""
                ));
            }
            in_audio = false;
        }

        if in_audio && trimmed.starts_with("sample_rate") {
            continue;
        } else if in_audio && trimmed.starts_with("output_format") {
            output.push(format!(
                "output_format = \"{LOCAL_TRANSCRIPTION_OUTPUT_FORMAT}\""
            ));
            wrote_output_format = true;
        } else {
            output.push(line.to_string());
        }
    }

    if in_audio {
        if !wrote_output_format {
            output.push(format!(
                "output_format = \"{LOCAL_TRANSCRIPTION_OUTPUT_FORMAT}\""
            ));
        }
    } else if !saw_audio {
        output.push(String::new());
        output.push("[audio]".to_string());
        output.push(format!(
            "output_format = \"{LOCAL_TRANSCRIPTION_OUTPUT_FORMAT}\""
        ));
    }

    format!("{}\n", output.join("\n").trim())
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

    #[test]
    fn local_transcription_audio_requires_wav_16khz() {
        let mut audio = AudioConfig {
            device: "default".to_string(),
            peak_volume_threshold: default_peak_volume_threshold(),
            reference_level_db: default_reference_level_db(),
            output_format: "pcm_s16le -ar 16000".to_string(),
            visualization: VisualizationType::default(),
        };

        assert!(is_local_transcription_audio_compatible(&audio));

        audio.output_format = default_output_format();
        assert!(!is_local_transcription_audio_compatible(&audio));
    }

    #[test]
    fn local_transcription_audio_update_preserves_other_config() {
        let content = r#"# ostt
[audio]
device = "default"
peak_volume_threshold = 90
output_format = "mp3 -ab 16k -ar 12000"
visualization = "spectrum"

[transcription]
provider = "openai"
model = "whisper-1"
"#;

        let updated = ensure_local_transcription_audio_config_content(content);

        assert!(updated.contains("device = \"default\""));
        assert!(!updated.contains("sample_rate"));
        assert!(updated.contains("peak_volume_threshold = 90"));
        assert!(updated.contains("output_format = \"pcm_s16le -ar 16000\""));
        assert!(updated.contains("[transcription]"));
        assert!(updated.contains("provider = \"openai\""));
    }

    fn validate_process_config(config: &ProcessConfig) -> Result<(), Box<dyn std::error::Error>> {
        for action in &config.actions {
            action.validate()?;
        }
        Ok(())
    }

    fn validate_ostt_config(config: &OsttConfig) -> anyhow::Result<()> {
        config.providers.local.validate()?;
        validate_model_options(&config.model_options)?;
        for action in &config.process.actions {
            action
                .validate()
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        }
        Ok(())
    }

    fn parse_ostt_config(toml_str: &str) -> Result<OsttConfig, toml::de::Error> {
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
            tool = "opencode"
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
                assert!(matches!(tool, Some(AiTool::OpenCode)));
                assert_eq!(model.as_deref(), Some("openai/gpt-4o"));
                assert_eq!(inputs.len(), 2);
            }
            _ => panic!("expected Ai variant"),
        }
    }

    #[test]
    fn valid_mixed_actions() {
        let toml_str = r#"
            [actions.copy]
            name = "Copy"
            type = "bash"
            command = "xclip"

            [actions.clean]
            name = "Clean"
            type = "ai"
            tool = "claude-code"
            model = "openai/gpt-4o"
            inputs = [{ role = "user", source = "transcription" }]
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
        "#;
        let config: OsttConfig = toml::from_str(toml_str).unwrap();
        assert!(config.process.actions.is_empty());
    }

    #[test]
    fn missing_local_provider_defaults_to_standard_local_params() {
        let toml_str = r#"
            [audio]
            device = "default"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let local = &config.providers.local;
        assert_eq!(local.language, "auto");
        assert!(local.no_timestamps);
        assert!(local.no_context);
        assert_eq!(local.temperature, 0.0);
        assert_eq!(local.entropy_thold, 2.4);
        assert_eq!(local.no_speech_thold, 0.6);
    }

    #[test]
    fn full_local_provider_deserializes() {
        let toml_str = r#"
            [audio]
            device = "default"

            [providers.local]
            language = "sv"
            no_timestamps = false
            no_context = false
            temperature = 0.2
            entropy_thold = 3.0
            no_speech_thold = 0.4
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let local = &config.providers.local;
        assert_eq!(local.language, "sv");
        assert!(!local.no_timestamps);
        assert!(!local.no_context);
        assert_eq!(local.temperature, 0.2);
        assert_eq!(local.entropy_thold, 3.0);
        assert_eq!(local.no_speech_thold, 0.4);
    }

    #[test]
    fn local_validation_rejects_invalid_global_values() {
        let cases = [
            ("temperature = 1.1", "temperature"),
            ("temperature = -0.1", "temperature"),
            ("entropy_thold = -0.1", "entropy_thold"),
            ("no_speech_thold = 1.1", "no_speech_thold"),
        ];

        for (local_setting, expected_error) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [providers.local]
                    {local_setting}
                "#
            );
            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(
                err.contains(expected_error),
                "expected '{err}' to contain '{expected_error}'"
            );
        }
    }

    #[test]
    fn local_validation_accepts_valid_values() {
        let toml_str = r#"
            [audio]
            device = "default"

            [providers.local]
            temperature = 1.0
            entropy_thold = 0.0
            no_speech_thold = 0.0
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_validate_against_model_schema() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-3"]
            detect_language = ["sv", "en"]
            smart_format = true
            keyterm = ["OSTT"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_allow_local_whisper_options_per_model() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."local/tiny"]
            language = "sv"
            no_timestamps = true
            no_context = false
            temperature = 0.0
            entropy_thold = 2.4
            no_speech_thold = 0.6

            [model_options."local/turbo"]
            language = "en"
            temperature = 0.2
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_invalid_local_whisper_values() {
        let cases = [
            ("temperature = 1.5", "Expected 0-1"),
            ("entropy_thold = -1.0", "Expected >= 0"),
            ("no_speech_thold = 1.5", "Expected 0-1"),
        ];

        for (setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."local/turbo"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_reject_unsafe_local_model_id() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."local/Turbo"]
            language = "en"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("Local model id"));
    }

    #[test]
    fn model_options_reject_wrong_value_types() {
        let cases = [
            (
                "deepgram/nova-3",
                "smart_format = \"true\"",
                "smart_format",
                "boolean",
            ),
            (
                "openai/gpt-4o-transcribe",
                "temperature = \"0.2\"",
                "temperature",
                "number",
            ),
            (
                "openai/gpt-4o-transcribe",
                "prompt = [\"OSTT\"]",
                "prompt",
                "string",
            ),
            (
                "deepgram/nova-3",
                "keyterm = \"OSTT\"",
                "keyterm",
                "string list",
            ),
            (
                "elevenlabs/scribe_v2",
                "num_speakers = \"2\"",
                "num_speakers",
                "integer",
            ),
        ];

        for (model_id, setting, option_name, expected_type) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."{model_id}"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(option_name), "{err}");
            assert!(err.contains(expected_type), "{err}");
        }
    }

    #[test]
    fn model_options_accept_bool_or_string_list_shapes() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-3"]
            detect_language = true

            [model_options."deepgram/nova-2"]
            detect_language = ["en", "sv"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_invalid_bool_or_string_list_shape() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-3"]
            detect_language = "en,sv"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("detect_language"));
        assert!(err.contains("boolean or string list"));
    }

    #[test]
    fn model_options_reject_duplicate_config_keys_at_parse_time() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-3"]
            smart_format = true
            smart_format = false
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("duplicate key") || err.contains("duplicate field"));
    }

    #[test]
    fn model_options_reject_unknown_option_for_model() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."openai/gpt-4o-transcribe"]
            smart_format = true
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("Invalid option 'smart_format'"));
    }

    #[test]
    fn model_options_allow_openai_documented_json_safe_options() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."openai/gpt-4o-transcribe"]
            include = ["logprobs"]

            [model_options."openai/whisper-1"]
            response_format = "verbose_json"
            timestamp_granularities = ["word", "segment"]

            [model_options."openai/gpt-4o-transcribe-diarize"]
            response_format = "diarized_json"
            chunking_strategy = "auto"
            known_speaker_names = ["agent"]
            known_speaker_references = ["data:audio/wav;base64,AAA..."]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_openai_options_that_break_json_parsing_or_api_rules() {
        let cases = [
            (
                "openai/gpt-4o-transcribe",
                "include = [\"timestamps\"]",
                "include",
            ),
            (
                "openai/whisper-1",
                "response_format = \"srt\"",
                "response_format",
            ),
            (
                "openai/whisper-1",
                "timestamp_granularities = [\"sentence\"]",
                "timestamp_granularities",
            ),
            (
                "openai/whisper-1",
                "response_format = \"json\"\ntimestamp_granularities = [\"word\"]",
                "verbose_json",
            ),
            (
                "openai/gpt-4o-transcribe-diarize",
                "chunking_strategy = \"none\"",
                "chunking_strategy",
            ),
        ];

        for (model_id, setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."{model_id}"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_reject_model_specific_deepgram_keyterm() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-2"]
            keyterm = ["OSTT"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("Invalid option 'keyterm'"));
    }

    #[test]
    fn model_options_allow_deepgram_nova_2_keywords() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-2"]
            keywords = ["OSTT"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_model_specific_elevenlabs_v2_option_on_v1() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."elevenlabs/scribe_v1"]
            no_verbatim = true
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("Invalid option 'no_verbatim'"));
    }

    #[test]
    fn model_options_allow_elevenlabs_documented_options() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."elevenlabs/scribe_v2"]
            language_code = "en"
            tag_audio_events = true
            timestamps_granularity = "word"
            diarize = true
            detect_speaker_roles = true
            diarization_threshold = 0.2
            temperature = 1.5
            file_format = "other"
            seed = 42
            use_multi_channel = false
            keyterms = ["OSTT"]
            no_verbatim = true
            entity_detection = ["pii", "phi"]
            entity_redaction = "pii"
            entity_redaction_mode = "enumerated_entity_type"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_invalid_elevenlabs_values() {
        let cases = [
            (
                "timestamps_granularity = \"sentence\"",
                "timestamps_granularity",
            ),
            ("file_format = \"wav\"", "file_format"),
            (
                "entity_redaction_mode = \"masked\"",
                "entity_redaction_mode",
            ),
            (
                "diarize = false\ndiarization_threshold = 0.2",
                "diarization_threshold requires diarize",
            ),
            (
                "diarize = true\nnum_speakers = 2\ndiarization_threshold = 0.2",
                "diarization_threshold cannot be used with num_speakers",
            ),
            (
                "detect_speaker_roles = true",
                "detect_speaker_roles requires diarize",
            ),
            (
                "diarize = true\ndetect_speaker_roles = true\nuse_multi_channel = true",
                "detect_speaker_roles cannot be used with use_multi_channel",
            ),
        ];

        for (setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."elevenlabs/scribe_v2"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_reject_berget_only_hotwords_on_groq() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."groq/whisper-large-v3"]
            hotwords = ["OSTT"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("Invalid option 'hotwords'"));
    }

    #[test]
    fn model_options_allow_berget_openapi_options() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."berget/KBLab/kb-whisper-large"]
            language = "sv"
            hotwords = ["OSTT", "Berget"]
            prompt = "Swedish technical dictation."
            temperature = 0.0
            response_format = "verbose_json"
            timestamp_granularities = ["word", "segment"]
            align = true
            diarize = true
            speaker_embeddings = true
            chunk_size = 30
            batch_size = 8
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_invalid_berget_values() {
        let cases = [
            ("response_format = \"srt\"", "response_format"),
            (
                "timestamp_granularities = [\"sentence\"]",
                "timestamp_granularities",
            ),
            ("chunk_size = 0", "chunk_size"),
            ("chunk_size = 61", "chunk_size"),
            ("batch_size = 0", "batch_size"),
            ("batch_size = 33", "batch_size"),
            ("stream = true", "Invalid option 'stream'"),
        ];

        for (setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."berget/openai/whisper-large-v3"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_allow_deepinfra_documented_options_and_models() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepinfra/openai/whisper-large-v3-turbo"]
            task = "transcribe"
            initial_prompt = "Names: OSTT, DeepInfra, Whisper."
            language = "en"
            temperature = 0.0
            chunk_level = "word"
            chunk_length_s = 30

            [model_options."deepinfra/mistralai/Voxtral-Mini-3B-2507"]
            task = "transcribe"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_invalid_deepinfra_values() {
        let cases = [
            ("task = \"summarize\"", "task"),
            ("chunk_level = \"sentence\"", "chunk_level"),
            ("chunk_length_s = 31", "chunk_length_s"),
            ("prompt = \"OSTT\"", "Invalid option 'prompt'"),
        ];

        for (setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."deepinfra/openai/whisper-large-v3"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_allow_groq_documented_json_safe_options() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."groq/whisper-large-v3-turbo"]
            response_format = "verbose_json"
            timestamp_granularities = ["word", "segment"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_groq_options_that_break_json_parsing_or_api_rules() {
        let cases = [
            ("response_format = \"text\"", "response_format"),
            (
                "timestamp_granularities = [\"sentence\"]",
                "timestamp_granularities",
            ),
            (
                "response_format = \"json\"\ntimestamp_granularities = [\"word\"]",
                "verbose_json",
            ),
        ];

        for (setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."groq/whisper-large-v3"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_allow_mistral_documented_options() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."mistral/voxtral-mini-latest"]
            language = "sv"
            diarize = true
            context_bias = ["OSTT"]
            temperature = 0.2
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_allow_mistral_timestamp_granularities() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."mistral/voxtral-mini-latest"]
            context_bias = ["OSTT"]
            diarize = true
            timestamp_granularities = ["word"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn model_options_reject_invalid_mistral_values() {
        let cases = [
            (
                "timestamp_granularities = [\"sentence\"]",
                "timestamp_granularities",
            ),
            (
                "language = \"en\"\ntimestamp_granularities = [\"word\"]",
                "not compatible with language",
            ),
        ];

        for (setting, expected) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    [model_options."mistral/voxtral-mini-latest"]
                    {setting}
                "#
            );

            let config = parse_ostt_config(&toml_str).unwrap();
            let err = validate_ostt_config(&config).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn model_options_reject_assemblyai_prompt_with_keyterms_prompt() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."assemblyai/universal-3-pro"]
            prompt = "Use Swedish spelling."
            keyterms_prompt = ["OSTT"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("prompt"));
        assert!(err.contains("keyterms_prompt"));
    }

    #[test]
    fn model_options_reject_out_of_range_values() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."openai/gpt-4o-transcribe"]
            temperature = 1.5
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("Expected 0-1"));
    }

    #[test]
    fn model_options_reject_empty_string_for_string_typed_option() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."openai/gpt-4o-transcribe"]
            prompt = ""
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("prompt"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn model_options_reject_empty_string_for_bool_or_string_option() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options."deepgram/nova-3"]
            language = ""
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let err = validate_ostt_config(&config).unwrap_err().to_string();
        assert!(err.contains("language"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn old_provider_level_request_options_fail_deserialization() {
        let toml_str = r#"
            [audio]
            device = "default"

            [providers.deepgram]
            smart_format = true
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("unknown field `deepgram`"));
    }

    #[test]
    fn local_config_validation_does_not_validate_active_model_id() {
        let toml_str = r#"
            [audio]
            device = "default"

            [providers.local]
            language = "auto"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn process_defaults_apply_to_ai_actions() {
        let toml_str = r#"
            default_tool = "opencode"
            default_model = "anthropic/claude-sonnet-4-6"

            [actions.clean]
            name = "Clean"
            type = "ai"
            inputs = [{ role = "user", source = "transcription" }]
        "#;

        let config = parse_process_config(toml_str).unwrap();
        assert_eq!(
            config.default_model.as_deref(),
            Some("anthropic/claude-sonnet-4-6")
        );
        match &config.actions[0].details {
            ActionDetails::Ai { tool, model, .. } => {
                assert!(matches!(tool, Some(AiTool::OpenCode)));
                assert_eq!(model.as_deref(), Some("anthropic/claude-sonnet-4-6"));
            }
            _ => panic!("expected Ai variant"),
        }
    }

    #[test]
    fn ai_action_without_tool_or_default_tool_fails_validation() {
        let toml_str = r#"
            default_model = "anthropic/claude-sonnet-4-6"

            [actions.clean]
            name = "Clean"
            type = "ai"
            inputs = [{ role = "user", source = "transcription" }]
        "#;

        let config = parse_process_config(toml_str).unwrap();
        let err = validate_process_config(&config).unwrap_err().to_string();
        assert!(err.contains("process.default_tool"));
    }

    #[test]
    fn ai_action_without_model_or_default_model_fails_validation() {
        let toml_str = r#"
            default_tool = "opencode"

            [actions.clean]
            name = "Clean"
            type = "ai"
            inputs = [{ role = "user", source = "transcription" }]
        "#;

        let config = parse_process_config(toml_str).unwrap();
        let err = validate_process_config(&config).unwrap_err().to_string();
        assert!(err.contains("process.default_model"));
    }

    #[test]
    fn ai_action_without_tool_model_or_defaults_fails_validation() {
        let toml_str = r#"
            [actions.clean]
            name = "Clean"
            type = "ai"
            inputs = [{ role = "user", source = "transcription" }]
        "#;

        let config = parse_process_config(toml_str).unwrap();
        let err = validate_process_config(&config).unwrap_err().to_string();
        assert!(err.contains("process.default_tool"));
    }

    #[test]
    fn ai_action_can_override_process_defaults() {
        let toml_str = r#"
            default_tool = "opencode"
            default_model = "anthropic/claude-sonnet-4-6"

            [actions.clean]
            name = "Clean"
            type = "ai"
            tool = "claude-code"
            model = "haiku"
            inputs = [{ role = "user", source = "transcription" }]
        "#;

        let config = parse_process_config(toml_str).unwrap();
        match &config.actions[0].details {
            ActionDetails::Ai { tool, model, .. } => {
                assert!(matches!(tool, Some(AiTool::ClaudeCode)));
                assert_eq!(model.as_deref(), Some("haiku"));
            }
            _ => panic!("expected Ai variant"),
        }
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
            tool = "opencode"

            [[inputs]]
            role = "user"
            source = "transcription"
        "#;
        let action = parse_action(toml_str).unwrap();
        assert!(action.validate().is_err());
    }

    #[test]
    fn invalid_ai_missing_inputs() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "opencode"
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
            tool = "opencode"
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
            tool = "opencode"
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
    // 2.1.B — AiTool and AI config tests
    // ═══════════════════════════════════════════════════════════════════

    /// Helper wrapper for deserializing a bare `AiTool` value from TOML.
    #[derive(Deserialize)]
    struct AiToolWrapper {
        tool: AiTool,
    }

    fn parse_ai_tool(toml_str: &str) -> Result<AiTool, toml::de::Error> {
        let wrapper: AiToolWrapper = toml::from_str(toml_str)?;
        Ok(wrapper.tool)
    }

    #[test]
    fn ai_tool_deserializes_all_kebab_case_variants() {
        let cases = [
            ("opencode", "OpenCode"),
            ("claude-code", "ClaudeCode"),
            ("gemini-cli", "GeminiCli"),
            ("codex-cli", "CodexCli"),
        ];
        for (kebab, expected_debug) in cases {
            let toml_str = format!("tool = \"{}\"", kebab);
            let tool = parse_ai_tool(&toml_str)
                .unwrap_or_else(|e| panic!("failed to parse '{}': {}", kebab, e));
            assert_eq!(
                format!("{:?}", tool),
                expected_debug,
                "variant mismatch for '{}'",
                kebab
            );
        }
    }

    #[test]
    fn ai_tool_unknown_variant_fails_deserialization() {
        let toml_str = r#"tool = "vim""#;
        assert!(parse_ai_tool(toml_str).is_err());
    }

    #[test]
    fn ai_action_missing_tool_fails_validation_without_default() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            model = "openai/gpt-4o"

            [[inputs]]
            role = "user"
            source = "transcription"
        "#;
        let action = parse_action(toml_str).unwrap();
        assert!(action.validate().is_err());
    }

    #[test]
    fn ai_action_optional_fields_default_to_none() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "opencode"
            model = "openai/gpt-4o"

            [[inputs]]
            role = "user"
            source = "transcription"
        "#;
        let action = parse_action(toml_str).unwrap();
        match &action.details {
            ActionDetails::Ai {
                tool_binary,
                tool_args,
                ..
            } => {
                assert!(tool_binary.is_none(), "tool_binary should default to None");
                assert!(tool_args.is_none(), "tool_args should default to None");
            }
            _ => panic!("expected Ai variant"),
        }
    }

    #[test]
    fn ai_tool_default_binary_returns_expected_names() {
        assert_eq!(AiTool::OpenCode.default_binary(), "opencode");
        assert_eq!(AiTool::ClaudeCode.default_binary(), "claude");
        assert_eq!(AiTool::GeminiCli.default_binary(), "gemini");
        assert_eq!(AiTool::CodexCli.default_binary(), "codex");
    }

    #[test]
    fn ai_action_tool_binary_and_tool_args_deserialize_correctly() {
        let toml_str = r#"
            id = "clean"
            name = "Clean"
            type = "ai"
            tool = "claude-code"
            model = "haiku"
            tool_binary = "/custom/path"
            tool_args = ["--flag", "value"]

            [[inputs]]
            role = "user"
            source = "transcription"
        "#;
        let action = parse_action(toml_str).unwrap();
        match &action.details {
            ActionDetails::Ai {
                tool_binary,
                tool_args,
                ..
            } => {
                assert_eq!(
                    tool_binary.as_deref(),
                    Some("/custom/path"),
                    "tool_binary should be /custom/path"
                );
                assert_eq!(
                    tool_args.as_deref(),
                    Some(&["--flag".to_string(), "value".to_string()][..]),
                    "tool_args should be [\"--flag\", \"value\"]"
                );
            }
            _ => panic!("expected Ai variant"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.1.18 — get_action lookup
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn get_action_returns_matching_action() {
        let toml_str = r#"
            [actions.clean]
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
