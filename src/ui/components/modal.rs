//! Dialog declarations; focus, values, and modal lifetime belong to the command.

use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratcn::{Button, Dialog, Input};

pub(crate) fn dialog<S: 'static, M: 'static>(
    title: impl Into<String>,
    description: impl Into<String>,
    action: Button<M>,
) -> Dialog<S, M> {
    Dialog::new()
        .title(title)
        .description(description)
        .action("accept", action)
}

pub(crate) fn form_dialog<S: 'static, M: 'static>(
    dialog: Dialog<S, M>,
    instructions: &'static str,
    inputs: Vec<Input<S, M>>,
) -> Dialog<S, M> {
    let instructions_height = if instructions.is_empty() { 0 } else { 3 };
    dialog.content(instructions_height + inputs.len() as u16 * 3, move |ctx| {
        let mut constraints = vec![Constraint::Length(instructions_height)];
        constraints.extend(inputs.iter().map(|_| Constraint::Length(3)));
        let areas = Layout::vertical(constraints).split(ctx.area());
        ctx.paint_widget(Paragraph::new(instructions), areas[0]);
        for (index, input) in inputs.into_iter().enumerate() {
            ctx.component(format!("input-{index}"), input, areas[index + 1]);
        }
    })
}
