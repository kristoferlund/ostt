//! Terminal user interface for audio recording with waveform visualization.
//!
//! Provides real-time volume display, recording duration tracking, and user input handling
//! for the recording workflow.

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::Sparkline,
};
use std::error::Error;
use std::io::{stdout, Stdout};

use crate::transcription::TranscriptionAnimation;

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

/// Terminal UI for audio recording with waveform visualization.
///
/// Displays real-time volume levels, recording duration, and animated transcription progress.
pub struct OsttTui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    volume_history: Vec<u64>,
    last_sample_time: std::time::Instant,
    sample_interval: std::time::Duration,
    last_peak: u8,
    terminal_width: usize,
    sample_rate: u32,
    recording_start_time: std::time::Instant,
    peak_hold: u8,
    peak_hold_time: std::time::Instant,
    peak_volume_threshold: u8,
    reference_level_db: i8,
    /// Whether recording is currently paused
    pub is_paused: bool,
    /// Total time paused (accumulated when paused)
    pause_duration: std::time::Duration,
    /// When pause started (for calculating pause duration)
    pause_start_time: Option<std::time::Instant>,
}

impl OsttTui {
    /// Creates a new TUI instance and enters alternate screen mode.
    ///
    /// # Errors
    /// - If terminal cannot be initialized
    /// - If raw mode cannot be enabled
    /// - If alternate screen cannot be entered
    pub fn new(
        sample_rate: u32,
        peak_volume_threshold: u8,
        reference_level_db: i8,
    ) -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let size = terminal.size()?;
        let terminal_width = size.width as usize;

        let sample_interval = std::time::Duration::from_millis(50);

        let volume_history = vec![0u64; terminal_width];

        let now = std::time::Instant::now();
        Ok(OsttTui {
            terminal,
            volume_history,
            last_sample_time: now,
            sample_interval,
            last_peak: 0,
            terminal_width,
            sample_rate,
            recording_start_time: now,
            peak_hold: 0,
            peak_hold_time: now,
            peak_volume_threshold,
            reference_level_db,
            is_paused: false,
            pause_duration: std::time::Duration::ZERO,
            pause_start_time: None,
        })
    }

    /// Renders the waveform visualization with current volume and recording duration.
    ///
    /// # Errors
    /// - If terminal rendering fails
    pub fn render_waveform(&mut self, samples: &[i16]) -> Result<(), Box<dyn Error>> {
        let current_volume = self.calculate_volume(samples);

        // Only update waveform if not paused
        if !self.is_paused && self.last_sample_time.elapsed() >= self.sample_interval {
            self.volume_history.push(current_volume as u64);

            if self.volume_history.len() > self.terminal_width {
                self.volume_history.remove(0);
            }

            self.last_sample_time = std::time::Instant::now();
        }

        let size = self.terminal.size()?;
        let current_width = size.width as usize;

        if current_width != self.terminal_width {
            self.terminal_width = current_width;
            if self.volume_history.len() > self.terminal_width {
                while self.volume_history.len() > self.terminal_width {
                    self.volume_history.remove(0);
                }
            } else if self.volume_history.len() < self.terminal_width {
                while self.volume_history.len() < self.terminal_width {
                    self.volume_history.insert(0, 0);
                }
            }
        }

        // Calculate these values before the draw closure to avoid borrow issues
        let is_paused = self.is_paused;
        let peak_hold = self.peak_hold;
        let last_peak = self.last_peak;
        let peak_volume_threshold = self.peak_volume_threshold;
        let recording_duration = self.get_recording_duration();

        self.terminal.draw(|frame| {
            let area = frame.area();

            let footer_height = 1;

            let content_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height.saturating_sub(footer_height),
            };

            let top_area_height = content_area.height / 3 * 2;

            let top_area = Rect {
                x: content_area.x,
                y: content_area.y,
                width: content_area.width,
                height: top_area_height,
            };

            let top_sparkline = Sparkline::default()
                .data(&self.volume_history)
                .max(80)
                .style(
                    Style::default()
                        .bg(Color::Rgb(0, 0, 0))
                        .fg(Color::Rgb(206, 224, 220)),
                );

            frame.render_widget(top_sparkline, top_area);

            let bottom_area = Rect {
                x: content_area.x,
                y: content_area.y + top_area_height,
                width: content_area.width,
                height: content_area.height.saturating_sub(top_area_height),
            };

            let inverted_data: Vec<u64> = self
                .volume_history
                .iter()
                .map(|&v| 100_u64.saturating_sub(v))
                .collect();

            let bottom_sparkline = Sparkline::default().data(&inverted_data).max(80).style(
                Style::default()
                    .bg(Color::Rgb(185, 207, 212))
                    .fg(Color::Rgb(0, 0, 0)),
            );

            frame.render_widget(bottom_sparkline, bottom_area);

            let footer_area = Rect {
                x: area.x,
                y: area.y + area.height.saturating_sub(footer_height),
                width: area.width,
                height: footer_height,
            };

            // When paused, show zeros for meters
            let (display_peak, display_volume) = if is_paused {
                (0u8, 0u8)
            } else {
                (peak_hold, last_peak)
            };

            let peak_style = if display_peak >= peak_volume_threshold {
                Style::default()
                    .bg(Color::Red)
                    .fg(Color::Rgb(255, 255, 255))
            } else {
                Style::default()
            };

            let duration_secs = recording_duration.as_secs();
            let minutes = duration_secs / 60;
            let secs = duration_secs % 60;
            let duration_span = ratatui::text::Span::raw(format!("{minutes}:{secs:02}"));

            let peak_span = ratatui::text::Span::styled(format!("{display_peak}%"), peak_style);

            let vol_span = ratatui::text::Span::raw(format!("{display_volume}%"));

            // Show pause symbol instead of red dot when paused
            let indicator = if is_paused {
                ratatui::text::Span::styled("⏸ ", Style::default().fg(Color::Yellow))
            } else {
                ratatui::text::Span::styled("● ", Style::default().fg(Color::Red))
            };

            let help_text = ratatui::text::Line::from(vec![
                indicator,
                duration_span,
                ratatui::text::Span::raw(" / "),
                vol_span,
                ratatui::text::Span::raw(" / "),
                peak_span,
            ]);

            let footer = ratatui::widgets::Paragraph::new(help_text).style(
                Style::default()
                    .fg(Color::Rgb(185, 207, 212))
                    .bg(Color::Rgb(0, 0, 0)),
            );

            frame.render_widget(footer, footer_area);
        })?;

        Ok(())
    }

    /// Calculates current volume in percentage and updates peak hold tracking.
    ///
    /// Converts RMS (Root Mean Square) audio samples to dBFS and normalizes to 0-100% scale
    /// based on the configured reference level. Also tracks the maximum volume seen in the
    /// last 3 seconds for the peak indicator.
    fn calculate_volume(&mut self, samples: &[i16]) -> u8 {
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

        let min_db = self.reference_level_db as f32 - 40.0;
        let normalized = ((db_fs - min_db) / 40.0 * 100.0).clamp(4.0, 100.0) as u8;

        self.last_peak = normalized;

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
                    KeyCode::Char('q') | KeyCode::Esc => {
                        tracing::debug!("Escape or 'q' pressed: canceling recording");
                        RecordingCommand::Cancel
                    }
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        tracing::debug!("Ctrl+C pressed: canceling recording");
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

    /// Cleans up terminal state and exits alternate screen mode.
    ///
    /// # Errors
    /// - If terminal mode cannot be disabled
    /// - If cursor cannot be shown
    pub fn cleanup(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
