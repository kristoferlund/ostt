//! Animated logo loader for transcription phase.
//!
//! Features a simple character-by-character animation where ASCII art characters slide in from the right,
//! assemble in the center, then slide out to the left, and the cycle repeats.

use ratatui::prelude::*;
use std::time::Instant;

/// Represents a single ASCII art character in the animation (2 lines)
#[derive(Clone, Debug)]
struct AsciiArtChar {
    x: f32,
    top: &'static str,    // Top line of ASCII art
    bottom: &'static str, // Bottom line of ASCII art
    width: usize,         // Width of the character
}

/// The animated logo loader
pub struct TranscriptionAnimation {
    chars: Vec<AsciiArtChar>,
    phase: f32, // 0.0 to 1.0, cycles through animation phases
    start_time: Instant,
    min_duration: std::time::Duration,
    frame_count: u32,
    status_label: String,
}

impl TranscriptionAnimation {
    /// Creates a new logo loader
    pub fn new(_terminal_width: usize) -> Self {
        Self {
            chars: Vec::new(),
            phase: 0.0,
            start_time: Instant::now(),
            min_duration: std::time::Duration::from_secs(5),
            frame_count: 0,
            status_label: "Transcribing...".to_string(),
        }
    }

    /// Sets the status label displayed below the animation.
    pub fn set_status_label(&mut self, label: &str) {
        self.status_label = label.to_string();
    }

    /// Initialize ASCII art characters for the animation
    fn init_chars(&mut self) {
        self.chars.clear();

        // Define each ASCII art character (top line, bottom line, width)
        // The logo "ostt" is:
        //  ┏┓┏╋╋
        //  ┗┛┛┗┗
        let ascii_chars = [
            AsciiArtChar {
                x: 0.0,
                top: "┏┓",
                bottom: "┗┛",
                width: 2,
            }, // o
            AsciiArtChar {
                x: 0.0,
                top: "┏",
                bottom: "┛",
                width: 1,
            }, // s
            AsciiArtChar {
                x: 0.0,
                top: "╋",
                bottom: "┗",
                width: 1,
            }, // t (first t)
            AsciiArtChar {
                x: 0.0,
                top: "╋",
                bottom: "┗",
                width: 1,
            }, // t (second t)
        ];

        self.chars = ascii_chars.to_vec();
    }

    /// Returns true if the animation minimum duration has not elapsed.
    pub fn is_running(&self) -> bool {
        self.start_time.elapsed() < self.min_duration
    }

    /// Returns elapsed time since animation start in seconds.
    pub fn elapsed_secs(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    /// Advances the animation to the next frame.
    pub fn update(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
    }

    /// Update character positions: slide in from right → assemble → slide out to left cycle
    fn update_chars(&mut self, width: u16, _height: u16) {
        if self.chars.is_empty() {
            self.init_chars();
        }

        // Update phase (0.0 to 1.0, cycles every frame)
        // Faster cycle: 0.015 instead of 0.0083 (~66 frames = ~1.1 seconds per cycle)
        self.phase = (self.frame_count as f32 * 0.03) % 1.0;

        // Calculate total width of the logo: 2+1+1+1 = 5
        let total_logo_width: i32 = self.chars.iter().map(|c| c.width as i32).sum();
        let center_x = (width as i32 / 2) - (total_logo_width / 2);

        // Phase breakdown with 500ms pause:
        // 0.0 - 0.35: slide in (characters appear one by one)
        // 0.35 - 0.60: pause (hold at center for 500ms)
        // 0.60 - 0.95: slide out (characters leave one by one)
        // 0.95 - 1.0: transition gap
        let slide_in_end = 0.35;
        let pause_end = 0.60;
        let slide_out_end = 0.95;

        let total_chars = self.chars.len() as f32;
        let phase_per_char_in = slide_in_end / total_chars;
        let phase_per_char_out = (slide_out_end - pause_end) / total_chars;

        // Calculate target x position for each character (using integers to avoid rounding issues)
        let mut current_target_x = center_x;

        for (char_idx, anim_char) in self.chars.iter_mut().enumerate() {
            let char_idx_f = char_idx as f32;

            // Sliding in phase: each character comes in at its own time
            let slide_in_start = char_idx_f * phase_per_char_in;
            let char_slide_in_end = slide_in_start + phase_per_char_in;

            // Sliding out phase: each character leaves at its own time (after pause)
            let slide_out_start = pause_end + char_idx_f * phase_per_char_out;
            let char_slide_out_end = slide_out_start + phase_per_char_out;

            if self.phase >= slide_in_start && self.phase < char_slide_in_end {
                // Sliding in from the right
                let progress = (self.phase - slide_in_start) / phase_per_char_in;
                let start_x = width as i32 + 5;
                let target_x = current_target_x;
                anim_char.x = start_x as f32 - (start_x - target_x) as f32 * progress;
            } else if self.phase >= slide_out_start && self.phase < char_slide_out_end {
                // Sliding out to the left
                let progress = (self.phase - slide_out_start) / phase_per_char_out;
                let target_x = current_target_x;
                let end_x = -5;
                anim_char.x = target_x as f32 - (target_x - end_x) as f32 * progress;
            } else if self.phase < slide_in_start {
                // Haven't appeared yet, start off screen to the right
                anim_char.x = (width as i32 + 5) as f32;
            } else if self.phase >= char_slide_out_end {
                // Already gone off screen to the left
                anim_char.x = -5.0;
            } else {
                // Between animations (paused or waiting) - snap to exact target position
                anim_char.x = current_target_x as f32;
            }

            current_target_x += anim_char.width as i32;
        }
    }

    /// Renders the animation
    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let width = area.width;
        let height = area.height;

        // Update characters before drawing
        self.update_chars(width, height);

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                frame
                    .buffer_mut()
                    .set_string(x, y, " ", Style::default().bg(Color::Rgb(0, 0, 0)));
            }
        }

        // Render ASCII art characters (2 lines each)
        let center_y = height / 2;
        let color = Color::Rgb(255, 255, 255);

        for anim_char in &self.chars {
            // Use the x position directly (already calculated in update_chars)
            let x = anim_char.x.round() as i32;
            let char_width = anim_char.width as i32;

            // Top line
            let y_top = (center_y as i32) - 1;
            if x >= 0 && x + char_width <= width as i32 && y_top >= 0 && y_top < height as i32 {
                frame.buffer_mut().set_string(
                    area.x + x as u16,
                    area.y + y_top as u16,
                    anim_char.top,
                    Style::default().fg(color).bold(),
                );
            }

            // Bottom line
            let y_bottom = center_y as i32;
            if x >= 0 && x + char_width <= width as i32 && y_bottom >= 0 && y_bottom < height as i32
            {
                frame.buffer_mut().set_string(
                    area.x + x as u16,
                    area.y + y_bottom as u16,
                    anim_char.bottom,
                    Style::default().fg(color).bold(),
                );
            }
        }

        // Render status label centered below the logo
        let label_x = (width / 2).saturating_sub(self.status_label.len() as u16 / 2);
        let label_y = center_y + 2;
        if !self.status_label.is_empty() && label_y < height {
            frame.buffer_mut().set_string(
                area.x + label_x,
                area.y + label_y,
                &self.status_label,
                Style::default().fg(Color::Rgb(128, 128, 128)),
            );
        }
    }
}
