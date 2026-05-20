//! Application command handlers for ostt.
//!
//! This module organizes command handling into separate submodules, each responsible for a specific
//! application command (auth, record, history viewing).
//!
//! # Commands
//! - `auth`: Provider + model selection and API key management (unified flow)
//! - `record`: Audio recording with optional transcription
//! - `history`: Transcription history view
//! - `keywords`: Keyword management for transcription
//! - `config`: Open configuration file in user's preferred editor
//! - `list_devices`: List available audio input devices
//! - `logs`: Display recent log entries
//! - `retry`: Retry the last recording with the same transcription model
//! - `replay`: Replay a previous recording from history

pub mod auth;
pub mod config;
pub mod history;
pub mod keywords;
pub mod launch;
pub mod list_devices;
pub mod logs;
pub mod model;
pub mod process;
pub mod record;
pub mod replay;
pub mod retry;
pub mod transcribe;

pub use auth::handle_auth;
pub use config::handle_config;
pub use history::handle_history;
pub use keywords::handle_keywords;
pub use launch::handle_launch;
pub use list_devices::handle_list_devices;
pub use logs::handle_logs;
pub use model::handle_model;
pub use process::handle_process;
pub use record::handle_record;
pub use replay::handle_replay;
pub use retry::handle_retry;
pub use transcribe::handle_transcribe;
