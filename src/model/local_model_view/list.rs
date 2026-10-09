//! The grouped model list. Section and group headers are rows of their own but
//! never selectable, which ratcn's `List` cannot express, so this paints a
//! `ListWidget` and the screen moves the selection itself.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratcn::runtime::{DeclareCtx, ScopeOptions};
use ratcn::{ListWidget, Theme};

use crate::ui::scroll::{update_scroll_offset, DEFAULT_SCROLL_MARGIN};

use super::format_bytes;
use super::types::{LocalModelEntry, State};
use super::Msg;

enum Row<'a> {
    Blank,
    Section(&'a str),
    Group(&'a str),
    /// Index into the entries.
    Model(usize),
}

/// Each entry, preceded by headers wherever its section or group changes.
fn rows(entries: &[LocalModelEntry]) -> Vec<Row<'_>> {
    let mut rows = Vec::new();
    let mut current_section = None;
    let mut current_group = None;
    for (index, entry) in entries.iter().enumerate() {
        let section = section_label(entry);
        let group = group_label(entry);
        if current_section != Some(section) {
            if current_section.is_some() {
                rows.push(Row::Blank);
            }
            rows.extend([Row::Section(section), Row::Blank]);
            current_section = Some(section);
            current_group = None;
        }
        if current_group != Some(group) {
            if current_group.is_some() {
                rows.push(Row::Blank);
            }
            if group != section {
                rows.extend([Row::Group(group), Row::Blank]);
            }
            current_group = Some(group);
        }
        rows.push(Row::Model(index));
    }
    rows
}

/// Scroll so the selected model stays in view of `height` rows.
pub(super) fn scroll_to_selection(state: &mut State, height: u16) {
    let rows = rows(&state.entries);
    let selected = rows
        .iter()
        .position(|row| matches!(row, Row::Model(index) if *index == state.selected));
    update_scroll_offset(
        &mut state.scroll_offset,
        selected,
        height as usize,
        rows.len(),
        DEFAULT_SCROLL_MARGIN,
    );
}

pub(super) fn declare(ctx: &mut DeclareCtx<'_, State, Msg>, body: Rect) {
    ctx.scope(
        "models",
        body,
        ScopeOptions::default().focusable(true),
        |ctx| {
            ctx.paint(move |paint| {
                let state = paint.state();
                let items: Vec<Text<'static>> = rows(&state.entries)
                    .into_iter()
                    .skip(state.scroll_offset)
                    .map(|row| match row {
                        Row::Blank => Text::from(Line::default()),
                        Row::Section(label) => header(label, Color::Green),
                        Row::Group(label) => header(label, Color::Magenta),
                        Row::Model(index) => {
                            model_row(&state.entries[index], index == state.selected, paint.theme)
                        }
                    })
                    .collect();
                paint.widget(ListWidget::new(&items).themed(paint.theme), body);
            });
        },
    );
}

fn header(label: &str, background: Color) -> Text<'static> {
    Text::from(Span::styled(
        format!(" {label} "),
        Style::default().fg(Color::Black).bg(background),
    ))
}

fn model_row(entry: &LocalModelEntry, is_selected: bool, theme: &Theme) -> Text<'static> {
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

    fn entry(provider_id: &str, id: &str, group_id: Option<&str>) -> LocalModelEntry {
        LocalModelEntry {
            id: id.to_string(),
            provider_id: provider_id.to_string(),
            name: id.to_string(),
            description: String::new(),
            size_mb: 1,
            is_downloaded: false,
            is_active: false,
            is_daemon_loaded: false,
            is_available_in_registry: true,
            languages: Vec::new(),
            url: String::new(),
            recommended_hardware: None,
            category: None,
            sha256: None,
            group_id: group_id.map(str::to_string),
        }
    }

    /// Headers separate sections and provider groups, but only model rows map
    /// back to entries, so the selection can never land on a header.
    #[test]
    fn headers_separate_sections_and_groups_without_taking_entry_indices() {
        let entries = [
            entry("http", "custom", Some("Custom models")),
            entry("openai", "cloud", Some("OpenAI")),
            entry("whisper", "local", None),
        ];
        let layout: Vec<String> = rows(&entries)
            .into_iter()
            .map(|row| match row {
                Row::Blank => String::new(),
                Row::Section(label) => format!("section {label}"),
                Row::Group(label) => format!("group {label}"),
                Row::Model(index) => format!("model {}", entries[index].id),
            })
            .collect();
        assert_eq!(
            layout,
            [
                "section Custom models",
                "",
                "model custom",
                "",
                "section Cloud models",
                "",
                "group OpenAI",
                "",
                "model cloud",
                "",
                "section Local models",
                "",
                "model local",
            ]
        );
    }
}
