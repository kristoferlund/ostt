use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

use crate::ui::{render_app_layout, render_footer};

use super::local_model_view_helpers::format_bytes;
use super::types::{LocalModelEntry, LocalModelsTui};

pub(super) struct LocalModelListView;

impl LocalModelListView {
    pub(super) fn render(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
        let layout = render_app_layout(frame, frame.area());
        let body = Rect {
            x: layout.title.x,
            y: layout.title.y,
            width: layout.title.width,
            height: layout.title.height.saturating_add(layout.body.height),
        };

        let selected_id = tui.selected_entry().map(model_key);
        let mut items = Vec::new();
        push_grouped_model_items(
            &mut items,
            tui.entries.iter().collect(),
            selected_id.as_deref(),
        );

        let selected_display_index = display_index_for_selected_model(tui);
        let mut state = ListState::default().with_selected(selected_display_index);
        // highlight_style is intentionally blank — selection bg is applied per-span below
        // so pill background colours are preserved on the selected row.
        frame.render_stateful_widget(
            List::new(items).highlight_style(Style::default()),
            body,
            &mut state,
        );

        render_footer(
            frame,
            layout.footer,
            "↑↓ nav, ↵ activate/download, x/del delete, i info, c custom, esc/q back",
        );
    }
}

fn section_header(label: impl Into<String>) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(Color::Black).bg(Color::Green),
    )))
}

fn group_header(label: impl Into<String>) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(Color::Black).bg(Color::Magenta),
    )))
}

fn push_grouped_model_items(
    items: &mut Vec<ListItem<'static>>,
    entries: Vec<&LocalModelEntry>,
    selected_id: Option<&str>,
) {
    let mut current_section: Option<&str> = None;
    let mut current_group: Option<&str> = None;
    for entry in entries {
        let section = section_label(entry);
        let group = group_label(entry);
        if current_section != Some(section) {
            if current_section.is_some() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(section_header(section.to_string()));
            items.push(ListItem::new(Line::from("")));
            current_section = Some(section);
            current_group = None;
        }
        if current_group != Some(group) {
            if current_group.is_some() {
                items.push(ListItem::new(Line::from("")));
            }
            if group != section {
                items.push(group_header(group.to_string()));
                items.push(ListItem::new(Line::from("")));
            }
            current_group = Some(group);
        }
        let is_selected = selected_id.as_deref() == Some(model_key(entry).as_str());
        items.push(local_model_list_item(entry, is_selected));
    }
}

fn local_model_list_item(entry: &LocalModelEntry, is_selected: bool) -> ListItem<'static> {
    let active_marker = if entry.is_active { "◉" } else { "○" };
    let description = entry.description.trim();

    let row_bg = if is_selected {
        Color::DarkGray
    } else {
        Color::Reset
    };
    let row_style = Style::default().bg(row_bg);

    let mut spans = vec![Span::styled(format!("{active_marker} "), row_style)];

    if entry.is_downloaded && entry.provider_id == "whisper" {
        let (pill_fg, pill_bg) = if is_selected {
            (Color::Black, Color::LightGreen)
        } else {
            (Color::Black, Color::Green)
        };
        spans.push(Span::styled(
            " dl ",
            Style::default().fg(pill_fg).bg(pill_bg),
        ));
        spans.push(Span::styled(" ", row_style));
    }

    if entry.is_daemon_loaded {
        let (pill_fg, pill_bg) = if is_selected {
            (Color::Black, Color::LightMagenta)
        } else {
            (Color::Black, Color::Magenta)
        };
        spans.push(Span::styled(
            " run ",
            Style::default().fg(pill_fg).bg(pill_bg),
        ));
        spans.push(Span::styled(" ", row_style));
    }

    let details = if entry.provider_id == "whisper" {
        let size = format_bytes(u64::from(entry.size_mb) * 1024 * 1024);
        if description.is_empty() {
            format!(
                "{}  {}/{}  {}",
                entry.name, entry.provider_id, entry.id, size
            )
        } else {
            format!(
                "{}  {}/{}  {}  {}",
                entry.name, entry.provider_id, entry.id, size, description
            )
        }
    } else if description.is_empty() {
        format!("{}  {}/{}", entry.name, entry.provider_id, entry.id)
    } else {
        format!(
            "{}  {}/{}  {}",
            entry.name, entry.provider_id, entry.id, description
        )
    };

    spans.push(Span::styled(details, row_style));

    ListItem::new(Line::from(spans))
}

fn display_index_for_selected_model(tui: &LocalModelsTui) -> Option<usize> {
    let selected_entry_id = tui.selected_entry().map(model_key)?;
    grouped_display_index(tui.entries.iter().collect(), &selected_entry_id, 0)
}

pub(super) fn grouped_display_index(
    entries: Vec<&LocalModelEntry>,
    selected_entry_id: &str,
    start_index: usize,
) -> Option<usize> {
    let mut index = start_index;
    let mut current_section: Option<&str> = None;
    let mut current_group: Option<&str> = None;
    for entry in entries {
        let section = section_label(entry);
        let group = group_label(entry);
        if current_section != Some(section) {
            if current_section.is_some() {
                index += 1;
            }
            index += 1;
            index += 1;
            current_section = Some(section);
            current_group = None;
        }
        if current_group != Some(group) {
            if current_group.is_some() {
                index += 1;
            }
            if group != section {
                index += 2;
            }
            current_group = Some(group);
        }
        if model_key(entry) == selected_entry_id {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn model_key(entry: &LocalModelEntry) -> String {
    format!("{}/{}", entry.provider_id, entry.id)
}

fn section_label(entry: &LocalModelEntry) -> &'static str {
    if entry.group_id.as_deref() == Some("Custom models") {
        "Custom models"
    } else if entry.provider_id == "whisper" {
        "Local models"
    } else {
        "Cloud models"
    }
}

fn group_label(entry: &LocalModelEntry) -> &str {
    entry
        .group_id
        .as_deref()
        .unwrap_or_else(|| section_label(entry))
}
