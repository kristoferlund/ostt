use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_title(frame: &mut Frame<'_>, area: Rect, title: &str) {
    render_styled_title(
        frame,
        area,
        title,
        Style::default().fg(Color::Black).bg(Color::Cyan),
    );
}

pub(crate) fn render_themed_title(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    theme: &ratcn::Theme,
) {
    render_styled_title(
        frame,
        area,
        title,
        Style::default()
            .fg(theme.primary_foreground)
            .bg(theme.primary),
    );
}

fn render_styled_title(frame: &mut Frame<'_>, area: Rect, title: &str, style: Style) {
    let label = format!(" {title} ");
    frame.render_widget(
        Paragraph::new(label.clone()).style(style),
        Rect {
            width: label.len() as u16,
            height: 1,
            ..area
        },
    );
}
