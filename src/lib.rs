//! ostt - Open Speech-to-Text
//!
//! An interactive terminal-based audio recording and speech-to-text transcription tool.
//!
//! ostt allows you to:
//! - Record audio with real-time waveform visualization and volume metering
//! - Automatically transcribe recordings using multiple AI providers and models
//! - Maintain a searchable history of all transcriptions
//! - Configure and authenticate with any supported transcription provider
//! - Select from available models for each provider
//!
//! Built with Rust for performance and minimal dependencies, ostt provides a command-line
//! interface for recording, provider authentication, model selection, and history browsing.

pub mod app;
pub mod auth;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod history;
pub mod keywords;
pub mod logging;
pub mod recording;
pub mod setup;
pub mod transcription;
pub mod ui;

pub use app::run;
