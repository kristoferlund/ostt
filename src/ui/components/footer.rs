use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_footer(frame: &mut Frame<'_>, area: Rect, text: &'static str) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        area,
    );
}
