use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Frame;

const LOGO: &str = "┏┓┏╋╋ \n┗┛┛┗┗ \n";

pub struct AppLayout {
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

    let [header_area, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(inner_area);

    frame.render_widget(Paragraph::new(LOGO).alignment(Alignment::Left), header_area);

    AppLayout {
        body,
        footer,
        full: area,
    }
}

pub fn render_footer(frame: &mut Frame<'_>, area: Rect, text: &'static str) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        area,
    );
}

pub fn render_title(frame: &mut Frame<'_>, area: Rect, title: &str) -> Rect {
    let [title_area, content_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let label = format!(" {title} ");
    frame.render_widget(
        Paragraph::new(label.clone()).style(Style::default().fg(Color::White).bg(Color::Blue)),
        Rect {
            width: label.len() as u16,
            height: 1,
            ..title_area
        },
    );
    content_area
}
