use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Frame;

const LOGO: &str = "┏┓┏╋╋ \n┗┛┛┗┗ \n";

pub struct AppLayout {
    pub title: Rect,
    pub body: Rect,
    pub footer: Rect,
    pub full: Rect,
}

pub fn render_app_layout(frame: &mut Frame<'_>, area: Rect) -> AppLayout {
    let padding_block = Block::default().padding(Padding::new(1, 1, 1, 0));
    frame.render_widget(&padding_block, area);
    let padded_area = padding_block.inner(area);

    let main_block = Block::default();
    frame.render_widget(&main_block, padded_area);
    let inner_area = main_block.inner(padded_area);

    let [header_area, title, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(inner_area);

    frame.render_widget(Paragraph::new(LOGO).alignment(Alignment::Left), header_area);

    AppLayout {
        title,
        body,
        footer,
        full: area,
    }
}
