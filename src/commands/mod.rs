//! Application command handlers for ostt.
//!
//! This module organizes command handling into separate submodules, each responsible for a specific
//! application command (auth, record, history viewing).
//!
//! # Commands
//! - `auth`: Provider + model selection and API key management (unified flow)
//! - `record`: Audio recording with optional transcription
//! - `history`: Transcription history viewer
//! - `keywords`: Keyword management for transcription
//! - `config`: Open configuration file in user's preferred editor
//! - `list_devices`: List available audio input devices
//! - `logs`: Display recent log entries
//! - `retry`: Retry the last recording with the same transcription model
//! - `replay`: Replay a previous recording from history

pub mod auth;
pub mod record;
pub mod history;
pub mod keywords;
pub mod config;
pub mod list_devices;
pub mod logs;
pub mod retry;
pub mod replay;
pub mod transcribe;

pub use auth::handle_auth;
pub use record::handle_record;
pub use history::handle_history;
pub use keywords::handle_keywords;
pub use config::handle_config;
pub use list_devices::handle_list_devices;
pub use logs::handle_logs;
pub use retry::handle_retry;
pub use replay::handle_replay;
pub use transcribe::handle_transcribe;
