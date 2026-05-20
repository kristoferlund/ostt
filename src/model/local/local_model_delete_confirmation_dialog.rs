use ratatui::text::Line;
use ratatui::Frame;

use crate::ui::{render_dialog, DialogAction};

use super::local_model_view_helpers::format_bytes;
use super::types::LocalModelEntry;

pub(super) struct LocalModelDeleteConfirmationDialog;

impl LocalModelDeleteConfirmationDialog {
    pub(super) fn render(
        frame: &mut Frame<'_>,
        entry: &LocalModelEntry,
        _selected_action: DialogAction,
    ) {
        render_dialog(
            frame,
            "Confirm Delete",
            vec![
                Line::from(format!(
                    "Delete \"{}\" ({})?",
                    entry.name,
                    format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
                )),
                Line::from(""),
                Line::from("This cannot be undone."),
            ],
            "Delete",
        );
    }
}
