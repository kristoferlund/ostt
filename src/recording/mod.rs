//! Audio recording feature for ostt.
//!
//! Provides audio capture, real-time visualization, and user interaction handling
//! for the recording workflow.

pub mod audio;
pub mod ffmpeg;
pub mod recording_history;
pub mod ui;
pub mod visualizations;

pub use audio::AudioRecorder;
pub use ffmpeg::find_ffmpeg;
pub use recording_history::RecordingHistory;
pub use ui::{RecordingCommand, OsttTui};
