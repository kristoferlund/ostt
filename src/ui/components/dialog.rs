use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

const BG: Color = Color::DarkGray;
const FG: Color = Color::White;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogAction {
    Ok,
    Cancel,
}

pub fn render_dialog(
    frame: &mut Frame<'_>,
    title: &'static str,
    mut lines: Vec<Line<'static>>,
    primary_action: &'static str,
) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("<{primary_action}>"),
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    render_dialog_content(frame, title, lines, 70, 9);
}

pub fn render_error_dialog(frame: &mut Frame<'_>, title: &'static str, message: String) {
    let lines = vec![
        Line::from(message),
        Line::from(""),
        Line::from(Span::styled(
            "<Close>",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    render_dialog_content(frame, title, lines, 70, 8);
}

pub fn centered_fixed_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

pub fn dialog_content_area(width: u16, height: u16, area: Rect) -> Rect {
    let area = centered_fixed_rect(width, height, area);
    Rect {
        x: area.x.saturating_add(2),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    }
}

pub fn render_dialog_content(
    frame: &mut Frame<'_>,
    title: &str,
    lines: Vec<Line<'static>>,
    width: u16,
    height: u16,
) {
    let area = centered_fixed_rect(width, height, frame.area());
    render_box(frame, area, title, lines);
}

fn render_box(frame: &mut Frame<'_>, area: Rect, title: &str, lines: Vec<Line<'static>>) {
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let inner_area = dialog_content_area(area.width, area.height, area);

    let title_width = title.len() as u16;
    let escape = "esc";
    let escape_width = escape.len() as u16;
    let spacer_width = inner_area
        .width
        .saturating_sub(title_width.saturating_add(escape_width));
    let title_line = Line::from(vec![
        Span::styled(
            title.to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ),
        Span::raw(" ".repeat(spacer_width as usize)),
        Span::styled(escape, Style::default().fg(Color::Gray)),
    ]);

    let mut padded_lines = Vec::with_capacity(lines.len() + 2);
    padded_lines.push(title_line);
    padded_lines.push(Line::from(""));
    padded_lines.extend(lines);

    frame.render_widget(
        Paragraph::new(padded_lines)
            .style(Style::default().fg(FG).bg(BG))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        inner_area,
    );
}
