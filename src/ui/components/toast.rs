use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};

use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::time::{Duration, Instant};

const TOAST_DURATION: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
pub struct Toast {
    message: String,
    style: ToastStyle,
    created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>) -> Self {
        Self::success(message)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            style: ToastStyle::success(),
            created_at: Instant::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            style: ToastStyle::error(),
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= TOAST_DURATION
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn style(&self) -> ToastStyle {
        self.style
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ToastStyle {
    pub fg: Color,
    pub bg: Color,
}

impl ToastStyle {
    pub const fn default() -> Self {
        Self::success()
    }

    pub const fn success() -> Self {
        Self {
            fg: Color::Black,
            bg: Color::Green,
        }
    }

    pub const fn error() -> Self {
        Self {
            fg: Color::Black,
            bg: Color::Red,
        }
    }
}

pub fn render_toast(frame: &mut Frame<'_>, toast: &Toast) {
    let style = toast.style();
    let screen = frame.area();
    let width = (toast.message().len() as u16 + 4)
        .clamp(20, 50)
        .min(screen.width);
    let height = 3.min(screen.height);
    let area = Rect {
        x: screen.x + screen.width.saturating_sub(width + 2),
        y: screen.y + screen.height.saturating_sub(height + 2),
        width,
        height,
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(toast.message().to_string())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(style.fg).bg(style.bg)),
            )
            .style(Style::default().fg(style.fg).bg(style.bg))
            .alignment(Alignment::Center),
        area,
    );
}
