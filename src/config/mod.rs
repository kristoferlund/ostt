//! Configuration management for ostt.
//!
//! This module handles loading and saving application configuration from TOML files,
//! as well as secure storage of API credentials. Configuration is stored in the
//! user's config directory, while credentials are stored with restricted permissions
//! in the user's local data directory.

pub mod file;
pub mod secrets;

pub use file::LocalTranscriptionConfig;
pub use file::{
    ActionDetails, ActionInput, AiTool, InputContent, InputRole, InputSource, ProcessAction,
    ProcessConfig,
};
pub use file::{
    AudioConfig, ModelOptionValue, OsttConfig, OutputConfig, ParamsConfig, PasteConfig,
    PopupConfig, ProviderConfig, ProviderConfigs, ProviderModelConfig, ProviderSettings,
    ReferenceLevel, TextConfig, TranscriptionSelectionConfig, VisualizationType,
};
pub use secrets::{
    clear_api_key, clear_selected_model, get_api_key, get_authorized_providers, get_selected_model,
    get_selected_model_entry, parse_provider_model, save_api_key, save_selected_model,
    SelectedModel,
};

pub use file::{is_local_transcription_audio_compatible, resolve_output_format, save_config};
