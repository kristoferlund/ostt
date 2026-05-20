use ratatui::text::Line;
use ratatui::Frame;

use crate::ui::{render_dialog, DialogAction};

use super::local_model_view_helpers::format_bytes;
use super::types::LocalModelEntry;

pub(super) struct LocalModelDownloadConfirmationDialog;

impl LocalModelDownloadConfirmationDialog {
    pub(super) fn render(
        frame: &mut Frame<'_>,
        entry: &LocalModelEntry,
        _selected_action: DialogAction,
    ) {
        render_dialog(
            frame,
            "Start Download",
            vec![
                Line::from(format!("Download \"{}\"?", entry.name)),
                Line::from(""),
                Line::from(format!(
                    "Size: {}",
                    format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
                )),
            ],
            "Download",
        );
    }
}
