use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub(crate) fn render_footer(frame: &mut Frame<'_>, area: Rect, text: &str, theme: &ratcn::Theme) {
    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Center).style(
            Style::default()
                .fg(theme.secondary_foreground)
                .bg(theme.secondary),
        ),
        area,
    );
}
