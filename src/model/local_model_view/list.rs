//! The grouped model list. Section and group headers are rows of their own,
//! disabled so the cursor skips them and the selection only lands on models.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratcn::runtime::DeclareCtx;
use ratcn::{List, ListItem, ListStyle};

use super::format_bytes;
use super::types::{LocalModelEntry, State};
use super::Msg;

/// Every row is keyed by its position, since list values must be unique and
/// a group name can head more than one run. Header text rides in the label.
#[derive(Clone, PartialEq)]
enum Row {
    Blank(usize),
    Section(usize),
    Group(usize),
    /// Index into the entries.
    Model(usize),
}

/// Each entry, preceded by headers wherever its section or group changes.
/// Only model rows are enabled, so the cursor skips the headers.
fn rows(entries: &[LocalModelEntry]) -> Vec<ListItem<Row>> {
    fn header(rows: &mut Vec<ListItem<Row>>, row: fn(usize) -> Row, label: &str) {
        rows.push(ListItem::new(row(rows.len()), label).disabled(true));
    }
    let mut rows = Vec::new();
    let mut current_section = None;
    let mut current_group = None;
    for (index, entry) in entries.iter().enumerate() {
        let section = section_label(entry);
        let group = group_label(entry);
        if current_section != Some(section) {
            if current_section.is_some() {
                header(&mut rows, Row::Blank, "");
            }
            header(&mut rows, Row::Section, section);
            header(&mut rows, Row::Blank, "");
            current_section = Some(section);
            current_group = None;
        }
        if current_group != Some(group) {
            if current_group.is_some() {
                header(&mut rows, Row::Blank, "");
            }
            if group != section {
                header(&mut rows, Row::Group, group);
                header(&mut rows, Row::Blank, "");
            }
            current_group = Some(group);
        }
        rows.push(ListItem::new(Row::Model(index), ""));
    }
    rows
}

fn entry_index(row: Row) -> usize {
    match row {
        Row::Model(index) => index,
        _ => unreachable!("header rows are disabled"),
    }
}

pub(super) fn declare(ctx: &mut DeclareCtx<'_, State, Msg>, body: Rect) {
    let list = List::new(rows(&ctx.state().entries))
        .item_focus(
            |state: &State| state.selected.map(Row::Model),
            |row, _| Msg::Select(entry_index(row)),
        )
        .selection(
            |state: &State| state.selected.map(Row::Model),
            |row| Msg::Activate(entry_index(row)),
        )
        // Headers are disabled only so the cursor skips them; they keep the
        // list's backdrop instead of the dimmed disabled fill. That fill is
        // fixed, so hovering must not shift the backdrop either.
        .style(|theme| {
            let style = ListStyle::from_theme(theme);
            ListStyle {
                hovered_background: style.focused_background,
                disabled_background: style.focused_background,
                ..style
            }
        })
        .paint_item(|state: &State, row| match row.value {
            Row::Blank(_) => Text::default(),
            Row::Section(_) => header(row.label, Color::Green),
            Row::Group(_) => header(row.label, Color::Magenta),
            Row::Model(index) => model_row(&state.entries[*index], row.selected),
        });
    ctx.component("models", list, body);
}

fn header(label: &str, background: Color) -> Text<'static> {
    Text::from(Span::styled(
        format!(" {label} "),
        Style::default().fg(Color::Black).bg(background),
    ))
}

/// Text without a style of its own takes the list's row colors.
fn model_row(entry: &LocalModelEntry, is_selected: bool) -> Text<'static> {
    let active_marker = if entry.is_active { "◉" } else { "○" };
    let description = entry.description.trim();

    let mut spans = vec![Span::raw(format!("{active_marker} "))];

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
        spans.push(Span::raw(" "));
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
        spans.push(Span::raw(" "));
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

    spans.push(Span::raw(details));

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

    /// Headers separate sections and provider groups, but only model rows are
    /// enabled, so the cursor skips headers and the selection can never land
    /// on one.
    #[test]
    fn headers_separate_sections_and_groups_and_only_models_are_selectable() {
        let entries = [
            entry("http", "custom", Some("Custom models")),
            entry("openai", "cloud", Some("OpenAI")),
            entry("whisper", "local", None),
        ];
        let layout: Vec<String> = rows(&entries)
            .into_iter()
            .map(|item| {
                let row = match item.value() {
                    Row::Blank(_) => String::new(),
                    Row::Section(_) => format!("section {}", item.label()),
                    Row::Group(_) => format!("group {}", item.label()),
                    Row::Model(index) => format!("model {}", entries[*index].id),
                };
                if item.is_disabled() {
                    row
                } else {
                    format!("{row} (selectable)")
                }
            })
            .collect();
        assert_eq!(
            layout,
            [
                "section Custom models",
                "",
                "model custom (selectable)",
                "",
                "section Cloud models",
                "",
                "group OpenAI",
                "",
                "model cloud (selectable)",
                "",
                "section Local models",
                "",
                "model local (selectable)",
            ]
        );
    }

    /// The list identifies rows by value, so a group heading two separate runs
    /// must not produce two rows that answer to the same identity.
    #[test]
    fn a_group_heading_two_runs_still_gives_every_row_its_own_value() {
        let entries = [
            entry("openai", "a1", Some("A")),
            entry("openai", "b1", Some("B")),
            entry("openai", "a2", Some("A")),
        ];
        let rows = rows(&entries);
        for (i, first) in rows.iter().enumerate() {
            for second in &rows[i + 1..] {
                assert!(first.value() != second.value());
            }
        }
    }
}
