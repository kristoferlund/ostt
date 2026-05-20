pub(crate) mod cloud;
pub(crate) mod local;
pub(crate) mod provider;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Frame;

pub(crate) const LOGO: &str = "┏┓┏╋╋ \n┗┛┛┗┗ \n";

pub(crate) fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub(crate) fn render_shell(frame: &mut Frame<'_>) -> [Rect; 3] {
    let area = frame.area();

    let padding_block = Block::default().padding(Padding::new(1, 1, 1, 0));
    frame.render_widget(&padding_block, area);
    let padded_area = padding_block.inner(area);

    let main_block = Block::default();
    frame.render_widget(&main_block, padded_area);
    let inner_area = main_block.inner(padded_area);

    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(inner_area);

    frame.render_widget(
        Paragraph::new(LOGO).alignment(Alignment::Left),
        header_area,
    );

    [body_area, footer_area, area]
}

pub(crate) fn render_footer(frame: &mut Frame<'_>, area: Rect, text: &'static str) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        area,
    );
}

pub(crate) fn render_title(frame: &mut Frame<'_>, area: Rect, title: &'static str) -> Rect {
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
