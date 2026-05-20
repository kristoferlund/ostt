use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::fs;
use std::time::SystemTime;
use tui_input::Input;

use crate::transcription::local_models::{model_destination, RegistryEntry};
use crate::ui::{
    dialog_content_area, render_app_layout, render_dialog, render_dialog_content,
    render_error_dialog, render_footer, render_title, render_toast, DialogAction,
};

use super::types::{CustomModelDetailsFocus, LocalModelEntry, LocalModelsMode, LocalModelsTui};

pub(crate) fn render_local_models(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
    match &tui.mode {
        LocalModelsMode::Browse => render_browse(frame, tui),
        LocalModelsMode::Info { entry } => {
            render_info(frame, entry);
        }
        LocalModelsMode::ConfirmDelete {
            entry,
            selected_action,
        } => {
            render_browse(frame, tui);
            render_confirm_delete(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmDownload {
            entry,
            selected_action,
        } => {
            render_browse(frame, tui);
            render_confirm_download(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmAudioConfig {
            entry,
            selected_action,
        } => {
            render_browse(frame, tui);
            render_confirm_audio_config(frame, entry, *selected_action);
        }
        LocalModelsMode::CustomModelInput {
            input,
            selected_action,
        } => {
            render_browse(frame, tui);
            render_custom_input(frame, input, *selected_action);
            set_custom_input_cursor(frame, input, 10, 5);
        }
        LocalModelsMode::CustomModelDetails {
            id_input,
            name_input,
            focus,
            selected_action,
            ..
        } => {
            render_browse(frame, tui);
            render_custom_details(frame, id_input, name_input, *focus, *selected_action);
            match focus {
                CustomModelDetailsFocus::Id => set_custom_input_cursor(frame, id_input, 13, 5),
                CustomModelDetailsFocus::Name => set_custom_input_cursor(frame, name_input, 13, 8),
            }
        }
        LocalModelsMode::Downloading(state) => {
            render_browse(frame, tui);
            render_download(frame, state);
        }
        LocalModelsMode::ErrorDialog {
            message,
            return_mode,
        } => {
            match return_mode.as_ref() {
                LocalModelsMode::CustomModelInput {
                    input,
                    selected_action,
                } => {
                    render_browse(frame, tui);
                    render_custom_input(frame, input, *selected_action);
                    set_custom_input_cursor(frame, input, 10, 5);
                }
                _ => render_browse(frame, tui),
            }
            render_error_dialog(frame, "Error", message.clone());
        }
    }

    if let Some(toast) = &tui.toast {
        render_toast(frame, toast);
    }
}

fn render_browse(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
    let layout = render_app_layout(frame, frame.area());
    let list_area = render_title(frame, layout.body, "Local Models");

    let mut items = Vec::new();
    push_grouped_model_items(&mut items, tui.entries.iter().collect());

    let selected_display_index = display_index_for_selected_model(tui);
    let mut state = ListState::default().with_selected(selected_display_index);
    frame.render_stateful_widget(
        List::new(items).highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        list_area,
        &mut state,
    );

    render_footer(
        frame,
        layout.footer,
        "↑↓ nav, ↵ activate/download, x/del delete, i info, c custom, esc/q back",
    );
}

fn render_info(frame: &mut Frame<'_>, entry: &LocalModelEntry) {
    let layout = render_app_layout(frame, frame.area());
    let content_area = render_title(frame, layout.body, &entry.name);

    let (_, path) = downloaded_details(entry);
    let mut lines = vec![
        Line::from(format!("ID: {}", entry.id)),
        Line::from(""),
        Line::from(entry.description.clone()),
        Line::from(""),
        Line::from(format!(
            "Recommended hardware: {}",
            entry.recommended_hardware.as_deref().unwrap_or("none")
        )),
        Line::from(""),
        Line::from(format!("Size (MB): {}", entry.size_mb)),
        Line::from(format!(
            "Languages: {}",
            if entry.languages.is_empty() {
                "unknown".to_string()
            } else {
                entry.languages.join(", ")
            }
        )),
        Line::from(format!("Url: {}", entry.url)),
        Line::from(format!(
            "Downloaded: {}",
            if entry.is_downloaded { "Yes" } else { "No" }
        )),
        Line::from(format!(
            "Active: {}",
            if entry.is_active { "Yes" } else { "No" }
        )),
    ];
    if entry.is_downloaded {
        lines.push(Line::from(format!("Local path: {path}")));
    }
    if let Some(sha256) = &entry.sha256 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("SHA256: {sha256}")));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        content_area,
    );
    render_footer(frame, layout.footer, "esc/q back");
}

fn render_confirm_delete(
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

fn render_confirm_download(
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

fn render_confirm_audio_config(
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

fn render_custom_input(frame: &mut Frame<'_>, input: &Input, selected_action: DialogAction) {
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
}

fn render_custom_details(
    frame: &mut Frame<'_>,
    id_input: &Input,
    name_input: &Input,
    _focus: CustomModelDetailsFocus,
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
}

fn set_custom_input_cursor(frame: &mut Frame<'_>, input: &Input, dialog_height: u16, line: u16) {
    let inner_area = dialog_content_area(70, dialog_height, frame.area());
    let cursor_x = inner_area
        .x
        .saturating_add(1)
        .saturating_add(input.cursor() as u16)
        .min(inner_area.x.saturating_add(inner_area.width.saturating_sub(1)));
    frame.set_cursor_position(Position::new(cursor_x, inner_area.y.saturating_add(line)));
}

fn render_dialog_input(frame: &mut Frame<'_>, input: &Input, dialog_height: u16, line: u16) {
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

fn render_download(frame: &mut Frame<'_>, state: &super::types::DownloadState) {
    let eta = if state.speed_mbps > 0.0 && state.total_bytes > state.downloaded_bytes {
        let remaining_mb = (state.total_bytes - state.downloaded_bytes) as f64 / (1024.0 * 1024.0);
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

fn section_header(label: impl Into<String>) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        label.into(),
        Style::default().add_modifier(Modifier::BOLD),
    )))
}

fn push_grouped_model_items(items: &mut Vec<ListItem<'static>>, entries: Vec<&LocalModelEntry>) {
    let mut current_group: Option<&str> = None;
    for entry in entries {
        let group = entry.group_id.as_deref().unwrap_or("Custom");
        if current_group != Some(group) {
            if current_group.is_some() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(section_header(group.to_string()));
            current_group = Some(group);
        }
        items.push(local_model_list_item(entry));
    }
}

fn local_model_list_item(entry: &LocalModelEntry) -> ListItem<'static> {
    let active_marker = if entry.is_active { "◉" } else { "○" };
    let size = format_bytes(u64::from(entry.size_mb) * 1024 * 1024);
    let description = entry.description.trim();

    let mut spans = vec![Span::raw(format!("{active_marker} "))];
    if entry.is_downloaded {
        spans.push(Span::raw("✅ "));
    }
    spans.push(Span::raw(format!(
        "{}, {}, {}",
        entry.name, size, description,
    )));

    ListItem::new(Line::from(spans))
}

fn display_index_for_selected_model(tui: &LocalModelsTui) -> Option<usize> {
    let selected_entry_id = tui.selected_entry().map(|entry| entry.id.as_str())?;
    grouped_display_index(tui.entries.iter().collect(), selected_entry_id, 0)
}

pub(super) fn grouped_display_index(
    entries: Vec<&LocalModelEntry>,
    selected_entry_id: &str,
    start_index: usize,
) -> Option<usize> {
    let mut index = start_index;
    let mut current_group: Option<&str> = None;
    for entry in entries {
        let group = entry.group_id.as_deref().unwrap_or("Custom");
        if current_group != Some(group) {
            if current_group.is_some() {
                index += 1;
            }
            index += 1;
            current_group = Some(group);
        }
        if entry.id == selected_entry_id {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn padded_line(text: impl Into<String>) -> Line<'static> {
    Line::from(format!(" {}", text.into()))
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

fn progress_bar(progress: f64, width: usize) -> String {
    let progress = progress.clamp(0.0, 1.0);
    let completed = (progress * width as f64).round() as usize;
    let remaining = width.saturating_sub(completed);
    format!("{}{}", "█".repeat(completed), "░".repeat(remaining))
}

fn registry_entry_from_model(entry: &LocalModelEntry) -> RegistryEntry {
    RegistryEntry {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        languages: entry.languages.clone(),
        size_mb: entry.size_mb,
        url: entry.url.clone(),
        recommended_hardware: entry.recommended_hardware.clone(),
        sha256: entry.sha256.clone(),
        category: entry.category.clone(),
        group_id: entry.group_id.clone(),
    }
}

fn downloaded_details(entry: &LocalModelEntry) -> (String, String) {
    let path = model_destination(&registry_entry_from_model(entry));
    let downloaded = fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| format!("{}s since epoch", duration.as_secs()))
        .unwrap_or_else(|| "No".to_string());

    (downloaded, path.display().to_string())
}

fn wizard_button(action: &'static str, _selected_action: DialogAction) -> Line<'static> {
    Line::from(Span::styled(
        format!("<{action}>"),
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
}
