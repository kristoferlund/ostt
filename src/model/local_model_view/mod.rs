pub(crate) mod custom_model_details_dialog;
pub(crate) mod custom_model_url_input_dialog;
pub(crate) mod local_model_audio_config_confirmation_dialog;
pub(crate) mod local_model_delete_confirmation_dialog;
pub(crate) mod local_model_download_confirmation_dialog;
pub(crate) mod local_model_download_progress_dialog;
pub(crate) mod local_model_info_view;
pub(crate) mod local_model_list_view;
pub(crate) mod local_model_view_helpers;
pub(crate) mod types;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Frame;
use ratatui::Terminal;
use std::io::Stdout;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::config::{self, SelectedModel};
use crate::model::UserQuit;
use crate::transcription::local_models::{
    delete_model, download_model_with_handle, fetch_registry, is_safe_model_id, load_state,
    mark_downloaded_registry_model, model_destination, register_downloaded_custom_model,
    resolve_custom_model, validate_custom_model_registration, validate_downloaded_model,
    DownloadHandle, LocalModelState, RegistryEntry,
};
use crate::ui::{render_error_dialog, render_toast, DialogAction, Toast};

use custom_model_details_dialog::CustomModelDetailsDialog;
use custom_model_url_input_dialog::CustomModelUrlInputDialog;
use local_model_audio_config_confirmation_dialog::LocalModelAudioConfigConfirmationDialog;
use local_model_delete_confirmation_dialog::LocalModelDeleteConfirmationDialog;
use local_model_download_confirmation_dialog::LocalModelDownloadConfirmationDialog;
use local_model_download_progress_dialog::LocalModelDownloadProgressDialog;
use local_model_info_view::LocalModelInfoView;
use local_model_list_view::LocalModelListView;
use types::{
    CustomModelDetailsFocus, DownloadState, LocalModelEntry, LocalModelsMode, LocalModelsTui,
    RunningDownload,
};

pub(crate) async fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let local_state = load_state();
    let registry = match fetch_registry().await {
        Ok(registry) => registry,
        Err(error) => {
            tracing::error!("Failed to fetch local model registry: {}", error);
            Vec::new()
        }
    };
    let selected_model = crate::config::get_selected_model_entry()?;
    let entries = build_local_model_entries(&local_state, &registry, selected_model.as_ref());
    let mut tui =
        types::LocalModelsTui::new(entries.clone(), downloaded_model_disk_usage_bytes(&entries));
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

    loop {
        if tui.toast.as_ref().is_some_and(Toast::is_expired) {
            tui.toast = None;
        }
        finish_completed_download(&mut tui, &registry, &mut running_download).await?;
        terminal.draw(|frame| render_local_models(frame, &tui))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if handle_key(&mut tui, &registry, &mut running_download, key).await? {
                break;
            }
        }
    }
    Ok(())
}

fn render_local_models(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
    // Dialog modes render over the browse list so users keep their place.
    match &tui.mode {
        LocalModelsMode::Browse => LocalModelListView::render(frame, tui),
        LocalModelsMode::Info { entry } => {
            LocalModelInfoView::render(frame, entry);
        }
        LocalModelsMode::ConfirmDelete {
            entry,
            selected_action,
        } => {
            LocalModelListView::render(frame, tui);
            LocalModelDeleteConfirmationDialog::render(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmDownload {
            entry,
            selected_action,
        } => {
            LocalModelListView::render(frame, tui);
            LocalModelDownloadConfirmationDialog::render(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmAudioConfig {
            entry,
            selected_action,
        } => {
            LocalModelListView::render(frame, tui);
            LocalModelAudioConfigConfirmationDialog::render(frame, entry, *selected_action);
        }
        LocalModelsMode::CustomModelInput {
            input,
            selected_action,
        } => {
            LocalModelListView::render(frame, tui);
            CustomModelUrlInputDialog::render(frame, input, *selected_action);
        }
        LocalModelsMode::CustomModelDetails {
            id_input,
            name_input,
            focus,
            selected_action,
            ..
        } => {
            LocalModelListView::render(frame, tui);
            CustomModelDetailsDialog::render(frame, id_input, name_input, *focus, *selected_action);
        }
        LocalModelsMode::Downloading(state) => {
            LocalModelListView::render(frame, tui);
            LocalModelDownloadProgressDialog::render(frame, state);
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
                    LocalModelListView::render(frame, tui);
                    CustomModelUrlInputDialog::render(frame, input, *selected_action);
                }
                _ => LocalModelListView::render(frame, tui),
            }
            render_error_dialog(frame, "Error", message.clone());
        }
    }

    if let Some(toast) = &tui.toast {
        render_toast(frame, toast);
    }
}

fn build_local_model_entries(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<LocalModelEntry> {
    // Custom entries are appended to the remote registry and marked as non-registry models.
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

fn downloaded_model_disk_usage_bytes(entries: &[LocalModelEntry]) -> u64 {
    entries
        .iter()
        .filter(|entry| entry.is_downloaded)
        .filter_map(|entry| {
            model_destination(&registry_entry_from_model(entry))
                .metadata()
                .ok()
        })
        .map(|metadata| metadata.len())
        .sum()
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

async fn handle_key(
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
        (LocalModelsMode::Browse, KeyCode::Char('i')) => {
            tracing::debug!("Opening local model info view");
            tui.show_info();
        }
        (LocalModelsMode::Browse, KeyCode::Char('c')) => {
            tracing::debug!("Opening custom local model input dialog");
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
    tracing::debug!("Selected local model '{}'", entry.id);

    if !entry.is_downloaded {
        if entry.is_available_in_registry {
            tracing::debug!("Confirming download for local model '{}'", entry.id);
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
        tracing::debug!(
            "Confirming audio config update before activating local model '{}'",
            entry.id
        );
        tui.mode = LocalModelsMode::ConfirmAudioConfig {
            entry,
            selected_action: DialogAction::Ok,
        };
        return Ok(());
    }

    match activate_entry(&entry) {
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

fn update_audio_config_and_activate(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    entry: &LocalModelEntry,
) -> anyhow::Result<()> {
    match config::ensure_local_transcription_audio_config().and_then(|()| activate_entry(entry)) {
        Ok(()) => {
            tracing::info!(
                "Updated audio config and activated local model '{}'",
                entry.id
            );
            tui.back_to_browse();
            tui.toast = Some(Toast::success(format!("Activated {}", entry.name)));
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => {
            tracing::error!(
                "Failed to update audio config for local model '{}': {}",
                entry.id,
                error
            );
            tui.toast = Some(Toast::error(error.to_string()));
        }
    }
    Ok(())
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
                focus: CustomModelDetailsFocus::Id,
                selected_action: DialogAction::Ok,
            };
        }
        Err(error) => {
            tracing::error!("Failed to resolve custom local model input: {}", error);
            tui.toast = Some(Toast::error(error.to_string()));
        }
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
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-models-tui-test-{unique}"));
        let models_dir = dir.join("models");
        set_test_models_dir(Some(models_dir.clone()));
        std::env::set_var("HOME", &dir);

        test(models_dir);

        set_test_models_dir(None);
        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(dir);
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
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

    #[test]
    fn build_local_model_entries_merges_registry_and_custom_entries() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![RegistryEntry {
                    category: Some("custom".to_string()),
                    group_id: Some("Custom".to_string()),
                    ..registry_entry("custom")
                }],
            };

            let entries = build_local_model_entries(&state, &registry, None);

            assert_eq!(entries.len(), 2);
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
    fn build_local_model_entries_marks_downloaded_and_active_from_filesystem_and_selection() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let selected = SelectedModel {
                provider_id: "local".to_string(),
                model_id: "turbo".to_string(),
            };

            let entries =
                build_local_model_entries(&LocalModelState::default(), &registry, Some(&selected));

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
            let entries = build_local_model_entries(&LocalModelState::default(), &registry, None);

            assert_eq!(downloaded_model_disk_usage_bytes(&entries), 3);
        });
    }

    #[test]
    fn selection_navigation_stays_in_bounds() {
        let entries = vec![
            LocalModelEntry {
                id: "a".to_string(),
                name: "A".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
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
                name: "B".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
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
    fn selection_navigation_uses_rendered_group_order() {
        let entries = vec![
            LocalModelEntry {
                id: "tiny".to_string(),
                name: "Tiny".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/tiny.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
            LocalModelEntry {
                id: "turbo".to_string(),
                name: "Turbo".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: true,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/turbo.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
            LocalModelEntry {
                id: "large-v3".to_string(),
                name: "Large".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/large-v3.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
        ];
        let tui = types::LocalModelsTui::new(entries, 0);

        assert_eq!(tui.selected_entry().expect("selected").id, "tiny");
    }

    #[test]
    fn grouped_display_index_finds_position_within_rendered_groups() {
        use super::local_model_list_view::grouped_display_index;

        let entries = vec![
            LocalModelEntry {
                id: "tiny".to_string(),
                name: "Tiny".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
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
                name: "Large".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: true,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/large.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
        ];
        let idx = grouped_display_index(entries.iter().collect(), "tiny", 0);
        assert_eq!(idx, Some(1));
    }
}
