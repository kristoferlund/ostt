//! Dialog declarations; focus, values, and modal lifetime belong to the command.
//!
//! Every dialog shares one look: the title, the body, and one centered
//! `<Action>` button.

use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratcn::{color::dim, runtime::DeclareCtx, Button, Dialog, DialogStyle, Input};

/// A dialog whose `body_height` rows of body are declared by `body`.
pub(crate) fn dialog<S: 'static, M: 'static>(
    title: impl Into<String>,
    body_height: u16,
    body: impl FnOnce(&mut DeclareCtx<'_, S, M>) + 'static,
    action: &str,
    on_press: impl Fn() -> M + 'static,
) -> Dialog<S, M> {
    let button = Button::new(format!("<{action}>")).on_press(on_press);
    Dialog::new()
        .style(|theme| {
            let style = DialogStyle::from_theme(theme);
            let bg = dim(theme.secondary_foreground, theme.background, 95);
            DialogStyle {
                background: bg,
                border: theme.border,
                ..style
            }
        })
        .title(title)
        .outer_width(70)
        .content(body_height, body)
        .footer(1, move |ctx| {
            let button_area = ctx
                .area()
                .centered_horizontally(Constraint::Length(button.width()));
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
        move |ctx| ctx.paint_widget(Paragraph::new(text).centered(), ctx.area()),
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
        move |ctx| {
            let areas = Layout::vertical(constraints).split(ctx.area());
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
