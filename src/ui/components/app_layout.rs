use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Frame;

const LOGO: &str = "┏┓┏╋╋ \n┗┛┛┗┗ \n";

pub struct AppLayout {
    pub header: Rect,
    pub title: Rect,
    pub body: Rect,
    pub footer: Rect,
}

fn app_layout(area: Rect) -> AppLayout {
    let inner = Block::default()
        .padding(Padding::new(0, 0, 1, 0))
        .inner(area);
    let [header, title, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(inner);
    AppLayout {
        header,
        title,
        body,
        footer,
    }
}

pub fn render_app_layout(frame: &mut Frame<'_>, area: Rect) -> AppLayout {
    let layout = app_layout(area);
    frame.render_widget(
        Paragraph::new(LOGO).alignment(Alignment::Left),
        layout.header,
    );
    layout
}
