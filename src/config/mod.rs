//! Configuration management for ostt.
//!
//! This module handles loading and saving application configuration from TOML files,
//! as well as secure storage of API credentials. Configuration is stored in the
//! user's config directory, while credentials are stored with restricted permissions
//! in the user's local data directory.

pub mod file;
pub mod secrets;

pub use file::{AudioConfig, OsttConfig, OutputMode, VisualizationType};
pub use secrets::{clear_api_key, get_api_key, get_authorized_providers, save_api_key, save_selected_model, get_selected_model};

pub use file::save_config;
