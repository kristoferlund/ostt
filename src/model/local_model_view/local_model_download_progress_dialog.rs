use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::ui::render_dialog_content;

use super::local_model_view_helpers::{format_bytes, progress_bar};
use super::types::DownloadState;

pub(super) struct LocalModelDownloadProgressDialog;

impl LocalModelDownloadProgressDialog {
    pub(super) fn render(frame: &mut Frame<'_>, state: &DownloadState) {
        let eta = if state.speed_mbps > 0.0 && state.total_bytes > state.downloaded_bytes {
            let remaining_mb =
                (state.total_bytes - state.downloaded_bytes) as f64 / (1024.0 * 1024.0);
            format!("ETA: {:.0}s", remaining_mb / state.speed_mbps)
        } else {
            "ETA: unknown".to_string()
        };
        render_dialog_content(
            frame,
            if state.is_custom {
                "Download Custom Model 3/3"
            } else {
                state.status.as_str()
            },
            vec![
                Line::from(format!("Model: {}", state.model_id)),
                if state.is_custom {
                    Line::from(format!("Status: {}", state.status))
                } else {
                    Line::from("")
                },
                Line::from(""),
                Line::from(progress_bar(state.progress, 58)),
                Line::from(""),
                Line::from(format!(
                    "{} / {}  •  {:.1} MB/s  •  {}",
                    format_bytes(state.downloaded_bytes),
                    if state.total_bytes == 0 {
                        "unknown".to_string()
                    } else {
                        format_bytes(state.total_bytes)
                    },
                    state.speed_mbps,
                    eta
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "<Cancel>",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
            ],
            70,
            10,
        );
    }
}
