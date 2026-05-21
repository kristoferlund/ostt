use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::ui::{render_dialog_content, DialogAction};

use super::types::LocalModelEntry;

pub(super) struct LocalModelAudioConfigConfirmationDialog;

impl LocalModelAudioConfigConfirmationDialog {
    pub(super) fn render(
        frame: &mut Frame<'_>,
        entry: &LocalModelEntry,
        _selected_action: DialogAction,
    ) {
        render_dialog_content(
            frame,
            "Update Audio Config",
            vec![
                Line::from(format!("Activate \"{}\"?", entry.name)),
                Line::from(""),
                Line::from("Local transcription requires WAV audio:"),
                Line::from("output_format = \"pcm_s16le -ar 16000\""),
                Line::from("sample_rate = 16000"),
                Line::from(""),
                Line::from(Span::styled(
                    "<Update>",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
            ],
            70,
            12,
        );
    }
}
