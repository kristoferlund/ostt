use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_title(frame: &mut Frame<'_>, area: Rect, title: &str) {
    let label = format!(" {title} ");
    frame.render_widget(
        Paragraph::new(label.clone()).style(Style::default().fg(Color::White).bg(Color::Blue)),
        Rect {
            width: label.len() as u16,
            height: 1,
            ..area
        },
    );
}
