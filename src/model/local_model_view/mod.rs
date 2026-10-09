pub(crate) mod custom_model_details_dialog;
pub(crate) mod custom_model_url_input_dialog;
pub(crate) mod local_model_delete_confirmation_dialog;
pub(crate) mod local_model_download_confirmation_dialog;
pub(crate) mod local_model_download_progress_dialog;
pub(crate) mod local_model_info_view;
pub(crate) mod local_model_list_view;
pub(crate) mod local_model_view_helpers;
pub(crate) mod types;

use ratatui::Frame;
use ratcn::runtime::{CellOffset, Event, EventResult, FocusState, KeyCode, KeyEvent, Ratcn};
use ratcn::InputState as Input;
use ratcn::{terminal::Session, Button, Dialog, Theme};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::{self, SelectedModel};
use crate::model::UserQuit;
use crate::transcription::local_models::{
    delete_model, download_model_with_handle, fetch_registry, is_safe_model_id, load_state,
    mark_downloaded_registry_model, model_destination, register_downloaded_custom_model,
    resolve_custom_model, validate_custom_model_registration, validate_downloaded_model,
    DownloadHandle, LocalModelState, RegistryEntry,
};
use crate::transcription::{self, TranscriptionProvider};
use crate::ui::components::modal::dialog;
use crate::ui::{
    render_app_layout, render_themed_footer, render_themed_title, render_toast, session, Toast,
};

use custom_model_details_dialog::CustomModelDetailsDialog;
use custom_model_url_input_dialog::CustomModelUrlInputDialog;
use local_model_delete_confirmation_dialog::LocalModelDeleteConfirmationDialog;
use local_model_download_confirmation_dialog::LocalModelDownloadConfirmationDialog;
use local_model_download_progress_dialog::LocalModelDownloadProgressDialog;
use local_model_info_view::LocalModelInfoView;
use local_model_list_view::LocalModelListView;
use types::{DownloadState, LocalModelEntry, LocalModelsMode, LocalModelsTui, RunningDownload};

pub(super) enum Msg {
    Focus(FocusState),
    Move(CellOffset),
    Url(Input),
    Id(Input),
    Name(Input),
    Accept,
    Dismiss,
    Quit,
    Previous,
    Next,
    Info,
    Custom,
    Delete,
}

fn runtime() -> Ratcn<LocalModelsTui, Msg> {
    Ratcn::new()
        .focus(|state: &LocalModelsTui| &state.focus, Msg::Focus)
        .modals(|state| &state.modals)
}

fn model_dialog(
    title: impl Into<String>,
    description: impl Into<String>,
    action: &'static str,
    offset: CellOffset,
) -> Dialog<LocalModelsTui, Msg> {
    dialog(
        title,
        description,
        Button::new(action).on_press(|| Msg::Accept),
    )
    .offset(offset)
    .on_offset_change(Msg::Move)
    .on_dismiss(|| Msg::Dismiss)
}

pub(crate) async fn run(session: &mut Session) -> anyhow::Result<()> {
    let local_state = load_state();
    let ostt_config =
        config::OsttConfig::load().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let authorized_provider_ids = config::get_authorized_providers()?;
    let registry = match fetch_registry().await {
        Ok(registry) => registry,
        Err(error) => {
            tracing::error!("Failed to fetch local model registry: {}", error);
            Vec::new()
        }
    };
    let selected_model = crate::config::get_selected_model_entry()?;

    // Probe daemon once at open to show initial loaded status.
    let daemon_model_id = crate::transcription::daemon_client::probe_daemon()
        .await
        .map(|d| d.model_id);

    let entries = build_model_entries(
        &ostt_config,
        &authorized_provider_ids,
        &local_state,
        &registry,
        selected_model.as_ref(),
        daemon_model_id.as_deref(),
    );
    let mut tui =
        types::LocalModelsTui::new(entries.clone(), downloaded_model_disk_usage_bytes(&entries));
    tui.daemon_model_id = daemon_model_id;

    tracing::debug!(
        "Local model view opened with {} registry models and {} custom models",
        registry.len(),
        local_state.custom_models.len()
    );
    if registry.is_empty() {
        tracing::debug!("Local model registry unavailable; custom model entry remains enabled");
        tui.show_error_dialog(
            "Could not load remote registry; custom URL entry is still available with [c]"
                .to_string(),
        );
    }
    let mut running_download: Option<types::RunningDownload> = None;
    tui.sync_modal()?;
    let mut ratcn = runtime();
    let mut last_daemon_probe = std::time::Instant::now()
        .checked_sub(Duration::from_secs(5))
        .unwrap_or_else(std::time::Instant::now);

    loop {
        if tui.toast.as_ref().is_some_and(Toast::is_expired) {
            tui.toast = None;
        }
        finish_completed_download(&mut tui, &registry, &mut running_download).await?;
        tui.sync_modal()?;

        // Re-probe daemon every 2 seconds to pick up background daemon startup.
        if last_daemon_probe.elapsed() >= Duration::from_secs(2) {
            let info = crate::transcription::daemon_client::probe_daemon().await;
            tui.update_daemon_status(info.as_ref().map(|d| d.model_id.as_str()));
            last_daemon_probe = std::time::Instant::now();
        }

        let theme = session::theme(session);
        session
            .terminal_mut()
            .draw(|frame| render_local_models(frame, &mut tui, &mut ratcn, &theme))?;
        session.set_pointer_shape(ratcn.pointer_shape())?;

        let Some(event) = session::next(session, Some(Duration::from_millis(100)))? else {
            continue;
        };
        // Keep confirmation/cancellation shortcuts; ordinary keys go through ratcn first.
        let legacy_key = matches!(&event, Event::Key(key) if (matches!(tui.mode, LocalModelsMode::ConfirmDownload { .. } | LocalModelsMode::ConfirmDelete { .. })
                && matches!(key.code, KeyCode::Char('y' | 'Y' | 'n' | 'N')))
            || (matches!(tui.mode, LocalModelsMode::Downloading(_)) && key.code == KeyCode::Tab));
        let message = if legacy_key {
            match event {
                Event::Key(key) => shortcut(&tui.mode, key),
                _ => None,
            }
        } else {
            let result = ratcn.handle_event(event.clone(), &tui);
            if let Some(text) = ratcn.take_clipboard() {
                session.set_clipboard(&text)?;
            } else if session::is_ctrl_c(&event) {
                return Err(UserQuit.into());
            }
            match result {
                EventResult::Emit(msg) => Some(msg),
                EventResult::Consumed => None,
                EventResult::Ignored => match event {
                    Event::Key(key) => shortcut(&tui.mode, key),
                    _ => None,
                },
            }
        };
        if let Some(msg) = message {
            if update(&mut tui, &registry, &mut running_download, msg).await? {
                break;
            }
        }
    }
    Ok(())
}

fn render_local_models(
    frame: &mut Frame<'_>,
    tui: &mut LocalModelsTui,
    ratcn: &mut Ratcn<LocalModelsTui, Msg>,
    theme: &Theme,
) {
    session::paint_background(frame, theme);
    let layout = render_app_layout(frame, frame.area());
    let body = ratatui::layout::Rect {
        height: layout.title.height.saturating_add(layout.body.height),
        ..layout.title
    };
    LocalModelListView::prepare(tui, body.height);
    if let LocalModelsMode::Info { entry } = &tui.mode {
        render_themed_title(frame, layout.title, &entry.name, theme);
    }
    let area = frame.area();
    ratcn.render(frame, area, tui, theme, |ctx| {
        if matches!(tui.mode, LocalModelsMode::Info { .. }) {
            LocalModelInfoView::declare(ctx, layout.body);
        } else {
            LocalModelListView::declare(ctx, body);
        }
        let offset = tui.dialog_offset;
        match &tui.mode {
            LocalModelsMode::ConfirmDelete { entry } => ctx.modal(
                "delete",
                LocalModelDeleteConfirmationDialog::dialog(entry, offset),
                area,
            ),
            LocalModelsMode::ConfirmDownload { entry } => ctx.modal(
                "download",
                LocalModelDownloadConfirmationDialog::dialog(entry, offset),
                area,
            ),
            LocalModelsMode::CustomModelInput { .. } => {
                ctx.modal("url", CustomModelUrlInputDialog::dialog(offset), area)
            }
            LocalModelsMode::CustomModelDetails { .. } => {
                ctx.modal("details", CustomModelDetailsDialog::dialog(offset), area)
            }
            LocalModelsMode::Downloading(download) => ctx.modal(
                "progress",
                LocalModelDownloadProgressDialog::dialog(download, offset),
                area,
            ),
            LocalModelsMode::ErrorDialog { message, .. } => ctx.modal(
                "error",
                model_dialog("Error", message.clone(), "Close", offset),
                area,
            ),
            _ => {}
        }
    });
    render_themed_footer(
        frame,
        layout.footer,
        if matches!(tui.mode, LocalModelsMode::Info { .. }) {
            "esc/q back"
        } else {
            "↑↓ nav, ↵ activate/download, x/del delete, i info, c custom, esc/q back"
        },
        theme,
    );
    if let Some(toast) = &tui.toast {
        render_toast(frame, toast, theme);
    }
}

pub(crate) fn build_model_entries(
    config: &config::OsttConfig,
    authorized_provider_ids: &[String],
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
    daemon_model_id: Option<&str>,
) -> Vec<LocalModelEntry> {
    let mut entries = Vec::new();
    entries.extend(build_custom_model_entries(
        config,
        local_state,
        selected_model,
        daemon_model_id,
    ));
    entries.extend(build_cloud_model_entries(
        authorized_provider_ids,
        selected_model,
    ));
    entries.extend(build_local_model_entries(
        local_state,
        registry,
        selected_model,
        daemon_model_id,
    ));
    entries
}

fn build_custom_model_entries(
    config: &config::OsttConfig,
    local_state: &LocalModelState,
    selected_model: Option<&SelectedModel>,
    daemon_model_id: Option<&str>,
) -> Vec<LocalModelEntry> {
    let mut entries: Vec<LocalModelEntry> = local_state
        .custom_models
        .iter()
        .map(|entry| {
            local_model_entry_from_registry_entry(entry, &[], selected_model, daemon_model_id)
        })
        .map(|mut entry| {
            entry.group_id = Some("Custom models".to_string());
            entry
        })
        .collect();

    for provider_id in ["command", "http"] {
        let Some(provider_config) = config.provider_configs.get(provider_id) else {
            continue;
        };
        for (profile_id, profile) in &provider_config.models {
            let description = match provider_id {
                "command" => profile
                    .settings
                    .command
                    .clone()
                    .unwrap_or_else(|| "External command backend".to_string()),
                "http" => profile
                    .settings
                    .endpoint
                    .clone()
                    .unwrap_or_else(|| "OpenAI-compatible HTTP endpoint".to_string()),
                _ => String::new(),
            };
            entries.push(LocalModelEntry {
                id: profile_id.clone(),
                provider_id: provider_id.to_string(),
                name: profile
                    .settings
                    .display_name
                    .clone()
                    .unwrap_or_else(|| profile_id.clone()),
                description,
                size_mb: 0,
                is_downloaded: true,
                is_active: selected_model
                    .map(|selected| {
                        selected.provider_id == provider_id && selected.model_id == *profile_id
                    })
                    .unwrap_or(false),
                is_daemon_loaded: false,
                is_available_in_registry: false,
                languages: vec!["Configured".to_string()],
                url: String::new(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: Some("Custom models".to_string()),
            });
        }
    }

    entries
}

fn build_cloud_model_entries(
    authorized_provider_ids: &[String],
    selected_model: Option<&SelectedModel>,
) -> Vec<LocalModelEntry> {
    let authorized: std::collections::HashSet<&str> =
        authorized_provider_ids.iter().map(String::as_str).collect();

    TranscriptionProvider::all()
        .iter()
        .filter(|provider| provider.requires_auth())
        .filter(|provider| authorized.contains(provider.id()))
        .flat_map(|provider| {
            transcription::models_for_provider(provider)
                .into_iter()
                .map(move |model| (provider.name(), model))
        })
        .map(|(provider_name, model)| LocalModelEntry {
            id: model.model_id.to_string(),
            provider_id: model.provider_id.to_string(),
            name: model.display_name.to_string(),
            description: model.description.to_string(),
            size_mb: 0,
            is_downloaded: true,
            is_active: selected_model
                .map(|selected| {
                    selected.provider_id == model.provider_id && selected.model_id == model.model_id
                })
                .unwrap_or(false),
            is_daemon_loaded: false,
            is_available_in_registry: false,
            languages: model
                .languages
                .iter()
                .map(|language| language.to_string())
                .collect(),
            url: String::new(),
            recommended_hardware: None,
            category: None,
            sha256: None,
            group_id: Some(provider_name.to_string()),
        })
        .collect()
}

pub(crate) fn build_local_model_entries(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
    daemon_model_id: Option<&str>,
) -> Vec<LocalModelEntry> {
    let _ = local_state;
    registry
        .iter()
        .map(|entry| {
            local_model_entry_from_registry_entry(entry, registry, selected_model, daemon_model_id)
        })
        .collect()
}

fn local_model_entry_from_registry_entry(
    entry: &RegistryEntry,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
    daemon_model_id: Option<&str>,
) -> LocalModelEntry {
    let is_downloaded = model_destination(entry).exists();
    let is_active = selected_model
        .map(|selected| selected.provider_id == entry.provider_id && selected.model_id == entry.id)
        .unwrap_or(false);
    let is_daemon_loaded = daemon_model_id == Some(entry.id.as_str());

    LocalModelEntry {
        id: entry.id.clone(),
        provider_id: entry.provider_id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        size_mb: entry.size_mb,
        is_downloaded,
        is_active,
        is_daemon_loaded,
        is_available_in_registry: registry
            .iter()
            .any(|registry_entry| registry_entry.id == entry.id),
        languages: entry.languages.clone(),
        url: entry.url.clone(),
        recommended_hardware: entry.recommended_hardware.clone(),
        category: entry.category.clone(),
        sha256: entry.sha256.clone(),
        group_id: entry
            .group_id
            .clone()
            .or_else(|| Some("Local models".to_string())),
    }
}

fn downloaded_model_disk_usage_bytes(entries: &[LocalModelEntry]) -> u64 {
    entries
        .iter()
        .filter(|entry| entry.provider_id == "whisper")
        .filter(|entry| entry.is_downloaded)
        .filter_map(|entry| {
            model_destination(&registry_entry_from_model(entry))
                .metadata()
                .ok()
        })
        .map(|metadata| metadata.len())
        .sum()
}

fn shortcut(mode: &LocalModelsMode, key: KeyEvent) -> Option<Msg> {
    match (mode, key.code) {
        (LocalModelsMode::Browse, KeyCode::Char('q') | KeyCode::Esc) => Some(Msg::Quit),
        (LocalModelsMode::Browse, KeyCode::Down) => Some(Msg::Next),
        (LocalModelsMode::Browse, KeyCode::Up) => Some(Msg::Previous),
        (LocalModelsMode::Browse, KeyCode::Enter) => Some(Msg::Accept),
        (LocalModelsMode::Browse, KeyCode::Char('i')) => Some(Msg::Info),
        (LocalModelsMode::Browse, KeyCode::Char('c')) => Some(Msg::Custom),
        (LocalModelsMode::Browse, KeyCode::Char('x' | 'd') | KeyCode::Delete) => Some(Msg::Delete),
        (LocalModelsMode::Info { .. }, KeyCode::Esc | KeyCode::Char('q')) => Some(Msg::Dismiss),
        (
            LocalModelsMode::ConfirmDownload { .. } | LocalModelsMode::ConfirmDelete { .. },
            KeyCode::Char('y' | 'Y'),
        ) => Some(Msg::Accept),
        (
            LocalModelsMode::ConfirmDownload { .. } | LocalModelsMode::ConfirmDelete { .. },
            KeyCode::Char('n' | 'N'),
        ) => Some(Msg::Dismiss),
        (LocalModelsMode::Downloading(_), KeyCode::Tab) => Some(Msg::Dismiss),
        _ => None,
    }
}

async fn update(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    running_download: &mut Option<RunningDownload>,
    msg: Msg,
) -> anyhow::Result<bool> {
    match msg {
        Msg::Focus(focus) => tui.focus = focus,
        Msg::Move(offset) => tui.dialog_offset = offset,
        Msg::Url(value) => {
            if let LocalModelsMode::CustomModelInput { input } = &mut tui.mode {
                *input = value;
            }
        }
        Msg::Id(value) => {
            if let LocalModelsMode::CustomModelDetails { id_input, .. } = &mut tui.mode {
                *id_input = value;
            }
        }
        Msg::Name(value) => {
            if let LocalModelsMode::CustomModelDetails { name_input, .. } = &mut tui.mode {
                *name_input = value;
            }
        }
        Msg::Quit => return Ok(true),
        Msg::Previous => tui.move_selection_up(),
        Msg::Next => tui.move_selection_down(),
        Msg::Info => {
            tracing::debug!("Opening local model info view");
            tui.show_info();
        }
        Msg::Custom => {
            tracing::debug!("Opening custom local model input dialog");
            tui.toast = None;
            tui.show_custom_input();
        }
        Msg::Delete => tui.confirm_delete(),
        Msg::Accept => match tui.mode.clone() {
            LocalModelsMode::Browse => handle_selected_entry(tui, registry).await?,
            LocalModelsMode::CustomModelInput { input } => {
                resolve_custom_input(tui, input.value()).await
            }
            LocalModelsMode::CustomModelDetails { .. } => {
                start_custom_details_download(tui, running_download)
            }
            LocalModelsMode::ConfirmDownload { entry } => {
                start_confirmed_download(tui, running_download, &entry)
            }
            LocalModelsMode::ConfirmDelete { entry } => {
                delete_confirmed_entry(tui, registry, &entry)?
            }
            LocalModelsMode::Downloading(_) => cancel_download(running_download),
            LocalModelsMode::ErrorDialog { .. } => tui.close_error_dialog(),
            LocalModelsMode::Info { .. } => {}
        },
        Msg::Dismiss => match tui.mode {
            LocalModelsMode::Downloading(_) => cancel_download(running_download),
            LocalModelsMode::ErrorDialog { .. } => tui.close_error_dialog(),
            _ => tui.back_to_browse(),
        },
    }

    tui.sync_modal()?;
    Ok(false)
}

async fn handle_selected_entry(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
) -> anyhow::Result<()> {
    let Some(entry) = tui.selected_entry().cloned() else {
        return Ok(());
    };
    tracing::debug!("Selected local model '{}'", entry.id);

    if entry.provider_id == "whisper" && !entry.is_downloaded {
        if entry.is_available_in_registry {
            tracing::debug!("Confirming download for local model '{}'", entry.id);
            tui.mode = LocalModelsMode::ConfirmDownload { entry };
        } else {
            tui.show_error_dialog("Custom models must be added through [c]".to_string());
        }
        return Ok(());
    }

    match activate_entry(&entry).await {
        Ok(()) => {
            tracing::info!("Activated local model '{}'", entry.id);
            tui.toast = Some(Toast::success(format!("Activated {}", entry.name)));
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => {
            tracing::error!("Failed to activate local model '{}': {}", entry.id, error);
            tui.toast = Some(Toast::error(error.to_string()));
        }
    }
    Ok(())
}

async fn activate_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if entry.provider_id != "whisper" {
        config::save_selected_model(&entry.provider_id, &entry.id)?;
        return Ok(());
    }

    if !entry.is_downloaded {
        anyhow::bail!("Download first with [d]");
    }
    let path = model_destination(&registry_entry_from_model(entry));
    if !path.exists() {
        anyhow::bail!("Download first with [d]");
    }
    config::save_selected_model(&entry.provider_id, &entry.id)?;
    reload_daemon_if_running(&entry.id).await?;
    Ok(())
}

async fn reload_daemon_if_running(model_id: &str) -> anyhow::Result<()> {
    let Some(info) = crate::transcription::daemon_client::probe_daemon().await else {
        tracing::debug!(
            "No local daemon running; selected model '{model_id}' will load in-process"
        );
        return Ok(());
    };

    if info.model_id == model_id {
        tracing::info!("Local daemon already loaded with activated model '{model_id}'");
        return Ok(());
    }

    tracing::info!(
        "Reloading local daemon after model activation: '{}' -> '{}'",
        info.model_id,
        model_id
    );
    crate::transcription::daemon_client::ensure_daemon(model_id, None).await
}

fn start_confirmed_download(
    tui: &mut LocalModelsTui,
    running_download: &mut Option<RunningDownload>,
    entry: &LocalModelEntry,
) {
    tracing::info!("Starting download for local model '{}'", entry.id);
    let running = start_download(registry_entry_from_model(entry), false);
    sync_download_progress(tui, &running);
    *running_download = Some(running);
}

fn start_download(entry: RegistryEntry, is_custom: bool) -> RunningDownload {
    let state = Arc::new(Mutex::new(initial_download_state(&entry, is_custom)));
    let progress_state = state.clone();
    let handle = DownloadHandle::new();
    let task_handle = handle.clone();
    let task_entry = entry.clone();
    tracing::debug!("Spawning local model download task for '{}'", entry.id);
    let task = tokio::spawn(async move {
        let destination = model_destination(&task_entry);
        // Progress is shared with the TUI loop through a small mutex-protected snapshot.
        if let Err(error) = download_model_with_handle(
            &task_entry.url,
            &destination,
            Some(Box::new(
                move |downloaded_bytes, total_bytes, speed_mbps| {
                    if let Ok(mut state) = progress_state.lock() {
                        state.downloaded_bytes = downloaded_bytes;
                        state.total_bytes = total_bytes;
                        state.speed_mbps = speed_mbps;
                        state.progress = if total_bytes > 0 {
                            downloaded_bytes as f64 / total_bytes as f64
                        } else {
                            0.0
                        };
                        state.status = "Downloading".to_string();
                    }
                },
            )),
            Some(task_handle),
        )
        .await
        {
            tracing::error!(
                "Failed to download local model '{}' from '{}': {}",
                task_entry.id,
                task_entry.url,
                error
            );
            return Err(error);
        }
        validate_downloaded_model(&task_entry)?;
        if is_custom {
            tracing::info!("Registered downloaded custom model '{}'", task_entry.id);
            register_downloaded_custom_model(task_entry)?;
        } else {
            tracing::info!("Marked registry model '{}' as downloaded", task_entry.id);
            mark_downloaded_registry_model(&task_entry)?;
        }
        Ok(())
    });

    RunningDownload {
        state,
        handle,
        task,
    }
}

fn initial_download_state(entry: &RegistryEntry, is_custom: bool) -> DownloadState {
    DownloadState {
        model_id: entry.id.clone(),
        provider_id: entry.provider_id.clone(),
        downloaded_bytes: 0,
        total_bytes: u64::from(entry.size_mb) * 1024 * 1024,
        progress: 0.0,
        speed_mbps: 0.0,
        status: "Starting download".to_string(),
        is_complete: false,
        is_custom,
    }
}

fn cancel_download(running_download: &mut Option<RunningDownload>) {
    if let Some(running) = running_download.as_ref() {
        tracing::info!("Cancelling local model download");
        running.handle.cancel();
        if let Ok(mut state) = running.state.lock() {
            state.status = "Cancelling download".to_string();
        }
    }
}

async fn resolve_custom_input(tui: &mut LocalModelsTui, value: &str) {
    tracing::debug!("Resolving custom local model input");
    match resolve_custom_model(value).await {
        Ok(entry) => {
            tracing::debug!("Resolved custom local model '{}'", entry.id);
            tui.toast = None;
            tui.mode = LocalModelsMode::CustomModelDetails {
                id_input: Input::new(entry.id.clone()),
                name_input: Input::new(entry.name.clone()),
                resolved_entry: entry,
            };
        }
        Err(error) => {
            tracing::error!("Failed to resolve custom local model input: {}", error);
            tui.toast = Some(Toast::error(error.to_string()));
        }
    }
}

fn start_custom_details_download(
    tui: &mut LocalModelsTui,
    running_download: &mut Option<RunningDownload>,
) {
    let LocalModelsMode::CustomModelDetails {
        mut resolved_entry,
        id_input,
        name_input,
        ..
    } = tui.mode.clone()
    else {
        return;
    };
    let id = id_input.value().trim();
    let name = name_input.value().trim();
    if !is_safe_model_id(id) {
        tui.toast = Some(Toast::error(
            "Model ID must use lowercase letters, numbers, '.', '_' or '-'",
        ));
        return;
    }
    if tui.entries.iter().any(|entry| entry.id == id) {
        tui.toast = Some(Toast::error(format!("Model ID '{id}' already exists")));
        return;
    }
    if name.is_empty() {
        tui.toast = Some(Toast::error("Model name is required"));
        return;
    }
    resolved_entry.id = id.to_string();
    resolved_entry.name = name.to_string();
    if let Err(error) = validate_custom_model_registration(&resolved_entry) {
        tui.toast = Some(Toast::error(error.to_string()));
        return;
    }
    tracing::info!("Starting download for custom local model '{id}'");
    let running = start_download(resolved_entry, true);
    sync_download_progress(tui, &running);
    *running_download = Some(running);
}

fn delete_confirmed_entry(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    entry: &LocalModelEntry,
) -> anyhow::Result<()> {
    match delete_entry(entry) {
        Ok(()) => {
            tracing::info!("Deleted local model '{}'", entry.id);
            if entry.is_daemon_loaded {
                stop_daemon_for_deleted_model(&entry.id);
            }
            tui.toast = Some(Toast::success(format!("Deleted {}", entry.name)));
            tui.back_to_browse();
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => {
            tracing::error!("Failed to delete local model '{}': {}", entry.id, error);
            tui.back_to_browse();
            tui.show_error_dialog(error.to_string());
        }
    }
    Ok(())
}

fn stop_daemon_for_deleted_model(model_id: &str) {
    #[cfg(unix)]
    {
        let model_id = model_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = crate::transcription::daemon_client::shutdown_daemon().await {
                tracing::warn!("could not stop daemon after deleting model '{model_id}': {e}");
            } else {
                tracing::info!("daemon stopped after deleting model '{model_id}'");
            }
        });
    }
}

fn delete_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if delete_model(&entry.id).is_ok() {
        return Ok(());
    }

    let path = model_destination(&registry_entry_from_model(entry));
    std::fs::remove_file(&path)?;
    if config::get_selected_model_entry()?.is_some_and(|selected| {
        selected.provider_id == entry.provider_id && selected.model_id == entry.id
    }) {
        config::clear_selected_model()?;
    }
    Ok(())
}

async fn finish_completed_download(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    running_download: &mut Option<RunningDownload>,
) -> anyhow::Result<()> {
    let Some(running) = running_download.as_ref() else {
        return Ok(());
    };

    sync_download_progress(tui, running);
    if !running.task.is_finished() {
        return Ok(());
    }

    let running = running_download.take().expect("running download");
    match running.task.await? {
        Ok(()) => {
            tracing::info!("Local model download completed");
            tui.back_to_browse();
            tui.refresh(&load_state(), registry)?;
            tui.toast = Some(Toast::success("Download complete"));
        }
        Err(error) => {
            if error.to_string() != "model download cancelled" {
                tracing::error!("Local model download failed: {}", error);
                tui.back_to_browse();
                tui.show_error_dialog(error.to_string());
            } else {
                tracing::debug!("Local model download cancelled");
                tui.back_to_browse();
            }
        }
    }
    Ok(())
}

fn sync_download_progress(tui: &mut LocalModelsTui, running: &RunningDownload) {
    if let Ok(state) = running.state.lock() {
        tui.mode = LocalModelsMode::Downloading(state.clone());
    }
}

fn registry_entry_from_model(entry: &LocalModelEntry) -> RegistryEntry {
    RegistryEntry {
        id: entry.id.clone(),
        provider_id: entry.provider_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::types::LocalModelEntry;
    use super::*;
    use crate::config::SelectedModel;
    use crate::transcription::local_models::{
        model_files_dir, set_test_models_dir, LocalModelState, RegistryEntry, TEST_ENV_LOCK,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn with_isolated_models_dir(test: impl FnOnce(PathBuf)) {
        let _guard = test_env_lock();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-models-tui-test-{unique}"));
        let models_dir = dir.join("models");
        set_test_models_dir(Some(models_dir.clone()));
        std::env::set_var("HOME", &dir);
        std::env::set_var("XDG_CONFIG_HOME", dir.join(".config"));

        test(models_dir);

        set_test_models_dir(None);
        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(previous_xdg_config_home) = previous_xdg_config_home {
            std::env::set_var("XDG_CONFIG_HOME", previous_xdg_config_home);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let _ = fs::remove_dir_all(dir);
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            provider_id: "whisper".to_string(),
            name: format!("{id} model"),
            description: "Test model".to_string(),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: Some("cpu".to_string()),
            sha256: None,
            category: None,
            group_id: None,
        }
    }

    #[tokio::test]
    async fn model_messages_transition_modal_state_before_the_next_frame() {
        let mut tui = LocalModelsTui::new(Vec::new(), 0);
        tui.focus = FocusState::intent(["models"]);
        let mut running = None;
        update(&mut tui, &[], &mut running, Msg::Custom)
            .await
            .unwrap();
        assert_eq!(tui.modals.top().map(|id| id.as_str()), Some("url"));
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|frame| render_local_models(frame, &mut tui, &mut ratcn, &Theme::default_dark()))
            .unwrap();
        let EventResult::Emit(msg @ Msg::Url(_)) = ratcn.handle_event(KeyCode::Char('q'), &tui)
        else {
            panic!("q must remain valid URL/name text")
        };
        update(&mut tui, &[], &mut running, msg).await.unwrap();
        let LocalModelsMode::CustomModelInput { input } = &tui.mode else {
            panic!("typing must not leave input mode")
        };
        assert_eq!(input.value(), "q");
        update(&mut tui, &[], &mut running, Msg::Dismiss)
            .await
            .unwrap();
        assert!(tui.modals.top().is_none());
        assert_eq!(tui.focus, FocusState::intent(["models"]));
        assert!(
            matches!(
                ratcn.handle_event(KeyCode::Enter, &tui),
                EventResult::Consumed
            ),
            "closing before redraw cannot access fields from the old mode"
        );
    }

    #[test]
    fn model_information_and_forms_share_the_same_runtime_and_active_palette() {
        let entry =
            local_model_entry_from_registry_entry(&registry_entry("small"), &[], None, None);
        let mut tui = LocalModelsTui::new(vec![entry.clone()], 0);
        let mut ratcn = runtime();
        let theme = Theme::adaptive(
            ratatui::style::Color::Rgb(253, 246, 227),
            ratatui::style::Color::Rgb(101, 123, 131),
            None,
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        tui.mode = LocalModelsMode::Info { entry };
        terminal
            .draw(|frame| render_local_models(frame, &mut tui, &mut ratcn, &theme))
            .unwrap();
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(screen.contains("ID: whisper/small"));
        assert_eq!(terminal.backend().buffer()[(99, 0)].bg, theme.background);
        tui.show_custom_input();
        tui.sync_modal().unwrap();
        terminal
            .draw(|frame| render_local_models(frame, &mut tui, &mut ratcn, &theme))
            .unwrap();
        assert!(terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.bg == theme.surface));
        assert!(matches!(
            ratcn.handle_event(Event::Paste("https://example.com/model.bin".into()), &tui),
            EventResult::Emit(Msg::Url(_))
        ));
    }

    #[test]
    fn model_picker_opens_on_active_model() {
        let entries = vec![
            LocalModelEntry {
                id: "whisper-1".to_string(),
                provider_id: "openai".to_string(),
                name: "Whisper".to_string(),
                description: String::new(),
                size_mb: 0,
                is_downloaded: true,
                is_active: false,
                is_daemon_loaded: false,
                is_available_in_registry: false,
                languages: Vec::new(),
                url: String::new(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: Some("OpenAI".to_string()),
            },
            LocalModelEntry {
                id: "base".to_string(),
                provider_id: "whisper".to_string(),
                name: "Base".to_string(),
                description: String::new(),
                size_mb: 0,
                is_downloaded: true,
                is_active: true,
                is_daemon_loaded: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: String::new(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: Some("Local models".to_string()),
            },
        ];

        let tui = types::LocalModelsTui::new(entries, 0);

        assert_eq!(
            tui.selected_entry().map(|entry| entry.id.as_str()),
            Some("base")
        );
    }

    #[test]
    fn build_model_entries_orders_custom_before_local_entries() {
        with_isolated_models_dir(|_| {
            let registry = vec![RegistryEntry {
                group_id: Some("nbailab".to_string()),
                ..registry_entry("turbo")
            }];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![RegistryEntry {
                    category: Some("custom".to_string()),
                    group_id: Some("Custom".to_string()),
                    ..registry_entry("custom")
                }],
            };
            let config = config::OsttConfig::default();

            let entries = build_model_entries(&config, &[], &state, &registry, None, None);

            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].id, "custom");
            assert_eq!(entries[0].group_id.as_deref(), Some("Custom models"));
            assert_eq!(entries[1].id, "turbo");
            assert_eq!(entries[1].group_id.as_deref(), Some("nbailab"));
            assert!(entries.iter().any(|entry| {
                entry.id == "turbo" && entry.is_available_in_registry && !entry.is_downloaded
            }));
            assert!(entries.iter().any(|entry| {
                entry.id == "custom"
                    && !entry.is_available_in_registry
                    && entry.category.as_deref() == Some("custom")
            }));
        });
    }

    #[test]
    fn build_model_entries_groups_cloud_models_by_provider_name() {
        with_isolated_models_dir(|_| {
            let config = config::OsttConfig::default();
            let entries = build_model_entries(
                &config,
                &["openai".to_string(), "deepgram".to_string()],
                &LocalModelState::default(),
                &[],
                None,
                None,
            );

            assert!(entries.iter().any(|entry| {
                entry.provider_id == "openai" && entry.group_id.as_deref() == Some("OpenAI")
            }));
            assert!(entries.iter().any(|entry| {
                entry.provider_id == "deepgram" && entry.group_id.as_deref() == Some("Deepgram")
            }));
        });
    }

    #[test]
    fn build_local_model_entries_marks_downloaded_and_active_from_filesystem_and_selection() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let selected = SelectedModel {
                provider_id: "whisper".to_string(),
                model_id: "turbo".to_string(),
            };

            let entries = build_local_model_entries(
                &LocalModelState::default(),
                &registry,
                Some(&selected),
                None,
            );

            assert_eq!(entries.len(), 1);
            assert!(entries[0].is_downloaded);
            assert!(entries[0].is_active);
        });
    }

    #[test]
    fn disk_usage_sums_downloaded_model_files_only() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo"), registry_entry("base")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let entries =
                build_local_model_entries(&LocalModelState::default(), &registry, None, None);

            assert_eq!(downloaded_model_disk_usage_bytes(&entries), 3);
        });
    }

    #[test]
    fn confirm_delete_only_opens_for_downloaded_models() {
        let mut entries = vec![LocalModelEntry {
            id: "missing".to_string(),
            provider_id: "whisper".to_string(),
            name: "Missing".to_string(),
            description: String::new(),
            size_mb: 1,
            is_downloaded: false,
            is_active: false,
            is_daemon_loaded: false,
            is_available_in_registry: true,
            languages: Vec::new(),
            url: "https://example.com/missing.bin".to_string(),
            recommended_hardware: None,
            category: None,
            sha256: None,
            group_id: None,
        }];
        let mut tui = types::LocalModelsTui::new(entries.clone(), 0);

        tui.confirm_delete();
        assert!(matches!(tui.mode, LocalModelsMode::Browse));

        entries[0].is_downloaded = true;
        let mut tui = types::LocalModelsTui::new(entries, 0);
        tui.confirm_delete();
        assert!(matches!(
            tui.mode,
            LocalModelsMode::ConfirmDelete { ref entry, .. } if entry.id == "missing"
        ));
    }

    #[test]
    fn selection_navigation_stays_in_bounds() {
        let entries = vec![
            LocalModelEntry {
                id: "a".to_string(),
                provider_id: "whisper".to_string(),
                name: "A".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_daemon_loaded: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/a.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
            LocalModelEntry {
                id: "b".to_string(),
                provider_id: "whisper".to_string(),
                name: "B".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_daemon_loaded: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/b.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
        ];
        let mut tui = types::LocalModelsTui::new(entries, 0);

        tui.move_selection_up();
        assert_eq!(tui.selected, 0);
        tui.move_selection_down();
        tui.move_selection_down();
        assert_eq!(tui.selected, 1);
    }

    #[test]
    fn grouped_display_index_finds_position_within_rendered_groups() {
        use super::local_model_list_view::grouped_display_index;

        let entries = vec![
            LocalModelEntry {
                id: "tiny".to_string(),
                provider_id: "whisper".to_string(),
                name: "Tiny".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_daemon_loaded: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/tiny.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: Some("group-a".to_string()),
            },
            LocalModelEntry {
                id: "large".to_string(),
                provider_id: "whisper".to_string(),
                name: "Large".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: true,
                is_active: false,
                is_daemon_loaded: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/large.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
        ];
        let idx = grouped_display_index(entries.iter().collect(), "whisper/tiny", 0);
        assert_eq!(idx, Some(4));
    }
}
