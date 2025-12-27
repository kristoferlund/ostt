//! Visualization modules for recording display.
//!
//! Each module provides a different way to visualize audio during recording.
//! New visualization types can be added by creating a new module here.

pub mod spectrum;
pub mod waveform;

pub use spectrum::SpectrumAnalyzer;
pub use waveform::{update_waveform, resize_waveform};
