//! Audio recording feature for ostt.
//!
//! Provides audio capture, real-time visualization, and user interaction handling
//! for the recording workflow.

pub mod audio;
pub mod ffmpeg;
pub mod ostt_tui;
pub mod recording_history;
pub mod visualizations;

pub use audio::AudioRecorder;
pub use ffmpeg::find_ffmpeg;
pub use ostt_tui::{OsttTui, PickerEvent, RecordingCommand};
pub use recording_history::RecordingHistory;
