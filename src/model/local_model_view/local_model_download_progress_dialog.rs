use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratcn::{runtime::CellOffset, Dialog, ProgressStyle, ProgressWidget};

use crate::transcription::local_models::full_model_id;
use crate::ui::components::modal::{dialog, ModalMessage, ModalViewState};

use super::local_model_view_helpers::format_bytes;
use super::types::DownloadState;

pub(super) struct LocalModelDownloadProgressDialog;

impl LocalModelDownloadProgressDialog {
    pub(super) fn dialog(
        state: &DownloadState,
        offset: CellOffset,
    ) -> Dialog<ModalViewState, ModalMessage> {
        let eta = if state.speed_mbps > 0.0 && state.total_bytes > state.downloaded_bytes {
            let remaining_mb =
                (state.total_bytes - state.downloaded_bytes) as f64 / (1024.0 * 1024.0);
            format!("ETA: {:.0}s", remaining_mb / state.speed_mbps)
        } else {
            "ETA: unknown".to_string()
        };
        let model = format!(
            "Model: {}\nStatus: {}",
            full_model_id(&state.provider_id, &state.model_id),
            state.status
        );
        let stats = format!(
            "{} / {}  •  {:.1} MB/s  •  {}",
            format_bytes(state.downloaded_bytes),
            if state.total_bytes == 0 {
                "unknown".to_string()
            } else {
                format_bytes(state.total_bytes)
            },
            state.speed_mbps,
            eta
        );
        let progress = state.progress;
        dialog(
            if state.is_custom {
                "Download Custom Whisper Model 3/3"
            } else {
                state.status.as_str()
            },
            String::new(),
            "Cancel",
            offset,
        )
        .content(7, move |ctx| {
            let [heading, bar, stats_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .areas(ctx.area());
            ctx.paint_widget(Paragraph::new(model), heading);
            ctx.paint_widget(
                ProgressWidget::new(progress)
                    .show_value(true)
                    .style(ProgressStyle {
                        fill: ratatui::style::Color::White,
                        track: ratatui::style::Color::DarkGray,
                        label: ratatui::style::Color::White,
                        value: ratatui::style::Color::White,
                    }),
                bar,
            );
            ctx.paint_widget(Paragraph::new(stats), stats_area);
        })
    }
}
