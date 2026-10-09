use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_footer(frame: &mut Frame<'_>, area: Rect, text: &'static str) {
    render_styled_footer(
        frame,
        area,
        text,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    );
}

pub(crate) fn render_themed_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    text: &str,
    theme: &ratcn::Theme,
) {
    render_styled_footer(
        frame,
        area,
        text,
        Style::default()
            .fg(theme.muted_foreground)
            .bg(theme.surface),
    );
}

fn render_styled_footer(frame: &mut Frame<'_>, area: Rect, text: &str, style: Style) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style),
        area,
    );
}
