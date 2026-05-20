use ratatui::text::Line;
use ratatui::Frame;
use tui_input::Input;

use crate::ui::{render_dialog_content, DialogAction};

use super::custom_model_url_input_dialog::{render_dialog_input, set_custom_input_cursor};
use super::local_model_view_helpers::{padded_line, wizard_button};
use super::types::CustomModelDetailsFocus;

pub(super) struct CustomModelDetailsDialog;

impl CustomModelDetailsDialog {
    pub(super) fn render(
        frame: &mut Frame<'_>,
        id_input: &Input,
        name_input: &Input,
        focus: CustomModelDetailsFocus,
        selected_action: DialogAction,
    ) {
        let lines = vec![
            padded_line("Choose how this custom model should appear in OSTT."),
            Line::from(""),
            padded_line("ID:"),
            Line::from(""),
            Line::from(""),
            padded_line("Name:"),
            Line::from(""),
            Line::from(""),
            wizard_button("Download", selected_action),
        ];
        render_dialog_content(frame, "Download Custom Model 2/3", lines, 70, 13);
        render_dialog_input(frame, id_input, 13, 5);
        render_dialog_input(frame, name_input, 13, 8);
        match focus {
            CustomModelDetailsFocus::Id => set_custom_input_cursor(frame, id_input, 13, 5),
            CustomModelDetailsFocus::Name => set_custom_input_cursor(frame, name_input, 13, 8),
        }
    }
}
