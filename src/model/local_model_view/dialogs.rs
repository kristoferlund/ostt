//! The model screen's dialogs. Each is rebuilt from state every frame.

use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratcn::{Button, Dialog, Input, ProgressWidget};

use super::types::{DownloadState, LocalModelEntry, Mode, State};
use super::{format_bytes, Msg};
use crate::transcription::local_models::full_model_id;
use crate::ui::components::modal::form_dialog;

pub(super) fn dialog(state: &State) -> Option<Dialog<State, Msg>> {
    let dialog = match &state.mode {
        Mode::Browse | Mode::Info { .. } => return None,
        Mode::CustomUrl => form_dialog(
            base("Download Custom Whisper Model 1/3", "Next"),
            "Paste a Hugging Face model page or a direct model file URL.\nSupported files: .gguf and ggml-*.bin.",
            vec![Input::new()
                .title("URL")
                .value(|s: &State| &s.url_input, Msg::Url)
                .on_submit(|| Msg::Accept)],
        ),
        Mode::CustomDetails { .. } => form_dialog(
            base("Download Custom Whisper Model 2/3", "Download"),
            "Choose how this custom model should appear in OSTT.",
            vec![
                Input::new()
                    .title("ID")
                    .value(|s: &State| &s.id_input, Msg::Id)
                    .on_submit(|| Msg::Accept),
                Input::new()
                    .title("Name")
                    .value(|s: &State| &s.name_input, Msg::Name)
                    .on_submit(|| Msg::Accept),
            ],
        ),
        Mode::ConfirmDownload { entry } => base("Start Download", "Download").description(format!(
            "Download \"{}\"?\nID: {}\n\nSize: {}",
            entry.name,
            full_model_id(&entry.provider_id, &entry.id),
            entry_size(entry)
        )),
        Mode::Downloading(download) => progress(download),
        Mode::ConfirmDelete { entry } => base("Confirm Delete", "Delete").description(format!(
            "Delete \"{}\" ({})?\nID: {}\n\nThis cannot be undone.",
            entry.name,
            entry_size(entry),
            full_model_id(&entry.provider_id, &entry.id)
        )),
        Mode::Error { message } => base("Error", "Close").description(message.clone()),
    };
    Some(dialog.offset(state.dialog_offset))
}

fn base(title: &str, action: &'static str) -> Dialog<State, Msg> {
    Dialog::new()
        .title(title)
        .on_offset_change(Msg::DialogMoved)
        .on_dismiss(|| Msg::Dismiss)
        .action("accept", Button::new(action).on_press(|| Msg::Accept))
}

fn entry_size(entry: &LocalModelEntry) -> String {
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
        "{} / {total}  •  {:.1} MB/s  •  {eta}",
        format_bytes(download.downloaded_bytes),
        download.speed_mbps,
    );
    let progress = download.progress;
    let title = if download.is_custom {
        "Download Custom Whisper Model 3/3"
    } else {
        download.status.as_str()
    };
    base(title, "Cancel").content(7, move |ctx| {
        let [heading, bar, stats_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .areas(ctx.area());
        ctx.paint_widget(Paragraph::new(model), heading);
        ctx.paint(move |paint| {
            paint.widget(
                ProgressWidget::new(progress)
                    .show_value(true)
                    .themed(paint.theme),
                bar,
            )
        });
        ctx.paint_widget(Paragraph::new(stats), stats_area);
    })
}
