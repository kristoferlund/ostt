use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::time::{Duration, Instant};

const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(255, 255, 255);
const TOAST_DURATION: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogAction {
    Ok,
    Cancel,
}

#[derive(Clone, Debug)]
pub struct Toast {
    message: String,
    created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= TOAST_DURATION
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ToastStyle {
    pub fg: Color,
    pub bg: Color,
}

impl ToastStyle {
    pub const fn default() -> Self {
        Self {
            fg: Color::Black,
            bg: Color::Green,
        }
    }
}

pub fn render_dialog(
    frame: &mut Frame<'_>,
    title: &'static str,
    mut lines: Vec<Line<'static>>,
    primary_action: &'static str,
) {
    let area = centered_fixed_rect(70, 9, frame.area());
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("<{primary_action}>"),
        Style::default().fg(BG).bg(FG).add_modifier(Modifier::BOLD),
    )));
    render_box(frame, area, title, lines);
}

pub fn render_error_dialog(frame: &mut Frame<'_>, title: &'static str, message: String) {
    let area = centered_fixed_rect(70, 8, frame.area());
    let lines = vec![
        Line::from(message),
        Line::from(""),
        Line::from(Span::styled(
            "<Close>",
            Style::default().fg(BG).bg(FG).add_modifier(Modifier::BOLD),
        )),
    ];
    render_box(frame, area, title, lines);
}

pub fn render_toast(frame: &mut Frame<'_>, toast: &Toast, style: ToastStyle) {
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

pub fn centered_fixed_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn render_box(frame: &mut Frame<'_>, area: Rect, title: &'static str, lines: Vec<Line<'static>>) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let inner_area = Rect {
        x: area.x.saturating_add(2),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let title_width = title.len() as u16;
    let escape = "esc";
    let escape_width = escape.len() as u16;
    let spacer_width = inner_area
        .width
        .saturating_sub(title_width.saturating_add(escape_width));
    let title_line = Line::from(vec![
        Span::styled(title, Style::default().add_modifier(Modifier::UNDERLINED)),
        Span::raw(" ".repeat(spacer_width as usize)),
        Span::styled(escape, Style::default().fg(Color::White)),
    ]);

    let mut padded_lines = Vec::with_capacity(lines.len() + 2);
    padded_lines.push(title_line);
    padded_lines.push(Line::from(""));
    padded_lines.extend(lines);

    frame.render_widget(
        Paragraph::new(padded_lines)
            .style(Style::default().bg(Color::Black))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        inner_area,
    );
}
