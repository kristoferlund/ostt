use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub(crate) fn render_title(frame: &mut Frame<'_>, area: Rect, title: &str, theme: &ratcn::Theme) {
    let label = format!(" {title} ");
    let width = label.len() as u16;
    frame.render_widget(
        Paragraph::new(label).style(
            Style::default()
                .fg(theme.primary_foreground)
                .bg(theme.primary),
        ),
        Rect {
            width,
            height: 1,
            ..area
        },
    );
}
