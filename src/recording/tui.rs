//! Terminal user interface for audio recording with configurable visualization.
//!
//! Supports frequency spectrum and time-domain waveform visualization modes.
//! Handles real-time display updates, volume metering, and user input during recording.

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{ListState, Paragraph, Sparkline, Wrap},
};
use std::error::Error;
use std::io::{stdout, Stdout};

use crate::config::{file::ProcessAction, OsttConfig};
use crate::config::{ReferenceLevel, VisualizationType};
use crate::process::process_view::{handle_picker_event, render_process_view, PickerResult};
use crate::transcription::TranscriptionAnimation;
use crate::ui::is_cancel_key;

use super::visualizations::{
    center_out_layout, resize_waveform, update_waveform, SpectrumAnalyzer,
};

const PENDING_AUDIO_SAMPLE_RATE: u32 = 48_000;

/// How fast peak caps sink, in display units (0-100) per rendered frame
/// (~20 fps → full-scale fall in ~3.3 seconds).
const CAP_FALL_PER_FRAME: f32 = 1.5;

/// True-peak level treated as clipping for the red indicator (auto mode).
/// Absolute, so it needs no per-machine calibration.
const CLIP_PEAK_DB: f32 = -3.0;

/// How long the clip indicator stays lit after a clip is detected.
const CLIP_HOLD: std::time::Duration = std::time::Duration::from_millis(1500);

/// Starting point for the adaptive reference level before any speech is heard.
const ADAPTIVE_REF_INITIAL_DB: f32 = -24.0;

/// Bounds for the adaptive reference level.
const ADAPTIVE_REF_MIN_DB: f32 = -35.0;
const ADAPTIVE_REF_MAX_DB: f32 = -6.0;

/// Slow release rate per rendered frame (~0.5 dB/s at 20 fps), applied while
/// signal is present but below the current reference.
const ADAPTIVE_REF_RELEASE_PER_FRAME: f32 = 0.025;

/// User input command during recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingCommand {
    /// Continue recording (no key pressed)
    Continue,
    /// Proceed to transcription (Enter key)
    Transcribe,
    /// Exit without transcription (Escape or 'q')
    Cancel,
    /// Pause/resume recording (Space key)
    TogglePause,
}

/// Terminal UI for audio recording with configurable visualization.
///
/// Supports multiple visualization types: frequency spectrum or time-domain waveform.
/// Displays real-time visualization, volume levels, recording duration, and animated transcription progress.
pub struct RecordingTui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    display_data: Vec<u64>,
    last_sample_time: std::time::Instant,
    sample_interval: std::time::Duration,
    terminal_width: usize,
    sample_rate: u32,
    recording_start_time: std::time::Instant,
    peak_hold: u8,
    peak_hold_time: std::time::Instant,
    peak_volume_threshold: u8,
    /// Configured reference level: auto (adaptive) or fixed dBFS
    reference_level: ReferenceLevel,
    /// Current adaptive reference in dBFS (used in auto mode)
    adaptive_ref_db: f32,
    /// When clipping was last detected (drives the red indicator in auto mode)
    last_clip_time: Option<std::time::Instant>,
    /// Whether recording is currently paused
    pub is_paused: bool,
    /// Total time paused (accumulated when paused)
    pause_duration: std::time::Duration,
    /// When pause started (for calculating pause duration)
    pause_start_time: Option<std::time::Instant>,
    /// Visualization type (spectrum or waveform)
    visualization_type: VisualizationType,
    /// Spectrum analyzer (used when visualization_type is Spectrum)
    spectrum_analyzer: Option<SpectrumAnalyzer>,
    /// Per-column peak cap positions (spectrum mode), sinking slowly over time
    peak_caps: Vec<f32>,
    cleaned_up: bool,
}

impl RecordingTui {
    /// Creates a new TUI instance and enters alternate screen mode.
    ///
    /// # Errors
    /// - If terminal cannot be initialized
    /// - If raw mode cannot be enabled
    /// - If alternate screen cannot be entered
    pub fn new(config: &OsttConfig, sample_rate: u32) -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let size = terminal.size()?;
        let terminal_width = size.width as usize;

        let sample_interval = std::time::Duration::from_millis(50);

        // Initialize visualization-specific data
        let display_data = vec![0u64; terminal_width];
        let spectrum_analyzer = if config.audio.visualization == VisualizationType::Spectrum {
            Some(SpectrumAnalyzer::new(terminal_width))
        } else {
            None
        };

        let now = std::time::Instant::now();
        Ok(RecordingTui {
            terminal,
            display_data,
            last_sample_time: now,
            sample_interval,
            terminal_width,
            sample_rate,
            recording_start_time: now,
            peak_hold: 0,
            peak_hold_time: now,
            peak_volume_threshold: config.audio.peak_volume_threshold,
            reference_level: config.audio.reference_level_db,
            adaptive_ref_db: ADAPTIVE_REF_INITIAL_DB,
            last_clip_time: None,
            is_paused: false,
            pause_duration: std::time::Duration::ZERO,
            pause_start_time: None,
            visualization_type: config.audio.visualization,
            spectrum_analyzer,
            peak_caps: Vec::new(),
            cleaned_up: false,
        })
    }

    /// Creates a TUI before the audio device has reported its actual sample rate.
    pub fn new_pending_audio(config: &OsttConfig) -> Result<Self, Box<dyn Error>> {
        Self::new(config, PENDING_AUDIO_SAMPLE_RATE)
    }

    /// Updates sample-rate-dependent state after audio startup succeeds.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        let now = std::time::Instant::now();
        self.recording_start_time = now;
        self.last_sample_time = now;
        self.peak_hold_time = now;
        self.pause_duration = std::time::Duration::ZERO;
        self.pause_start_time = None;
        self.is_paused = false;
        self.adaptive_ref_db = ADAPTIVE_REF_INITIAL_DB;
        self.last_clip_time = None;
    }

    /// Returns the reference level (dBFS) currently used for meter scaling.
    fn effective_reference_db(&self) -> f32 {
        match self.reference_level {
            ReferenceLevel::Db(v) => f32::from(v),
            ReferenceLevel::Auto => self.adaptive_ref_db,
        }
    }

    /// Returns true if clipping was detected recently (auto mode indicator).
    fn is_clipping(&self) -> bool {
        self.last_clip_time.is_some_and(|t| t.elapsed() < CLIP_HOLD)
    }

    /// Renders the visualization with current volume and recording duration.
    ///
    /// # Errors
    /// - If terminal rendering fails
    pub fn render_waveform(
        &mut self,
        samples: &[i16],
        raw_true_peak: u16,
    ) -> Result<(), Box<dyn Error>> {
        let current_volume = self.calculate_volume(samples, raw_true_peak);
        let reference_db = self.effective_reference_db();

        if !self.is_paused && self.last_sample_time.elapsed() >= self.sample_interval {
            match self.visualization_type {
                VisualizationType::Spectrum => {
                    if let Some(analyzer) = &mut self.spectrum_analyzer {
                        analyzer.update(samples, self.sample_rate, reference_db);
                        self.display_data = analyzer.data().to_vec();
                    }
                }
                VisualizationType::Waveform => {
                    update_waveform(&mut self.display_data, current_volume, self.terminal_width);
                }
            }

            self.last_sample_time = std::time::Instant::now();
        }

        let size = self.terminal.size()?;
        let current_width = size.width as usize;

        if current_width != self.terminal_width {
            self.terminal_width = current_width;

            match self.visualization_type {
                VisualizationType::Spectrum => {
                    if let Some(analyzer) = &mut self.spectrum_analyzer {
                        analyzer.resize(current_width, samples, self.sample_rate, reference_db);
                        self.display_data = analyzer.data().to_vec();
                    }
                }
                VisualizationType::Waveform => {
                    resize_waveform(&mut self.display_data, self.terminal_width);
                }
            }
        }

        // Pre-calculate values to avoid borrow checker issues in closure
        let is_paused = self.is_paused;
        let peak_hold = self.peak_hold;
        let recording_duration = self.get_recording_duration();

        // Auto mode: red means actual clipping (absolute, no calibration).
        // Fixed mode: red means exceeding the configured threshold.
        let peak_alert = !is_paused
            && match self.reference_level {
                ReferenceLevel::Auto => self.is_clipping(),
                ReferenceLevel::Db(_) => peak_hold >= self.peak_volume_threshold,
            };

        // Spectrum mode renders center-out: low frequencies in the middle,
        // highs mirrored toward both edges. Sized to the padded content width
        // so the mirror axis lands on the visual center of the screen.
        let content_width = current_width.saturating_sub(4);
        let render_data: Vec<u64> = match self.visualization_type {
            VisualizationType::Spectrum => center_out_layout(&self.display_data, content_width),
            VisualizationType::Waveform => self.display_data.clone(),
        };

        // Peak caps: hold each column's recent maximum and let it sink slowly
        let show_caps = self.visualization_type == VisualizationType::Spectrum;
        if show_caps {
            if self.peak_caps.len() != render_data.len() {
                self.peak_caps = vec![0.0; render_data.len()];
            }
            if !is_paused {
                for (cap, &v) in self.peak_caps.iter_mut().zip(render_data.iter()) {
                    *cap = (*cap - CAP_FALL_PER_FRAME).max(v as f32);
                }
            }
        }

        // Dim the whole visualization while paused so the state is obvious
        let (viz_fg, mirror_bg, footer_fg, cap_fg) = if is_paused {
            (
                Color::Rgb(98, 110, 113),
                Color::Rgb(88, 99, 102),
                Color::Rgb(98, 110, 113),
                Color::Rgb(120, 127, 125),
            )
        } else {
            (
                Color::Rgb(206, 224, 220),
                Color::Rgb(185, 207, 212),
                Color::Rgb(185, 207, 212),
                Color::Rgb(240, 250, 246),
            )
        };

        self.terminal.draw(|frame| {
            let area = frame.area();
            let background = ratatui::widgets::Block::default().style(
                Style::default()
                    .bg(Color::Rgb(0, 0, 0))
                    .fg(Color::Rgb(185, 207, 212)),
            );
            frame.render_widget(background, area);

            let padded_area = Rect {
                x: area.x.saturating_add(2),
                y: area.y.saturating_add(1),
                width: area.width.saturating_sub(4),
                height: area.height.saturating_sub(2),
            };

            let footer_height = 1;

            let content_area = Rect {
                x: padded_area.x,
                y: padded_area.y,
                width: padded_area.width,
                height: padded_area.height.saturating_sub(footer_height),
            };

            let top_area_height = content_area.height / 3 * 2;

            let top_area = Rect {
                x: content_area.x,
                y: content_area.y,
                width: content_area.width,
                height: top_area_height,
            };

            let top_sparkline = Sparkline::default()
                .data(&render_data)
                .max(100)
                .style(Style::default().bg(Color::Rgb(0, 0, 0)).fg(viz_fg));

            frame.render_widget(top_sparkline, top_area);

            // Peak caps: a thin marker resting just above each column's bar
            if show_caps {
                let max_rows = i32::from(top_area.height);
                let buf = frame.buffer_mut();
                for (i, &cap) in self.peak_caps.iter().enumerate() {
                    if i >= top_area.width as usize {
                        break;
                    }
                    let cap_row = ((cap / 100.0) * (max_rows * 8) as f32) as i32 / 8;
                    let bar_row = render_data[i] as i32 * max_rows * 8 / 100 / 8;
                    // Only draw while the cap floats above the bar's top cell,
                    // so it never clobbers the bar's partial block glyph
                    if cap_row > bar_row && cap_row < max_rows {
                        let x = top_area.x + i as u16;
                        let y = top_area.y + top_area.height - 1 - cap_row as u16;
                        buf.set_string(
                            x,
                            y,
                            "▁",
                            Style::default().fg(cap_fg).bg(Color::Rgb(0, 0, 0)),
                        );
                    }
                }
            }

            let bottom_area = Rect {
                x: content_area.x,
                y: content_area.y + top_area_height,
                width: content_area.width,
                height: content_area.height.saturating_sub(top_area_height),
            };

            let inverted_data: Vec<u64> = render_data
                .iter()
                .map(|&v| 100_u64.saturating_sub(v))
                .collect();

            let bottom_sparkline = Sparkline::default()
                .data(&inverted_data)
                .max(100)
                .style(Style::default().bg(mirror_bg).fg(Color::Rgb(0, 0, 0)));

            frame.render_widget(bottom_sparkline, bottom_area);

            let footer_area = Rect {
                x: padded_area.x,
                y: padded_area.y + padded_area.height.saturating_sub(footer_height),
                width: padded_area.width,
                height: footer_height,
            };

            // When paused, show zero for the peak meter
            let display_peak = if is_paused { 0u8 } else { peak_hold };

            // The whole footer lights up red when the signal peaks
            let footer_style = if peak_alert {
                Style::default()
                    .bg(Color::Red)
                    .fg(Color::Rgb(255, 255, 255))
            } else {
                Style::default().fg(footer_fg).bg(Color::Rgb(0, 0, 0))
            };

            let duration_secs = recording_duration.as_secs();
            let minutes = duration_secs / 60;
            let secs = duration_secs % 60;

            let mut spans = Vec::new();
            if is_paused {
                spans.push(ratatui::text::Span::styled(
                    "⏸ ",
                    Style::default().fg(Color::Yellow),
                ));
            }
            spans.push(ratatui::text::Span::raw(format!(
                "{minutes}:{secs:02} / {display_peak}%"
            )));

            let footer = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(spans))
                .style(footer_style);

            frame.render_widget(footer, footer_area);
        })?;

        Ok(())
    }

    /// Calculates current volume in percentage and updates peak hold tracking.
    ///
    /// Converts RMS (Root Mean Square) audio samples to dBFS and normalizes to 0-100% scale
    /// based on the configured reference level. Also tracks the maximum volume seen in the
    /// last 3 seconds for the peak indicator.
    fn calculate_volume(&mut self, samples: &[i16], raw_true_peak: u16) -> u8 {
        if samples.is_empty() {
            return 0;
        }

        let last_samples_count =
            std::cmp::min(self.sample_rate / 20, samples.len() as u32) as usize;
        let recent_samples = &samples[samples.len() - last_samples_count..];

        let sum_of_squares: i64 = recent_samples.iter().map(|&x| (x as i64).pow(2)).sum();
        let mean_square = sum_of_squares / recent_samples.len() as i64;
        let rms = (mean_square as f32).sqrt();

        let db_fs = if rms > 0.0 {
            20.0 * (rms / 32767.0).log10()
        } else {
            -160.0
        };

        // Absolute clip indicator: use the raw, un-downmixed true peak supplied
        // by the recorder. A mono mic on one channel of a multi-channel device
        // would otherwise be attenuated by the mono averaging and never clip.
        if !self.is_paused && raw_true_peak > 0 {
            let true_peak_db = 20.0 * (f32::from(raw_true_peak) / 32767.0).log10();
            if true_peak_db >= CLIP_PEAK_DB {
                self.last_clip_time = Some(std::time::Instant::now());
            }
        }

        // Auto mode: adapt the reference toward the observed speech level —
        // fast attack when the signal exceeds it, slow release while signal
        // is present but quieter, hold during silence
        if self.reference_level == ReferenceLevel::Auto && !self.is_paused {
            if db_fs > self.adaptive_ref_db {
                self.adaptive_ref_db += (db_fs - self.adaptive_ref_db) * 0.5;
            } else if db_fs > self.adaptive_ref_db - 25.0 {
                self.adaptive_ref_db -= ADAPTIVE_REF_RELEASE_PER_FRAME;
            }
            self.adaptive_ref_db = self
                .adaptive_ref_db
                .clamp(ADAPTIVE_REF_MIN_DB, ADAPTIVE_REF_MAX_DB);
        }

        let min_db = self.effective_reference_db() - 40.0;
        let normalized = ((db_fs - min_db) / 40.0 * 100.0).clamp(4.0, 100.0) as u8;

        if normalized > self.peak_hold || self.peak_hold_time.elapsed().as_secs() >= 3 {
            self.peak_hold = normalized;
            self.peak_hold_time = std::time::Instant::now();
        }

        normalized
    }

    /// Processes user input and returns the appropriate recording command.
    ///
    /// Only responds to Enter (transcribe), Escape, and 'q' (cancel) keys.
    /// All other keys are ignored.
    ///
    /// # Returns
    /// - `Continue` if no key or unrecognized key was pressed
    /// - `Transcribe` if Enter was pressed
    /// - `Cancel` if Escape or 'q' was pressed
    ///
    /// # Errors
    /// - If event polling fails
    pub fn handle_input(&mut self) -> Result<RecordingCommand, Box<dyn Error>> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                return Ok(match key.code {
                    KeyCode::Enter => {
                        tracing::debug!("Enter pressed: proceeding to transcription");
                        RecordingCommand::Transcribe
                    }
                    _ if is_cancel_key(&key) => {
                        tracing::debug!("Cancel key pressed: canceling recording");
                        RecordingCommand::Cancel
                    }
                    KeyCode::Char(' ') => {
                        tracing::debug!("Space pressed: toggling pause");
                        self.toggle_pause_state();
                        RecordingCommand::TogglePause
                    }
                    _ => RecordingCommand::Continue,
                });
            }
        }
        Ok(RecordingCommand::Continue)
    }

    /// Handles pause state transitions, managing pause duration tracking.
    fn toggle_pause_state(&mut self) {
        if self.is_paused {
            // Resuming from pause
            if let Some(pause_start) = self.pause_start_time {
                self.pause_duration += pause_start.elapsed();
                self.pause_start_time = None;
            }
            self.is_paused = false;
        } else {
            // Starting pause
            self.pause_start_time = Some(std::time::Instant::now());
            self.is_paused = true;
        }
    }

    /// Gets the elapsed recording time, excluding paused duration.
    fn get_recording_duration(&self) -> std::time::Duration {
        let total_elapsed = self.recording_start_time.elapsed();
        let mut pause_time = self.pause_duration;

        // If currently paused, add the current pause duration
        if self.is_paused {
            if let Some(pause_start) = self.pause_start_time {
                pause_time += pause_start.elapsed();
            }
        }

        total_elapsed.saturating_sub(pause_time)
    }

    /// Renders one frame of the transcription animation.
    ///
    /// # Errors
    /// - If terminal rendering fails
    pub fn render_transcription_animation(
        &mut self,
        animation: &mut TranscriptionAnimation,
    ) -> Result<(), Box<dyn Error>> {
        self.terminal.draw(|f| {
            let main_area = f.area();
            animation.draw(f, main_area);
        })?;
        animation.update();
        Ok(())
    }

    /// Renders one frame of the action picker and polls for input.
    ///
    /// Returns `Ok(Some(PickerResult))` if the user made a selection or cancelled,
    /// `Ok(None)` if the event loop should continue (no actionable input).
    ///
    /// # Errors
    /// - If terminal rendering fails
    /// - If event polling fails
    pub fn render_action_picker(
        &mut self,
        actions: &[ProcessAction],
        list_state: &mut ListState,
    ) -> Result<Option<PickerResult>, Box<dyn Error>> {
        let mut list_area = Rect::default();
        self.terminal.draw(|frame| {
            let area = frame.area();
            list_area = render_process_view(frame, area, actions, list_state, None);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            return Ok(handle_picker_event(
                event::read()?,
                actions,
                list_state,
                None,
                list_area,
            ));
        }

        Ok(None)
    }

    /// Displays an error in the active recording UI until the user presses a key.
    pub fn show_error(&mut self, title: &str, message: &str) -> Result<(), Box<dyn Error>> {
        loop {
            self.terminal.draw(|frame| {
                let area = frame.area();
                let background = ratatui::widgets::Block::default().style(Style::reset());
                frame.render_widget(background, area);

                let horizontal_padding = area.width / 10;
                let content_width = area.width.saturating_sub(horizontal_padding * 2).max(1);
                let message_lines = error_message_lines(message);
                let wrapped_message_height = message_lines
                    .iter()
                    .map(|line| wrapped_line_count(line, content_width))
                    .sum::<u16>();
                let content_height = wrapped_message_height.saturating_add(4).min(area.height);
                let padded_area = Rect {
                    x: area.x.saturating_add(horizontal_padding),
                    y: area
                        .y
                        .saturating_add(area.height.saturating_sub(content_height) / 2),
                    width: content_width,
                    height: content_height,
                };

                let title_line = ratatui::text::Line::from(ratatui::text::Span::styled(
                    format!(" {title} "),
                    Style::default().fg(Color::Black).bg(Color::Red),
                ))
                .alignment(Alignment::Center);

                let mut lines = Vec::with_capacity(message_lines.len() + 4);
                lines.push(title_line);
                lines.push(ratatui::text::Line::raw(""));
                lines.extend(message_lines.into_iter().map(ratatui::text::Line::raw));
                lines.push(ratatui::text::Line::raw(""));
                lines.push(ratatui::text::Line::raw("Press any key to close."));
                let text = ratatui::text::Text::from(lines);

                let paragraph = Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .style(Style::reset().fg(Color::White));

                frame.render_widget(paragraph, padded_area);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(_) = event::read()? {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Cleans up terminal state and exits alternate screen mode.
    ///
    /// # Errors
    /// - If terminal mode cannot be disabled
    /// - If cursor cannot be shown
    pub fn cleanup(&mut self) -> Result<(), Box<dyn Error>> {
        if self.cleaned_up {
            return Ok(());
        }

        self.cleaned_up = true;
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

fn error_message_lines(message: &str) -> Vec<String> {
    let lines = message.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines.into_iter().map(ToString::to_string).collect()
    }
}

fn wrapped_line_count(line: &str, width: u16) -> u16 {
    if line.is_empty() {
        return 1;
    }

    let width = usize::from(width.max(1));
    line.chars().count().div_ceil(width) as u16
}

impl Drop for RecordingTui {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
