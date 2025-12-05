//! Generic error screen for displaying human-readable error messages.
//!
//! Provides a full-screen error display with centered text and user-friendly formatting.

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::Paragraph,
};
use std::io::{self, Stdout};

/// Error screen for displaying human-readable error messages.
///
/// Features:
/// - Full screen red background
/// - Centered white text (both horizontally and vertically)
/// - Displays error message in a readable format
pub struct ErrorScreen {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl ErrorScreen {
    /// Creates a new error screen and enters alternate screen mode.
    ///
    /// # Errors
    /// - If terminal cannot be initialized
    /// - If raw mode cannot be enabled
    /// - If alternate screen cannot be entered
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(ErrorScreen { terminal })
    }

    /// Displays an error message on a full red screen with centered white text.
    ///
    /// Waits for any key press to dismiss the error. Message wraps to 80% of screen width.
    ///
    /// # Errors
    /// - If terminal rendering fails
    pub fn show_error(&mut self, error_message: &str) -> anyhow::Result<()> {
        loop {
            self.terminal.draw(|frame| {
                let area = frame.area();

                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        frame.buffer_mut().set_string(
                            x,
                            y,
                            " ",
                            Style::default().bg(Color::Rgb(255, 0, 0)),
                        );
                    }
                }

                let padding_x = area.width / 10;
                let text_width = (area.width * 80) / 100;

                let error_text = ratatui::text::Line::from(ratatui::text::Span::styled(
                    error_message,
                    Style::default()
                        .fg(Color::Rgb(255, 255, 255))
                        .bg(Color::Rgb(255, 0, 0)),
                ));

                let paragraph = Paragraph::new(error_text)
                    .alignment(Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: true });

                let centered_area = Rect {
                    x: area.x + padding_x,
                    y: area.y + area.height / 2,
                    width: text_width,
                    height: area.height / 2,
                };

                frame.render_widget(paragraph, centered_area);
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
    pub fn cleanup(&mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for ErrorScreen {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
