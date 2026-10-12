//! The model screen's dialogs. Each is rebuilt from state every frame.

use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratcn::{runtime::DeclareCtx, Dialog, Input, ProgressWidget};

use super::types::{DownloadState, Mode, ModelEntry, State};
use super::{format_bytes, Msg};
use crate::transcription::local_models::full_model_id;
use crate::ui::components::modal::{self, form_dialog, message_dialog};

pub(super) fn dialog(state: &State) -> Option<Dialog<State, Msg>> {
    let dialog = match &state.mode {
        Mode::Browse | Mode::Info { .. } => return None,
        Mode::CustomUrl => form_dialog(
            "Download Custom Whisper Model 1/3",
            "Paste a Hugging Face model page or a direct model file URL.\nSupported files: .gguf and ggml-*.bin.",
            vec![(
                "",
                Input::new()
                    .placeholder("https://huggingface.co/…")
                    .value(|s: &State| &s.url_input, Msg::Url)
                    .on_submit(|| Msg::Accept),
            )],
            "Next",
            || Msg::Accept,
        ),
        Mode::CustomDetails { .. } => form_dialog(
            "Download Custom Whisper Model 2/3",
            "Choose how this custom model should appear in OSTT.",
            vec![
                (
                    "ID",
                    Input::new()
                        .value(|s: &State| &s.id_input, Msg::Id)
                        .on_submit(|| Msg::Accept),
                ),
                (
                    "Name",
                    Input::new()
                        .value(|s: &State| &s.name_input, Msg::Name)
                        .on_submit(|| Msg::Accept),
                ),
            ],
            "Download",
            || Msg::Accept,
        ),
        Mode::ConfirmDownload { entry } => message_dialog(
            "Start Download",
            format!(
                "Download \"{}\"?\nID: {}\n\nSize: {}",
                entry.name,
                full_model_id(&entry.provider_id, &entry.id),
                entry_size(entry)
            ),
            "Download",
            || Msg::Accept,
        ),
        Mode::Downloading(download) => progress(download),
        Mode::ConfirmDelete { entry } => message_dialog(
            "Confirm Delete",
            format!(
                "Delete \"{}\" ({})?\nID: {}\n\nThis cannot be undone.",
                entry.name,
                entry_size(entry),
                full_model_id(&entry.provider_id, &entry.id)
            ),
            "Delete",
            || Msg::Accept,
        ),
        Mode::Error { message } => message_dialog("Error", message.clone(), "Close", || Msg::Accept),
    };
    Some(
        dialog
            .offset(state.dialog_offset)
            .on_offset_change(Msg::DialogMoved)
            .on_dismiss(|| Msg::Dismiss),
    )
}

fn entry_size(entry: &ModelEntry) -> String {
    format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
}

fn progress(download: &DownloadState) -> Dialog<State, Msg> {
    let eta = if download.speed_mbps > 0.0 && download.total_bytes > download.downloaded_bytes {
        let remaining_mb =
            (download.total_bytes - download.downloaded_bytes) as f64 / (1024.0 * 1024.0);
        format!("ETA: {:.0}s", remaining_mb / download.speed_mbps)
    } else {
        "ETA: unknown".to_string()
    };
    let model = format!(
        "Model: {}\nStatus: {}",
        full_model_id(&download.provider_id, &download.model_id),
        download.status
    );
    let total = if download.total_bytes == 0 {
        "unknown".to_string()
    } else {
        format_bytes(download.total_bytes)
    };
    let stats = format!(
        "{:.0}%  •  {} / {total}  •  {:.1} MB/s  •  {eta}",
        download.progress * 100.0,
        format_bytes(download.downloaded_bytes),
        download.speed_mbps,
    );
    let progress = download.progress;
    let title = if download.is_custom {
        "Download Custom Whisper Model 3/3"
    } else {
        download.status.as_str()
    };
    let body = move |ctx: &mut DeclareCtx<'_, State, Msg>| {
        let [heading, bar, stats_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .areas(ctx.area());
        ctx.paint_widget(Paragraph::new(model).centered(), heading);
        ctx.paint(move |paint| {
            paint.widget(ProgressWidget::new(progress).themed(paint.theme), bar)
        });
        ctx.paint_widget(Paragraph::new(stats).centered(), stats_area);
    };
    modal::dialog(title, 7, body, "Cancel", || Msg::Accept)
}
