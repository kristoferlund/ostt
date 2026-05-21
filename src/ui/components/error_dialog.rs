use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use super::dialog::render_dialog_content;

pub fn render_error_dialog(frame: &mut Frame<'_>, title: &'static str, message: String) {
    let lines = vec![
        Line::from(message),
        Line::from(""),
        Line::from(Span::styled(
            "<Close>",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    render_dialog_content(frame, title, lines, 70, 8);
}
