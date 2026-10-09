use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratcn::runtime::{DeclareCtx, ScopeOptions};
use ratcn::{ListWidget, Theme};

use crate::ui::scroll::update_scroll_offset;

use super::local_model_view_helpers::format_bytes;
use super::types::{LocalModelEntry, LocalModelsTui};
use super::Msg;

pub(super) struct LocalModelListView;

impl LocalModelListView {
    pub(super) fn prepare(tui: &mut LocalModelsTui, height: u16) {
        let item_count = tui
            .entries
            .last()
            .and_then(|entry| {
                grouped_display_index(tui.entries.iter().collect(), &model_key(entry), 0)
            })
            .map_or(0, |index| index + 1);
        let selected_display_index = display_index_for_selected_model(tui);
        update_scroll_offset(
            &mut tui.scroll_offset,
            selected_display_index,
            height as usize,
            item_count,
            crate::ui::scroll::DEFAULT_SCROLL_MARGIN,
        );
    }

    pub(super) fn declare(ctx: &mut DeclareCtx<'_, LocalModelsTui, Msg>, body: Rect) {
        ctx.scope(
            "models",
            body,
            ScopeOptions::default().focusable(true),
            |ctx| {
                ctx.paint(move |paint| {
                    let tui = paint.state();
                    let selected_id = tui.selected_entry().map(model_key);
                    let mut items = Vec::new();
                    push_grouped_model_items(
                        &mut items,
                        tui.entries.iter().collect(),
                        selected_id.as_deref(),
                        paint.theme,
                    );
                    let offset = tui.scroll_offset;
                    // Keep per-span pills and grouping; headers are not selectable entries.
                    paint.widget(ListWidget::new(&items[offset..]).themed(paint.theme), body);
                });
            },
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
    theme: &Theme,
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
        items.push(local_model_list_item(entry, is_selected, theme));
    }
}

fn local_model_list_item(
    entry: &LocalModelEntry,
    is_selected: bool,
    theme: &Theme,
) -> Text<'static> {
    let active_marker = if entry.is_active { "◉" } else { "○" };
    let description = entry.description.trim();

    let row_style = if is_selected {
        Style::default()
            .fg(theme.primary_foreground)
            .bg(theme.primary)
    } else {
        Style::default().fg(theme.foreground).bg(theme.field)
    };

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
        push_grouped_model_items(
            &mut rows,
            entries.iter().collect(),
            Some("whisper/local"),
            &Theme::default_dark(),
        );

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
    fn adaptive_model_rows_preserve_grouping_and_pills_when_scrolled_or_clipped() {
        let entries = [
            entry("http", "custom", Some("Custom models")),
            entry("openai", "cloud", Some("OpenAI")),
            entry("whisper", "local", None),
        ];
        for theme in [
            Theme::adaptive(Color::Rgb(26, 27, 38), Color::Rgb(192, 202, 245), None),
            Theme::adaptive(Color::Rgb(253, 246, 227), Color::Rgb(101, 123, 131), None),
        ] {
            let mut rows = Vec::new();
            push_grouped_model_items(
                &mut rows,
                entries.iter().collect(),
                Some("whisper/local"),
                &theme,
            );
            // Compare glyphs and pill styling against Ratatui with the same active palette.
            for (width, height) in [(100, 20), (30, 4), (1, 1), (0, 0)] {
                for offset in 0..rows.len() {
                    let area = Rect::new(2, 1, width, height);
                    let mut legacy = Buffer::empty(area);
                    let mut migrated = Buffer::empty(area);
                    let mut state = ListState::default().with_offset(offset);
                    StatefulWidget::render(
                        List::new(rows.iter().cloned().map(ListItem::new)).style(
                            Style::default()
                                .fg(ratcn::ListStyle::from_theme(&theme).foreground)
                                .bg(ratcn::ListStyle::from_theme(&theme).background),
                        ),
                        area,
                        &mut legacy,
                        &mut state,
                    );
                    ListWidget::new(&rows[offset..])
                        .themed(&theme)
                        .render(area, &mut migrated);
                    assert_eq!(legacy, migrated, "size {width}x{height}, offset {offset}");
                }
            }
        }
    }
}
