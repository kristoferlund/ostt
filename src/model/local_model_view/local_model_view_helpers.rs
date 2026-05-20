use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::ui::DialogAction;

pub(super) fn padded_line(text: impl Into<String>) -> Line<'static> {
    Line::from(format!(" {}", text.into()))
}

pub(super) fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

pub(super) fn progress_bar(progress: f64, width: usize) -> String {
    let progress = progress.clamp(0.0, 1.0);
    let completed = (progress * width as f64).round() as usize;
    let remaining = width.saturating_sub(completed);
    format!("{}{}", "█".repeat(completed), "░".repeat(remaining))
}

pub(super) fn wizard_button(action: &'static str, _selected_action: DialogAction) -> Line<'static> {
    Line::from(Span::styled(
        format!("<{action}>"),
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
}
