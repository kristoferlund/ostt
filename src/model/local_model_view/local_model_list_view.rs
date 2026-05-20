use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

use crate::ui::{render_app_layout, render_footer, render_title};

use super::local_model_view_helpers::format_bytes;
use super::types::{LocalModelEntry, LocalModelsTui};

pub(super) struct LocalModelListView;

impl LocalModelListView {
    pub(super) fn render(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
        let layout = render_app_layout(frame, frame.area());
        render_title(frame, layout.title, "Local Models");

        let selected_id = tui.selected_entry().map(|e| e.id.as_str());
        let mut items = Vec::new();
        push_grouped_model_items(&mut items, tui.entries.iter().collect(), selected_id);

        let selected_display_index = display_index_for_selected_model(tui);
        let mut state = ListState::default().with_selected(selected_display_index);
        // highlight_style is intentionally blank — selection bg is applied per-span below
        // so pill background colours are preserved on the selected row.
        frame.render_stateful_widget(
            List::new(items).highlight_style(Style::default()),
            layout.body,
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
        label.into(),
        Style::default().add_modifier(Modifier::BOLD),
    )))
}

fn push_grouped_model_items(
    items: &mut Vec<ListItem<'static>>,
    entries: Vec<&LocalModelEntry>,
    selected_id: Option<&str>,
) {
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
        let is_selected = selected_id == Some(entry.id.as_str());
        items.push(local_model_list_item(entry, is_selected));
    }
}

fn local_model_list_item(entry: &LocalModelEntry, is_selected: bool) -> ListItem<'static> {
    let active_marker = if entry.is_active { "◉" } else { "○" };
    let size = format_bytes(u64::from(entry.size_mb) * 1024 * 1024);
    let description = entry.description.trim();

    let row_bg = if is_selected {
        Color::DarkGray
    } else {
        Color::Reset
    };
    let row_style = Style::default().bg(row_bg);

    let mut spans = vec![Span::styled(format!("{active_marker} "), row_style)];

    if entry.is_downloaded {
        let (pill_fg, pill_bg) = if is_selected {
            (Color::White, Color::LightGreen)
        } else {
            (Color::White, Color::Green)
        };
        spans.push(Span::styled(
            " dl ",
            Style::default().fg(pill_fg).bg(pill_bg),
        ));
        spans.push(Span::styled(" ", row_style));
    }

    if entry.is_daemon_loaded {
        let (pill_fg, pill_bg) = if is_selected {
            (Color::White, Color::LightMagenta)
        } else {
            (Color::White, Color::Magenta)
        };
        spans.push(Span::styled(
            " run ",
            Style::default().fg(pill_fg).bg(pill_bg),
        ));
        spans.push(Span::styled(" ", row_style));
    }

    spans.push(Span::styled(
        format!("{}, {}, {}", entry.name, size, description),
        row_style,
    ));

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
