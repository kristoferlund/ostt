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

/// Built-in whisper.cpp transcription defaults.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelOptionValue {
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    StringList(Vec<String>),
}

pub type ParamsConfig = IndexMap<String, ModelOptionValue>;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderModelConfig {
    #[serde(flatten)]
    pub settings: ProviderSettings,
    #[serde(default)]
    pub params: ParamsConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(flatten)]
    pub settings: ProviderSettings,
    #[serde(default)]
    pub params: ParamsConfig,
    #[serde(default)]
    pub models: IndexMap<String, ProviderModelConfig>,
}

pub type ProviderConfigs = IndexMap<String, ProviderConfig>;

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

const FIXED_TOP_LEVEL_SECTIONS: &[&str] = &["audio", "transcription", "process", "popup"];
const DEPRECATED_TOP_LEVEL_SECTIONS: &[&str] = &["providers", "model_options"];

/// Complete application configuration.
#[derive(Debug)]
pub struct OsttConfig {
    pub config_version: String,
    pub audio: AudioConfig,
    pub transcription: TranscriptionSelectionConfig,
    pub provider_configs: ProviderConfigs,
    pub process: ProcessConfig,
    pub popup: PopupConfig,
}

impl<'de> Deserialize<'de> for OsttConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut table = toml::Table::deserialize(deserializer)?;
        parse_config_table(&mut table).map_err(serde::de::Error::custom)
    }
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
        validate_config(&config)?;
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
        let config_content = toml::to_string_pretty(&config_to_table(self)?)?;
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
            provider_configs: ProviderConfigs::default(),
            process: ProcessConfig::default(),
            popup: PopupConfig::default(),
        }
    }
}

fn parse_config_table(table: &mut toml::Table) -> anyhow::Result<OsttConfig> {
    reject_deprecated_top_level_sections(table)?;
    reject_deprecated_section_keys(table)?;

    let config_version = match table.remove("config_version") {
        Some(value) => value.try_into()?,
        None => current_config_version(),
    };
    let audio = take_required_section::<AudioConfig>(table, "audio")?;
    let transcription =
        take_optional_section::<TranscriptionSelectionConfig>(table, "transcription")?;
    let process = take_optional_section::<ProcessConfig>(table, "process")?;
    let popup = take_optional_section::<PopupConfig>(table, "popup")?;

    let mut provider_configs = ProviderConfigs::new();
    let provider_tables = std::mem::take(table);
    for (provider_id, value) in provider_tables {
        if FIXED_TOP_LEVEL_SECTIONS.contains(&provider_id.as_str()) {
            anyhow::bail!("duplicate top-level section '{provider_id}'");
        }
        if crate::transcription::TranscriptionProvider::from_id(&provider_id).is_none() {
            anyhow::bail!(
                "Unknown top-level config section '{}'. Expected one of: audio, transcription, process, popup, {}.",
                provider_id,
                crate::transcription::TranscriptionProvider::supported_ids().join(", ")
            );
        }
        let toml::Value::Table(provider_table) = value else {
            anyhow::bail!("Provider config '{provider_id}' must be a table");
        };
        provider_configs.insert(
            provider_id.clone(),
            parse_provider_config(&provider_id, provider_table)?,
        );
    }

    let config = OsttConfig {
        config_version,
        audio,
        transcription,
        provider_configs,
        process,
        popup,
    };
    validate_config(&config)?;
    Ok(config)
}

fn reject_deprecated_top_level_sections(table: &toml::Table) -> anyhow::Result<()> {
    for section in DEPRECATED_TOP_LEVEL_SECTIONS {
        if table.contains_key(*section) {
            anyhow::bail!(
                "Deprecated config section '[{}]' is no longer supported. Use top-level provider sections with '.params', for example '[whisper.params]' or '[openai.gpt-4o-transcribe.params]'.",
                section
            );
        }
    }
    Ok(())
}

fn reject_deprecated_section_keys(table: &toml::Table) -> anyhow::Result<()> {
    if table
        .get("audio")
        .and_then(toml::Value::as_table)
        .is_some_and(|audio| audio.contains_key("sample_rate"))
    {
        anyhow::bail!(
            "Deprecated config key '[audio].sample_rate' is no longer supported. Use '[audio].output_format' instead, for example output_format = \"mp3 -ab 16k -ar 12000\"."
        );
    }

    if table
        .get("process")
        .and_then(toml::Value::as_table)
        .and_then(|process| process.get("actions"))
        .is_some_and(toml::Value::is_array)
    {
        anyhow::bail!(
            "Deprecated config table '[[process.actions]]' is no longer supported. Use named action tables instead, for example '[process.actions.clean]'."
        );
    }

    Ok(())
}

fn take_required_section<T>(table: &mut toml::Table, name: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let Some(value) = table.remove(name) else {
        anyhow::bail!("Missing required config section '[{name}]'");
    };
    Ok(value.try_into()?)
}

fn take_optional_section<T>(table: &mut toml::Table, name: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match table.remove(name) {
        Some(value) => Ok(value.try_into()?),
        None => Ok(T::default()),
    }
}

fn parse_provider_config(provider_id: &str, table: toml::Table) -> anyhow::Result<ProviderConfig> {
    let mut config = ProviderConfig::default();

    for (key, value) in table {
        if key == "params" {
            config.params = parse_params_table(provider_id, None, value)?;
            continue;
        }

        if is_provider_setting_key(&key) {
            set_provider_setting(&mut config.settings, &key, value)?;
            continue;
        }

        let toml::Value::Table(model_table) = value else {
            anyhow::bail!(
                "Unknown setting '{}' in provider config '[{}]'",
                key,
                provider_id
            );
        };
        config.models.insert(
            key.clone(),
            parse_provider_model_config(provider_id, &key, model_table)?,
        );
    }

    Ok(config)
}

fn parse_provider_model_config(
    provider_id: &str,
    model_id: &str,
    table: toml::Table,
) -> anyhow::Result<ProviderModelConfig> {
    let mut config = ProviderModelConfig::default();

    for (key, value) in table {
        if key == "params" {
            config.params = parse_params_table(provider_id, Some(model_id), value)?;
            continue;
        }

        if is_provider_setting_key(&key) {
            set_provider_setting(&mut config.settings, &key, value)?;
            continue;
        }

        anyhow::bail!(
            "Unknown setting '{}' in provider model config '[{}.{}]'",
            key,
            provider_id,
            model_id
        );
    }

    Ok(config)
}

fn parse_params_table(
    provider_id: &str,
    model_id: Option<&str>,
    value: toml::Value,
) -> anyhow::Result<ParamsConfig> {
    let toml::Value::Table(table) = value else {
        anyhow::bail!("Params for provider '{}' must be a table", provider_id);
    };
    let params: ParamsConfig = toml::Value::Table(table).try_into()?;
    if let Some(model_id) = model_id {
        validate_params_for_model(provider_id, model_id, &params)?;
    }
    Ok(params)
}

fn is_provider_setting_key(key: &str) -> bool {
    matches!(
        key,
        "output_format"
            | "timeout_secs"
            | "command"
            | "endpoint"
            | "model"
            | "api_key_env"
            | "display_name"
    )
}

fn set_provider_setting(
    settings: &mut ProviderSettings,
    key: &str,
    value: toml::Value,
) -> anyhow::Result<()> {
    match key {
        "output_format" => settings.output_format = Some(value.try_into()?),
        "timeout_secs" => settings.timeout_secs = Some(value.try_into()?),
        "command" => settings.command = Some(value.try_into()?),
        "endpoint" => settings.endpoint = Some(value.try_into()?),
        "model" => settings.model = Some(value.try_into()?),
        "api_key_env" => settings.api_key_env = Some(value.try_into()?),
        "display_name" => settings.display_name = Some(value.try_into()?),
        _ => unreachable!("unknown provider setting was pre-filtered"),
    }
    Ok(())
}

fn config_to_table(config: &OsttConfig) -> anyhow::Result<toml::Table> {
    let mut table = toml::Table::new();
    table.insert(
        "config_version".to_string(),
        toml::Value::String(config.config_version.clone()),
    );
    table.insert("audio".to_string(), toml::Value::try_from(&config.audio)?);
    if config.transcription.provider.is_some() || config.transcription.model.is_some() {
        table.insert(
            "transcription".to_string(),
            toml::Value::try_from(&config.transcription)?,
        );
    }
    for (provider_id, provider_config) in &config.provider_configs {
        table.insert(
            provider_id.clone(),
            provider_config_to_value(provider_config)?,
        );
    }
    if !config.process.actions.is_empty() {
        table.insert(
            "process".to_string(),
            toml::Value::try_from(&config.process)?,
        );
    }
    table.insert("popup".to_string(), toml::Value::try_from(&config.popup)?);
    Ok(table)
}

fn provider_config_to_value(config: &ProviderConfig) -> anyhow::Result<toml::Value> {
    let mut table = provider_settings_to_table(&config.settings)?;
    if !config.params.is_empty() {
        table.insert("params".to_string(), toml::Value::try_from(&config.params)?);
    }
    for (model_id, model_config) in &config.models {
        table.insert(
            model_id.clone(),
            provider_model_config_to_value(model_config)?,
        );
    }
    Ok(toml::Value::Table(table))
}

fn provider_model_config_to_value(config: &ProviderModelConfig) -> anyhow::Result<toml::Value> {
    let mut table = provider_settings_to_table(&config.settings)?;
    if !config.params.is_empty() {
        table.insert("params".to_string(), toml::Value::try_from(&config.params)?);
    }
    Ok(toml::Value::Table(table))
}

fn provider_settings_to_table(settings: &ProviderSettings) -> anyhow::Result<toml::Table> {
    let mut table = toml::Table::new();
    if let Some(value) = &settings.output_format {
        table.insert(
            "output_format".to_string(),
            toml::Value::String(value.clone()),
        );
    }
    if let Some(value) = settings.timeout_secs {
        table.insert("timeout_secs".to_string(), toml::Value::try_from(value)?);
    }
    if let Some(value) = &settings.command {
        table.insert("command".to_string(), toml::Value::String(value.clone()));
    }
    if let Some(value) = &settings.endpoint {
        table.insert("endpoint".to_string(), toml::Value::String(value.clone()));
    }
    if let Some(value) = &settings.model {
        table.insert("model".to_string(), toml::Value::String(value.clone()));
    }
    if let Some(value) = &settings.api_key_env {
        table.insert(
            "api_key_env".to_string(),
            toml::Value::String(value.clone()),
        );
    }
    if let Some(value) = &settings.display_name {
        table.insert(
            "display_name".to_string(),
            toml::Value::String(value.clone()),
        );
    }
    Ok(table)
}

pub fn validate_config(config: &OsttConfig) -> anyhow::Result<()> {
    for (provider_id, provider_config) in &config.provider_configs {
        if crate::transcription::TranscriptionProvider::from_id(provider_id).is_none() {
            anyhow::bail!("Unknown provider config '{}'.", provider_id);
        }

        for model_id in provider_config.models.keys() {
            validate_model_id(provider_id, model_id)?;
        }
    }

    if config.transcription.provider.as_deref() == Some("local") {
        anyhow::bail!("Provider 'local' is no longer supported. Use provider = \"whisper\".");
    }

    Ok(())
}

pub fn resolve_output_format(
    config: &OsttConfig,
    selected_model: &crate::config::SelectedModel,
) -> String {
    if let Some(output_format) = config
        .provider_configs
        .get(&selected_model.provider_id)
        .and_then(|provider| provider.models.get(&selected_model.model_id))
        .and_then(|model| model.settings.output_format.as_deref())
    {
        return output_format.to_string();
    }

    if let Some(output_format) = config
        .provider_configs
        .get(&selected_model.provider_id)
        .and_then(|provider| provider.settings.output_format.as_deref())
    {
        return output_format.to_string();
    }

    if selected_model.provider_id == "whisper" {
        return LOCAL_TRANSCRIPTION_OUTPUT_FORMAT.to_string();
    }

    config.audio.output_format.clone()
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

pub fn validate_model_id(provider_id: &str, model_id: &str) -> anyhow::Result<()> {
    if provider_id != "whisper" && model::find_model(provider_id, model_id).is_none() {
        anyhow::bail!(
            "Unknown model '{}' for provider '{}'. Please run 'ostt model' to select a supported model.",
            model_id,
            provider_id
        );
    }

    if provider_id == "whisper" && !crate::transcription::local_models::is_safe_model_id(model_id) {
        anyhow::bail!(
            "Whisper model id '{}' must contain only lowercase letters, digits, '.', '_' or '-'.",
            model_id
        );
    }

    Ok(())
}

pub fn validate_params_for_model(
    provider_id: &str,
    model_id: &str,
    params: &ParamsConfig,
) -> anyhow::Result<()> {
    validate_model_id(provider_id, model_id)?;
    let full_model_id = format!("{provider_id}/{model_id}");
    let schema = api::option_schema(provider_id, model_id).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid params for '{}'. No params are supported for this model.",
            full_model_id
        )
    })?;

    for (option_name, value) in params {
        let Some(spec) = schema.option(option_name) else {
            anyhow::bail!(
                "Invalid param '{}' for '{}'. Supported params: {}.",
                option_name,
                full_model_id,
                schema.option_names().join(", ")
            );
        };
        validate_model_option_type(&full_model_id, option_name, value, spec.kind)?;
    }

    api::validate_params(provider_id, &full_model_id, params)?;

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
            "Invalid value for param '{}' in '{}'. Expected {}.",
            option_name,
            full_model_id,
            expected.name()
        );
    }

    if matches_empty_string(expected, value) {
        anyhow::bail!(
            "Invalid value for param '{}' in '{}'. Value must not be empty.",
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

    fn validate_process_config(config: &ProcessConfig) -> Result<(), Box<dyn std::error::Error>> {
        for action in &config.actions {
            action.validate()?;
        }
        Ok(())
    }

    fn validate_ostt_config(config: &OsttConfig) -> anyhow::Result<()> {
        validate_config(config)?;
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
    fn missing_whisper_provider_uses_builtin_output_format_default() {
        let toml_str = r#"
            [audio]
            device = "default"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let selected = crate::config::SelectedModel {
            provider_id: "whisper".to_string(),
            model_id: "turbo".to_string(),
        };
        assert_eq!(
            resolve_output_format(&config, &selected),
            "pcm_s16le -ar 16000"
        );
    }

    #[test]
    fn full_whisper_params_deserialize() {
        let toml_str = r#"
            [audio]
            device = "default"

            [whisper.params]
            language = "sv"
            no_timestamps = false
            no_context = false
            temperature = 0.2
            entropy_thold = 3.0
            no_speech_thold = 0.4
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        let params = &config.provider_configs["whisper"].params;
        assert_eq!(
            params["language"],
            ModelOptionValue::String("sv".to_string())
        );
        assert_eq!(params["no_timestamps"], ModelOptionValue::Bool(false));
        assert_eq!(params["no_context"], ModelOptionValue::Bool(false));
        assert_eq!(params["temperature"], ModelOptionValue::Number(0.2));
        assert_eq!(params["entropy_thold"], ModelOptionValue::Number(3.0));
        assert_eq!(params["no_speech_thold"], ModelOptionValue::Number(0.4));
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

                    [whisper.turbo.params]
                    {local_setting}
                "#
            );
            let err = parse_ostt_config(&toml_str).unwrap_err().to_string();
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

            [whisper.params]
            temperature = 1.0
            entropy_thold = 0.0
            no_speech_thold = 0.0
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn params_validate_against_model_schema() {
        let toml_str = r#"
            [audio]
            device = "default"

            [deepgram.nova-3.params]
            detect_language = ["sv", "en"]
            smart_format = true
            keyterm = ["OSTT"]
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn params_allow_whisper_params_per_model() {
        let toml_str = r#"
            [audio]
            device = "default"

            [whisper.tiny.params]
            language = "sv"
            no_timestamps = true
            no_context = false
            temperature = 0.0
            entropy_thold = 2.4
            no_speech_thold = 0.6

            [whisper.turbo.params]
            language = "en"
            temperature = 0.2
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn params_reject_invalid_whisper_values() {
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

                    [whisper.turbo.params]
                    {setting}
                "#
            );

            let err = parse_ostt_config(&toml_str).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn params_reject_unsafe_whisper_model_id() {
        let toml_str = r#"
            [audio]
            device = "default"

            [whisper.Turbo.params]
            language = "en"
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Whisper model id"));
    }

    #[test]
    fn params_reject_wrong_value_types() {
        let cases = [
            (
                "[deepgram.nova-3.params]",
                "smart_format = \"true\"",
                "smart_format",
                "boolean",
            ),
            (
                "[openai.gpt-4o-transcribe.params]",
                "temperature = \"0.2\"",
                "temperature",
                "number",
            ),
            (
                "[openai.gpt-4o-transcribe.params]",
                "prompt = [\"OSTT\"]",
                "prompt",
                "string",
            ),
            (
                "[deepgram.nova-3.params]",
                "keyterm = \"OSTT\"",
                "keyterm",
                "string list",
            ),
            (
                "[elevenlabs.scribe_v2.params]",
                "num_speakers = \"2\"",
                "num_speakers",
                "integer",
            ),
        ];

        for (header, setting, param_name, expected_type) in cases {
            let toml_str = format!(
                r#"
                    [audio]
                    device = "default"

                    {header}
                    {setting}
                "#
            );

            let err = parse_ostt_config(&toml_str).unwrap_err().to_string();
            assert!(err.contains(param_name), "{err}");
            assert!(err.contains(expected_type), "{err}");
        }
    }

    #[test]
    fn params_parse_quoted_model_ids_with_slashes() {
        let toml_str = r#"
            [audio]
            device = "default"

            [berget."openai/whisper-large-v3".params]
            language = "sv"
            hotwords = ["OSTT", "Berget"]
            prompt = "Swedish technical dictation."
            temperature = 0.0

            [deepinfra."mistralai/Voxtral-Mini-3B-2507".params]
            task = "transcribe"
        "#;

        let config = parse_ostt_config(toml_str).unwrap();
        validate_ostt_config(&config).unwrap();
    }

    #[test]
    fn params_reject_unknown_param_for_model() {
        let toml_str = r#"
            [audio]
            device = "default"

            [openai.gpt-4o-transcribe.params]
            smart_format = true
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Invalid param 'smart_format'"));
    }

    #[test]
    fn deprecated_providers_section_fails_deserialization() {
        let toml_str = r#"
            [audio]
            device = "default"

            [providers]
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Deprecated config section '[providers]'"));
    }

    #[test]
    fn deprecated_model_options_section_fails_deserialization() {
        let toml_str = r#"
            [audio]
            device = "default"

            [model_options]
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Deprecated config section '[model_options]'"));
    }

    #[test]
    fn deprecated_audio_sample_rate_fails_deserialization() {
        let toml_str = r#"
            [audio]
            device = "default"
            sample_rate = 16000
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Deprecated config key '[audio].sample_rate'"));
    }

    #[test]
    fn deprecated_process_actions_array_fails_deserialization() {
        let toml_str = r#"
            [audio]
            device = "default"

            [[process.actions]]
            id = "clean"
            name = "Clean up"
            type = "bash"
            command = "cat"
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Deprecated config table '[[process.actions]]'"));
    }

    #[test]
    fn deprecated_local_provider_selection_fails_validation() {
        let toml_str = r#"
            [audio]
            device = "default"

            [transcription]
            provider = "local"
            model = "turbo"
        "#;

        let err = parse_ostt_config(toml_str).unwrap_err().to_string();
        assert!(err.contains("Provider 'local' is no longer supported"));
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
