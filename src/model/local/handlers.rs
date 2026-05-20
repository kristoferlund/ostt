use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::sync::{Arc, Mutex};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::model::UserQuit;
use crate::config;
use crate::config::SelectedModel;
use crate::transcription::local_models::{
    delete_model, download_model_with_handle, is_safe_model_id, load_state,
    mark_downloaded_registry_model, model_destination, register_downloaded_custom_model,
    resolve_custom_model, validate_custom_model_registration, validate_downloaded_model,
    DownloadHandle, LocalModelState, RegistryEntry,
};
use crate::ui::{DialogAction, Toast};

use super::types::{CustomModelDetailsFocus, DownloadState, LocalModelEntry, LocalModelsMode, LocalModelsTui, RunningDownload};

pub(crate) fn build_local_model_entries(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<LocalModelEntry> {
    registry
        .iter()
        .chain(local_state.custom_models.iter())
        .map(|entry| {
            let is_downloaded = model_destination(entry).exists();
            let is_active = selected_model
                .map(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
                .unwrap_or(false);

            LocalModelEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                size_mb: entry.size_mb,
                is_downloaded,
                is_active,
                is_available_in_registry: registry
                    .iter()
                    .any(|registry_entry| registry_entry.id == entry.id),
                languages: entry.languages.clone(),
                url: entry.url.clone(),
                recommended_hardware: entry.recommended_hardware.clone(),
                category: entry.category.clone(),
                sha256: entry.sha256.clone(),
                group_id: entry.group_id.clone(),
            }
        })
        .collect()
}

pub(crate) fn downloaded_model_disk_usage_bytes(entries: &[LocalModelEntry]) -> u64 {
    entries
        .iter()
        .filter(|entry| entry.is_downloaded)
        .filter_map(|entry| {
            let registry_entry = RegistryEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                languages: entry.languages.clone(),
                size_mb: entry.size_mb,
                url: entry.url.clone(),
                recommended_hardware: entry.recommended_hardware.clone(),
                sha256: None,
                category: entry.category.clone(),
                group_id: entry.group_id.clone(),
            };
            model_destination(&registry_entry).metadata().ok()
        })
        .map(|metadata| metadata.len())
        .sum()
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn activate_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if !entry.is_downloaded {
        anyhow::bail!("Download first with [d]");
    }
    let path = model_destination(&registry_entry_from_model(entry));
    if !path.exists() {
        anyhow::bail!("Download first with [d]");
    }
    config::save_selected_model("local", &entry.id)
}

fn delete_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if delete_model(&entry.id).is_ok() {
        return Ok(());
    }

    let path = model_destination(&registry_entry_from_model(entry));
    std::fs::remove_file(&path)?;
    if config::get_selected_model_entry()?
        .is_some_and(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
    {
        config::clear_selected_model()?;
    }
    Ok(())
}

fn initial_download_state(entry: &RegistryEntry, is_custom: bool) -> DownloadState {
    DownloadState {
        model_id: entry.id.clone(),
        downloaded_bytes: 0,
        total_bytes: u64::from(entry.size_mb) * 1024 * 1024,
        progress: 0.0,
        speed_mbps: 0.0,
        status: "Starting download".to_string(),
        is_complete: false,
        is_custom,
    }
}

fn start_download(entry: RegistryEntry, is_custom: bool) -> RunningDownload {
    let state = Arc::new(Mutex::new(initial_download_state(&entry, is_custom)));
    let progress_state = state.clone();
    let handle = DownloadHandle::new();
    let task_handle = handle.clone();
    let task_entry = entry.clone();
    let task = tokio::spawn(async move {
        let destination = model_destination(&task_entry);
        download_model_with_handle(
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
        .await?;
        validate_downloaded_model(&task_entry)?;
        if is_custom {
            register_downloaded_custom_model(task_entry)?;
        } else {
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

fn sync_download_progress(tui: &mut LocalModelsTui, running: &RunningDownload) {
    if let Ok(state) = running.state.lock() {
        tui.mode = LocalModelsMode::Downloading(state.clone());
    }
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

pub(crate) async fn handle_key(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    running_download: &mut Option<RunningDownload>,
    key: KeyEvent,
) -> anyhow::Result<bool> {
    if is_ctrl_c(&key) {
        return Err(UserQuit.into());
    }

    let mode = tui.mode.clone();
    match (mode, key.code) {
        (LocalModelsMode::Browse, KeyCode::Char('q') | KeyCode::Esc) => return Ok(true),
        (LocalModelsMode::Browse, KeyCode::Down) => tui.move_selection_down(),
        (LocalModelsMode::Browse, KeyCode::Up) => tui.move_selection_up(),
        (LocalModelsMode::Browse, KeyCode::Enter) => handle_selected_entry(tui, registry)?,
        (LocalModelsMode::Browse, KeyCode::Char('i')) => tui.show_info(),
        (LocalModelsMode::Browse, KeyCode::Char('c')) => {
            tui.toast = None;
            tui.show_custom_input();
        }
        (LocalModelsMode::ErrorDialog { .. }, KeyCode::Enter | KeyCode::Esc) => {
            tui.close_error_dialog()
        }
        (LocalModelsMode::Browse, KeyCode::Char('x') | KeyCode::Delete) => tui.confirm_delete(),
        (LocalModelsMode::Info { .. }, KeyCode::Esc | KeyCode::Char('q')) => tui.back_to_browse(),
        (LocalModelsMode::Downloading(_), KeyCode::Enter | KeyCode::Tab | KeyCode::Esc) => {
            cancel_download(running_download)
        }
        (LocalModelsMode::CustomModelInput { .. }, KeyCode::Esc | KeyCode::Char('q')) => {
            tui.back_to_browse()
        }
        (
            LocalModelsMode::CustomModelInput {
                input,
                selected_action,
            },
            KeyCode::Enter,
        ) => {
            if selected_action == DialogAction::Ok {
                resolve_custom_input(tui, input.value()).await;
            } else {
                tui.back_to_browse();
            }
        }
        (
            LocalModelsMode::CustomModelInput {
                input,
                selected_action,
            },
            _,
        ) => {
            let mut input = input.clone();
            input.handle_event(&Event::Key(key));
            tui.mode = LocalModelsMode::CustomModelInput {
                input,
                selected_action,
            };
        }
        (LocalModelsMode::CustomModelDetails { .. }, KeyCode::Esc | KeyCode::Char('q')) => {
            tui.back_to_browse()
        }
        (LocalModelsMode::CustomModelDetails { .. }, KeyCode::Tab) => {
            toggle_custom_details_focus(tui)
        }
        (
            LocalModelsMode::CustomModelDetails {
                selected_action, ..
            },
            KeyCode::Enter,
        ) => {
            if selected_action == DialogAction::Ok {
                start_custom_details_download(tui, running_download)
            } else {
                tui.back_to_browse();
            }
        }
        (
            LocalModelsMode::CustomModelDetails {
                id_input,
                name_input,
                focus,
                resolved_entry,
                selected_action,
            },
            _,
        ) => {
            let mut id_input = id_input.clone();
            let mut name_input = name_input.clone();
            match focus {
                CustomModelDetailsFocus::Id => id_input.handle_event(&Event::Key(key)),
                CustomModelDetailsFocus::Name => name_input.handle_event(&Event::Key(key)),
            };
            tui.mode = LocalModelsMode::CustomModelDetails {
                resolved_entry,
                id_input,
                name_input,
                focus,
                selected_action,
            };
        }
        (
            LocalModelsMode::ConfirmDownload { .. },
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'),
        ) => tui.back_to_browse(),
        (LocalModelsMode::ConfirmDownload { entry, .. }, KeyCode::Enter) => {
            start_confirmed_download(tui, running_download, &entry);
        }
        (
            LocalModelsMode::ConfirmDownload { entry, .. },
            KeyCode::Char('y') | KeyCode::Char('Y'),
        ) => {
            start_confirmed_download(tui, running_download, &entry);
        }
        (
            LocalModelsMode::ConfirmAudioConfig { .. },
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'),
        ) => tui.back_to_browse(),
        (LocalModelsMode::ConfirmAudioConfig { entry, .. }, KeyCode::Enter) => {
            update_audio_config_and_activate(tui, registry, &entry)?;
        }
        (
            LocalModelsMode::ConfirmAudioConfig { entry, .. },
            KeyCode::Char('y') | KeyCode::Char('Y'),
        ) => update_audio_config_and_activate(tui, registry, &entry)?,
        (
            LocalModelsMode::ConfirmDelete { .. },
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'),
        ) => tui.back_to_browse(),
        (LocalModelsMode::ConfirmDelete { entry, .. }, KeyCode::Enter) => {
            delete_confirmed_entry(tui, registry, &entry)?;
        }
        (LocalModelsMode::ConfirmDelete { entry, .. }, KeyCode::Char('y') | KeyCode::Char('Y')) => {
            delete_confirmed_entry(tui, registry, &entry)?
        }
        _ => {}
    }

    Ok(false)
}

fn handle_selected_entry(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
) -> anyhow::Result<()> {
    let Some(entry) = tui.selected_entry().cloned() else {
        return Ok(());
    };

    if !entry.is_downloaded {
        if entry.is_available_in_registry {
            tui.mode = LocalModelsMode::ConfirmDownload {
                entry,
                selected_action: DialogAction::Ok,
            };
        } else {
            tui.show_error_dialog("Custom models must be added through [c]".to_string());
        }
        return Ok(());
    }

    let config = config::OsttConfig::load().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    if !config::is_local_transcription_audio_compatible(&config.audio) {
        tui.mode = LocalModelsMode::ConfirmAudioConfig {
            entry,
            selected_action: DialogAction::Ok,
        };
        return Ok(());
    }

    match activate_entry(&entry) {
        Ok(()) => {
            tui.toast = Some(Toast::success(format!("Activated {}", entry.name)));
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => tui.toast = Some(Toast::error(error.to_string())),
    }
    Ok(())
}

fn update_audio_config_and_activate(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    entry: &LocalModelEntry,
) -> anyhow::Result<()> {
    match config::ensure_local_transcription_audio_config().and_then(|()| activate_entry(entry)) {
        Ok(()) => {
            tui.back_to_browse();
            tui.toast = Some(Toast::success(format!("Activated {}", entry.name)));
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => tui.toast = Some(Toast::error(error.to_string())),
    }
    Ok(())
}

fn start_confirmed_download(
    tui: &mut LocalModelsTui,
    running_download: &mut Option<RunningDownload>,
    entry: &LocalModelEntry,
) {
    let running = start_download(registry_entry_from_model(entry), false);
    sync_download_progress(tui, &running);
    *running_download = Some(running);
}

fn cancel_download(running_download: &mut Option<RunningDownload>) {
    if let Some(running) = running_download.as_ref() {
        running.handle.cancel();
        if let Ok(mut state) = running.state.lock() {
            state.status = "Cancelling download".to_string();
        }
    }
}

async fn resolve_custom_input(tui: &mut LocalModelsTui, value: &str) {
    match resolve_custom_model(value).await {
        Ok(entry) => {
            tui.toast = None;
            tui.mode = LocalModelsMode::CustomModelDetails {
                id_input: Input::new(entry.id.clone()),
                name_input: Input::new(entry.name.clone()),
                resolved_entry: entry,
                focus: CustomModelDetailsFocus::Id,
                selected_action: DialogAction::Ok,
            };
        }
        Err(error) => tui.toast = Some(Toast::error(error.to_string())),
    }
}

fn toggle_custom_details_focus(tui: &mut LocalModelsTui) {
    if let LocalModelsMode::CustomModelDetails {
        resolved_entry,
        id_input,
        name_input,
        focus,
        selected_action,
    } = tui.mode.clone()
    {
        tui.mode = LocalModelsMode::CustomModelDetails {
            resolved_entry,
            id_input,
            name_input,
            focus: match focus {
                CustomModelDetailsFocus::Id => CustomModelDetailsFocus::Name,
                CustomModelDetailsFocus::Name => CustomModelDetailsFocus::Id,
            },
            selected_action,
        };
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
            tui.toast = Some(Toast::success(format!("Deleted {}", entry.name)));
            tui.back_to_browse();
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => {
            tui.back_to_browse();
            tui.show_error_dialog(error.to_string());
        }
    }
    Ok(())
}

pub(crate) async fn finish_completed_download(
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
            tui.back_to_browse();
            tui.refresh(&load_state(), registry)?;
            tui.toast = Some(Toast::success("Download complete"));
        }
        Err(error) => {
            if error.to_string() != "model download cancelled" {
                tui.back_to_browse();
                tui.show_error_dialog(error.to_string());
            } else {
                tui.back_to_browse();
            }
        }
    }
    Ok(())
}
