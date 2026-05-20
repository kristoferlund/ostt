use crate::config::{self, SelectedModel};
use crate::model::UserQuit;
use crate::transcription::{TranscriptionModel, TranscriptionProvider};
use crate::ui::{render_toast, Toast};
use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::HashSet;
use std::io::Stdout;
use std::time::Duration;

use super::{is_ctrl_c, render_footer, render_shell, render_title};

#[derive(Debug, Clone)]
pub(crate) struct CloudModelEntry {
    pub(crate) provider_id: String,
    pub(crate) model_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) languages: Vec<String>,
    pub(crate) is_active: bool,
    pub(crate) model: TranscriptionModel,
}

#[derive(Debug, Clone)]
pub(crate) struct CloudProviderSection {
    pub(crate) title: String,
    pub(crate) models: Vec<CloudModelEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloudModelMode {
    Browse,
    Info,
}

pub(crate) async fn run_cloud_model_selector(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    let authorized_provider_ids = config::get_authorized_providers()?;

    if authorized_provider_ids.is_empty() {
        show_no_cloud_providers_screen(terminal).await?;
        return Ok(());
    }

    let selected_model = config::get_selected_model_entry()?;
    let mut sections =
        build_cloud_provider_sections(&authorized_provider_ids, selected_model.as_ref());
    let model_count = cloud_model_count(&sections);

    if model_count == 0 {
        show_no_cloud_providers_screen(terminal).await?;
        return Ok(());
    }

    let mut toast: Option<Toast> = None;
    let mut selected = active_cloud_model_index(&sections).unwrap_or(0);
    let mut mode = CloudModelMode::Browse;

    loop {
        if toast.as_ref().is_some_and(Toast::is_expired) {
            toast = None;
        }

        terminal.draw(|frame| {
            let [body_area, footer_area, _] = render_shell(frame);
            match mode {
                CloudModelMode::Browse => {
                    let list_area = render_title(frame, body_area, "Cloud Model");
                    let (items, selected_display_index) =
                        cloud_model_list_items(&sections, selected);
                    let mut state = ListState::default().with_selected(selected_display_index);
                    frame.render_stateful_widget(
                        List::new(items)
                            .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray)),
                        list_area,
                        &mut state,
                    );
                    render_footer(
                        frame,
                        footer_area,
                        "↑↓ select, ↵ activate, i info, esc/q back",
                    );
                }
                CloudModelMode::Info => {
                    if let Some(entry) = cloud_model_at(&sections, selected) {
                        render_cloud_model_info(frame, body_area, entry);
                    }
                    render_footer(frame, footer_area, "esc/q back");
                }
            }

            if let Some(toast) = &toast {
                render_toast(frame, toast);
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if mode == CloudModelMode::Info {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => mode = CloudModelMode::Browse,
                        _ if is_ctrl_c(&key) => return Err(UserQuit.into()),
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(model_count.saturating_sub(1)),
                    KeyCode::Enter => {
                        if let Some(entry) = cloud_model_at(&sections, selected).cloned() {
                            save_cloud_selection(&entry.provider_id, &entry.model)?;
                            mark_active_cloud_model(
                                &mut sections,
                                &entry.provider_id,
                                &entry.model_id,
                            );
                            toast = Some(Toast::success(format!("Activated {}", entry.name)));
                        }
                    }
                    KeyCode::Char('i') => mode = CloudModelMode::Info,
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ if is_ctrl_c(&key) => return Err(UserQuit.into()),
                    _ => {}
                }
            }
        }
    }
}

async fn show_no_cloud_providers_screen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            let [body_area, footer_area, _] = render_shell(frame);
            let content_area = render_title(frame, body_area, "Cloud Provider");

            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        "No cloud providers authenticated.",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("Run `ostt auth login` to add credentials."),
                ])
                .wrap(Wrap { trim: false }),
                content_area,
            );

            render_footer(frame, footer_area, "esc/q back");
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ if is_ctrl_c(&key) => return Err(UserQuit.into()),
                    _ => {}
                }
            }
        }
    }
}

pub(crate) fn build_cloud_provider_sections(
    authorized_provider_ids: &[String],
    selected_model: Option<&SelectedModel>,
) -> Vec<CloudProviderSection> {
    let authorized: HashSet<&str> = authorized_provider_ids.iter().map(String::as_str).collect();

    TranscriptionProvider::all()
        .iter()
        .filter(|provider| **provider != TranscriptionProvider::Local)
        .filter(|provider| authorized.contains(provider.id()))
        .filter_map(|provider| {
            let models: Vec<CloudModelEntry> = TranscriptionModel::models_for_provider(provider)
                .into_iter()
                .map(|model| CloudModelEntry {
                    provider_id: provider.id().to_string(),
                    model_id: model.id().to_string(),
                    name: model.name().to_string(),
                    description: model.detailed_description().to_string(),
                    languages: model
                        .languages()
                        .iter()
                        .map(|language| language.to_string())
                        .collect(),
                    is_active: selected_model
                        .map(|selected| {
                            selected.provider_id == provider.id() && selected.model_id == model.id()
                        })
                        .unwrap_or(false),
                    model,
                })
                .collect();

            (!models.is_empty()).then(|| CloudProviderSection {
                title: provider.name().to_string(),
                models,
            })
        })
        .collect()
}

pub(crate) fn save_cloud_selection(
    provider_id: &str,
    model: &TranscriptionModel,
) -> anyhow::Result<()> {
    config::save_selected_model(provider_id, model.id())
}

fn render_cloud_model_info(frame: &mut Frame<'_>, area: Rect, entry: &CloudModelEntry) {
    let content_area = render_title(frame, area, "Cloud Model Info");
    let lines = vec![
        Line::from(format!("ID: {}", entry.model_id)),
        Line::from(format!("Name: {}", entry.name)),
        Line::from(""),
        Line::from(entry.description.clone()),
        Line::from(""),
        Line::from(format!(
            "Languages: {}",
            if entry.languages.is_empty() {
                "unknown".to_string()
            } else {
                entry.languages.join(", ")
            }
        )),
        Line::from(format!(
            "Active: {}",
            if entry.is_active { "Yes" } else { "No" }
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        content_area,
    );
}

fn cloud_model_list_item(entry: &CloudModelEntry) -> ListItem<'static> {
    let marker = if entry.is_active { "◉" } else { "○" };
    ListItem::new(Line::from(format!("{marker} {}", entry.name)))
}

fn cloud_model_list_items(
    sections: &[CloudProviderSection],
    selected: usize,
) -> (Vec<ListItem<'static>>, Option<usize>) {
    let mut items = Vec::new();
    let mut display_index = 0_usize;
    let mut model_index = 0_usize;
    let mut selected_display_index = None;

    for section in sections {
        items.push(ListItem::new(Line::from(Span::styled(
            section.title.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ))));
        display_index += 1;

        for entry in &section.models {
            if model_index == selected {
                selected_display_index = Some(display_index);
            }
            items.push(cloud_model_list_item(entry));
            display_index += 1;
            model_index += 1;
        }

        items.push(ListItem::new(Line::from("")));
        display_index += 1;
    }

    (items, selected_display_index)
}

fn cloud_model_count(sections: &[CloudProviderSection]) -> usize {
    sections.iter().map(|section| section.models.len()).sum()
}

fn cloud_model_at(sections: &[CloudProviderSection], index: usize) -> Option<&CloudModelEntry> {
    sections
        .iter()
        .flat_map(|section| section.models.iter())
        .nth(index)
}

fn active_cloud_model_index(sections: &[CloudProviderSection]) -> Option<usize> {
    sections
        .iter()
        .flat_map(|section| section.models.iter())
        .position(|entry| entry.is_active)
}

fn mark_active_cloud_model(
    sections: &mut [CloudProviderSection],
    provider_id: &str,
    model_id: &str,
) {
    for entry in sections
        .iter_mut()
        .flat_map(|section| section.models.iter_mut())
    {
        entry.is_active = entry.provider_id == provider_id && entry.model_id == model_id;
    }
}
