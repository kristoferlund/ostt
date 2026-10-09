//! Dialog declarations; focus, values, and modal lifetime belong to the command.
//!
//! Every dialog has ostt's look: no visible border, an underlined title with an
//! `esc` hint, the body, and one centered `<Action>` button.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::Paragraph,
};
use ratcn::{runtime::DeclareCtx, Button, Dialog, DialogStyle, Input};

/// The title row and the blank row under it.
const HEADER_HEIGHT: u16 = 2;

/// A dialog whose `body_height` rows of body are declared by `body`.
pub(crate) fn dialog<S: 'static, M: 'static>(
    title: impl Into<String>,
    body_height: u16,
    body: impl FnOnce(&mut DeclareCtx<'_, S, M>, Rect) + 'static,
    action: &str,
    on_press: impl Fn() -> M + 'static,
) -> Dialog<S, M> {
    let title = title.into();
    let button = Button::new(format!("<{action}>")).on_press(on_press);
    Dialog::new()
        .style(|theme| {
            let style = DialogStyle::from_theme(theme);
            DialogStyle {
                border: style.background,
                ..style
            }
        })
        .content(HEADER_HEIGHT + body_height, move |ctx| {
            let [header, _, body_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(ctx.area());
            let title_style = Style::default()
                .fg(ctx.theme.foreground)
                .add_modifier(Modifier::UNDERLINED);
            let hint_style = Style::default().fg(ctx.theme.muted_foreground);
            ctx.paint_widget(Paragraph::new(Span::styled(title, title_style)), header);
            ctx.paint_widget(
                Paragraph::new(Span::styled("esc", hint_style)).right_aligned(),
                header,
            );
            body(ctx, body_area);
        })
        .footer(1, move |ctx| {
            let area = ctx.area();
            let button_area = area.centered_horizontally(Constraint::Length(button.width()));
            ctx.component("accept", button, button_area);
        })
}

/// A dialog that shows centered `text`.
pub(crate) fn message_dialog<S: 'static, M: 'static>(
    title: impl Into<String>,
    text: String,
    action: &str,
    on_press: impl Fn() -> M + 'static,
) -> Dialog<S, M> {
    let height = text.lines().count() as u16;
    dialog(
        title,
        height,
        move |ctx, area| ctx.paint_widget(Paragraph::new(text).centered(), area),
        action,
        on_press,
    )
}

/// A dialog with centered `instructions` above borderless fields, each with an
/// optional label line. Inputs are declared as `input-0`, `input-1`, ….
pub(crate) fn form_dialog<S: 'static, M: 'static>(
    title: impl Into<String>,
    instructions: &'static str,
    fields: Vec<(&'static str, Input<S, M>)>,
    action: &str,
    on_press: impl Fn() -> M + 'static,
) -> Dialog<S, M> {
    let instructions_height = match instructions.lines().count() as u16 {
        0 => 0,
        lines => lines + 1,
    };
    // Instructions, then per field: a blank row between fields, the label
    // (if any) and the input.
    let mut rows = vec![instructions_height];
    for (index, (label, _)) in fields.iter().enumerate() {
        if index > 0 {
            rows.push(1);
        }
        rows.extend([u16::from(!label.is_empty()), 1]);
    }
    let height = rows.iter().sum();
    let constraints: Vec<_> = rows.into_iter().map(Constraint::Length).collect();
    dialog(
        title,
        height,
        move |ctx, area| {
            let areas = Layout::vertical(constraints).split(area);
            ctx.paint_widget(Paragraph::new(instructions).centered(), areas[0]);
            for (index, (label, input)) in fields.into_iter().enumerate() {
                let row = 1 + index * 3;
                ctx.paint_widget(Paragraph::new(label), areas[row]);
                ctx.component(format!("input-{index}"), input, areas[row + 1]);
            }
        },
        action,
        on_press,
    )
}
