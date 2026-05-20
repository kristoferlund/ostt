use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::Input;

use crate::ui::{dialog_content_area, render_dialog_content, DialogAction};

use super::local_model_view_helpers::{padded_line, wizard_button};

pub(super) struct CustomModelUrlInputDialog;

impl CustomModelUrlInputDialog {
    pub(super) fn render(
        frame: &mut Frame<'_>,
        input: &Input,
        selected_action: DialogAction,
    ) {
        let lines = vec![
            padded_line("Paste a Hugging Face model page or a direct model file URL."),
            padded_line("Supported files: .gguf and ggml-*.bin."),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            wizard_button("Next", selected_action),
        ];
        render_dialog_content(frame, "Download Custom Model 1/3", lines, 70, 10);
        render_dialog_input(frame, input, 10, 5);
        set_custom_input_cursor(frame, input, 10, 5);
    }
}

pub(super) fn set_custom_input_cursor(
    frame: &mut Frame<'_>,
    input: &Input,
    dialog_height: u16,
    line: u16,
) {
    let inner_area = dialog_content_area(70, dialog_height, frame.area());
    let cursor_x = inner_area
        .x
        .saturating_add(1)
        .saturating_add(input.cursor() as u16)
        .min(inner_area.x.saturating_add(inner_area.width.saturating_sub(1)));
    frame.set_cursor_position(Position::new(cursor_x, inner_area.y.saturating_add(line)));
}

pub(super) fn render_dialog_input(
    frame: &mut Frame<'_>,
    input: &Input,
    dialog_height: u16,
    line: u16,
) {
    let inner_area = dialog_content_area(70, dialog_height, frame.area());
    let area = Rect {
        x: inner_area.x.saturating_add(1),
        y: inner_area.y.saturating_add(line),
        width: inner_area.width.saturating_sub(2),
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(input.value().to_string())
            .style(Style::default().fg(Color::DarkGray).bg(Color::Gray)),
        area,
    );
}
