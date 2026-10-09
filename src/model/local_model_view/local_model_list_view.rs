use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::Frame;
use ratcn::ListWidget;

use crate::ui::scroll::update_scroll_offset;
use crate::ui::{render_app_layout, render_footer};

use super::local_model_view_helpers::format_bytes;
use super::types::{LocalModelEntry, LocalModelsTui};

pub(super) struct LocalModelListView;

impl LocalModelListView {
    pub(super) fn render(frame: &mut Frame<'_>, tui: &mut LocalModelsTui) {
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
        update_scroll_offset(
            &mut tui.scroll_offset,
            selected_display_index,
            body.height as usize,
            items.len(),
            crate::ui::scroll::DEFAULT_SCROLL_MARGIN,
        );
        // Keep selection styling per-span so downloaded/running pills retain their colors.
        // Headers and separators remain display rows, not navigable model entries.
        frame.render_widget(ListWidget::new(&items[tui.scroll_offset..]), body);

        render_footer(
            frame,
            layout.footer,
            "↑↓ nav, ↵ activate/download, x/del delete, i info, c custom, esc/q back",
        );
    }
}

fn section_header(label: impl Into<String>) -> Text<'static> {
    Text::from(Line::from(Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(Color::Black).bg(Color::Green),
    )))
}

fn group_header(label: impl Into<String>) -> Text<'static> {
    Text::from(Line::from(Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(Color::Black).bg(Color::Magenta),
    )))
}

fn push_grouped_model_items(
    items: &mut Vec<Text<'static>>,
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
                items.push(Text::from(Line::from("")));
            }
            items.push(section_header(section.to_string()));
            items.push(Text::from(Line::from("")));
            current_section = Some(section);
            current_group = None;
        }
        if current_group != Some(group) {
            if current_group.is_some() {
                items.push(Text::from(Line::from("")));
            }
            if group != section {
                items.push(group_header(group.to_string()));
                items.push(Text::from(Line::from("")));
            }
            current_group = Some(group);
        }
        let is_selected = selected_id == Some(model_key(entry).as_str());
        items.push(local_model_list_item(entry, is_selected));
    }
}

fn local_model_list_item(entry: &LocalModelEntry, is_selected: bool) -> Text<'static> {
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

    Text::from(Line::from(spans))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::{List, ListItem, ListState, StatefulWidget, Widget};

    fn entry(provider: &str, id: &str, group: Option<&str>) -> LocalModelEntry {
        LocalModelEntry {
            id: id.to_string(),
            provider_id: provider.to_string(),
            name: format!("Model {id}"),
            description: "Multilingual 日本語".to_string(),
            size_mb: 100,
            is_downloaded: provider == "whisper",
            is_active: provider == "whisper",
            is_daemon_loaded: provider == "whisper",
            is_available_in_registry: true,
            languages: vec![],
            url: String::new(),
            recommended_hardware: None,
            category: None,
            sha256: None,
            group_id: group.map(str::to_string),
        }
    }

    #[test]
    fn grouped_headers_and_separators_do_not_change_model_selection_indices() {
        let entries = [
            entry("http", "custom", Some("Custom models")),
            entry("openai", "cloud", Some("OpenAI")),
            entry("whisper", "local", None),
        ];
        let mut rows = Vec::new();
        push_grouped_model_items(&mut rows, entries.iter().collect(), Some("whisper/local"));

        for (key, expected_index) in [
            ("http/custom", 2),
            ("openai/cloud", 8),
            ("whisper/local", 12),
        ] {
            assert_eq!(
                grouped_display_index(entries.iter().collect(), key, 0),
                Some(expected_index)
            );
            assert!(rows[expected_index].to_string().contains(key));
        }
        assert!(rows[0].to_string().contains("Custom models"));
        assert!(rows[4].to_string().contains("Cloud models"));
        assert!(rows[6].to_string().contains("OpenAI"));
        assert!(rows[10].to_string().contains("Local models"));
    }

    #[test]
    fn ratcn_list_preserves_legacy_cells_when_scrolled_or_clipped() {
        let entries = [
            entry("http", "custom", Some("Custom models")),
            entry("openai", "cloud", Some("OpenAI")),
            entry("whisper", "local", None),
        ];
        let mut rows = Vec::new();
        push_grouped_model_items(&mut rows, entries.iter().collect(), Some("whisper/local"));

        // Compare all cell styles as well as glyphs: selection must not erase pill colors.
        for (width, height) in [(100, 20), (30, 4), (1, 1), (0, 0)] {
            for offset in 0..rows.len() {
                let area = Rect::new(2, 1, width, height);
                let mut legacy = Buffer::empty(area);
                let mut migrated = Buffer::empty(area);
                let mut state = ListState::default().with_offset(offset);
                StatefulWidget::render(
                    List::new(rows.iter().cloned().map(ListItem::new)),
                    area,
                    &mut legacy,
                    &mut state,
                );
                ListWidget::new(&rows[offset..]).render(area, &mut migrated);
                assert_eq!(legacy, migrated, "size {width}x{height}, offset {offset}");
            }
        }
    }
}
