mod dialogs;
mod info;
mod list;
mod types;

use ratcn::runtime::{CellOffset, DeclareCtx, Event, FocusState, KeyCode, Ratcn};
use ratcn::{terminal::Session, InputState, Toast};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    delete_model, download_model_with_handle, fetch_registry, is_safe_model_id, load_state,
    mark_downloaded_registry_model, model_destination, register_downloaded_custom_model,
    resolve_custom_model, validate_custom_model_registration, validate_downloaded_model,
    DownloadHandle, LocalModelState, RegistryEntry,
};
use crate::transcription::{self, TranscriptionProvider};
use crate::ui::session::{self, Chrome, Routed};

use types::{DownloadState, LocalModelEntry, Mode, RunningDownload, State};

pub(super) enum Msg {
    Focus(FocusState),
    DialogMoved(CellOffset),
    Url(InputState),
    Id(InputState),
    Name(InputState),
    Accept,
    Dismiss,
    Previous,
    Next,
    Info,
    Custom,
    Delete,
}

fn runtime() -> Ratcn<State, Msg> {
    Ratcn::new()
        .focus(|state: &State| &state.focus, Msg::Focus)
        .modals(|state| &state.modals)
}

/// Run the model screen until the user leaves it.
pub(crate) async fn run() -> anyhow::Result<()> {
    let local_state = load_state();
    let ostt_config =
        config::OsttConfig::load().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let authorized_provider_ids = config::get_authorized_providers()?;
    let registry = fetch_registry().await.unwrap_or_else(|error| {
        tracing::error!("Failed to fetch local model registry: {}", error);
        Vec::new()
    });
    let selected_model = config::get_selected_model_entry()?;
    let daemon_model_id = transcription::daemon_client::probe_daemon()
        .await
        .map(|daemon| daemon.model_id);

    let mut state = State::new(build_model_entries(
        &ostt_config,
        &authorized_provider_ids,
        &local_state,
        &registry,
        selected_model.as_ref(),
        daemon_model_id.as_deref(),
    ));
    state.daemon_model_id = daemon_model_id;
    if registry.is_empty() {
        state.set_mode(Mode::Error {
            message: "Could not load remote registry; custom URL entry is still available with [c]"
                .to_string(),
        });
    }

    ModelsView {
        session: session::open()?,
        ratcn: runtime(),
        state,
        registry,
        download: None,
    }
    .run()
    .await
}

struct ModelsView {
    session: Session,
    ratcn: Ratcn<State, Msg>,
    state: State,
    registry: Vec<RegistryEntry>,
    download: Option<RunningDownload>,
}

impl ModelsView {
    async fn run(mut self) -> anyhow::Result<()> {
        let mut last_daemon_probe = Instant::now();
        loop {
            self.finish_download().await?;
            // Re-probe to pick up a daemon that started or stopped in the background.
            if last_daemon_probe.elapsed() >= Duration::from_secs(2) {
                let daemon = transcription::daemon_client::probe_daemon().await;
                self.state
                    .update_daemon_status(daemon.as_ref().map(|d| d.model_id.as_str()));
                last_daemon_probe = Instant::now();
            }
            self.state.toasts.prune_expired(session::now());
            self.draw()?;

            let Some(event) =
                session::next_event(&mut self.session, Some(Duration::from_millis(100)))?
            else {
                continue;
            };
            let msg = match dialog_shortcut(&self.state.mode, &event) {
                Some(msg) => msg,
                None => {
                    match session::route(&mut self.session, &mut self.ratcn, &self.state, event)? {
                        Routed::Msg(msg) => msg,
                        Routed::Ignored(event)
                            if matches!(self.state.mode, Mode::Browse)
                                && session::is_cancel(&event) =>
                        {
                            return Ok(())
                        }
                        Routed::Ignored(event) => match shortcut(&self.state.mode, &event) {
                            Some(msg) => msg,
                            None => continue,
                        },
                        Routed::Quit => return Ok(()),
                        Routed::Redraw => continue,
                    }
                }
            };
            self.update(msg).await?;
        }
    }

    fn draw(&mut self) -> std::io::Result<()> {
        let title = match &self.state.mode {
            Mode::Info { entry } => Some(entry.name.clone()),
            _ => None,
        };
        let size = self.session.terminal_mut().size()?;
        let body = session::body_area(size.into(), title.is_some());
        list::scroll_to_selection(&mut self.state, body.height);
        let state = &self.state;
        let chrome = Chrome {
            title: title.as_deref(),
            footer: if title.is_some() {
                "esc/q back"
            } else {
                "↑↓ nav, ↵ activate/download, x/del delete, i info, c custom, esc/q back"
            },
            toasts: Some(&state.toasts),
        };
        session::draw(
            &mut self.session,
            &mut self.ratcn,
            state,
            chrome,
            |ctx, body| declare(ctx, body, state),
        )
    }

    async fn update(&mut self, msg: Msg) -> anyhow::Result<()> {
        let state = &mut self.state;
        match msg {
            Msg::Focus(focus) => state.focus = focus,
            Msg::DialogMoved(offset) => state.dialog_offset = offset,
            Msg::Url(input) => state.url_input = input,
            Msg::Id(input) => state.id_input = input,
            Msg::Name(input) => state.name_input = input,
            Msg::Previous => state.move_selection_up(),
            Msg::Next => state.move_selection_down(),
            Msg::Info => state.show_info(),
            Msg::Custom => state.show_custom_url(),
            Msg::Delete => state.confirm_delete(),
            Msg::Accept => match state.mode.clone() {
                Mode::Browse => self.activate_selected().await?,
                Mode::CustomUrl => self.resolve_custom_url().await,
                Mode::CustomDetails { resolved_entry } => {
                    self.start_custom_download(resolved_entry)
                }
                Mode::ConfirmDownload { entry } => {
                    tracing::info!("Starting download for local model '{}'", entry.id);
                    self.start_download(entry.registry_entry(), false);
                }
                Mode::ConfirmDelete { entry } => self.delete(&entry)?,
                Mode::Downloading(_) => self.cancel_download(),
                Mode::Error { .. } | Mode::Info { .. } => state.set_mode(Mode::Browse),
            },
            Msg::Dismiss => match state.mode {
                Mode::Downloading(_) => self.cancel_download(),
                _ => state.set_mode(Mode::Browse),
            },
        }
        Ok(())
    }

    async fn activate_selected(&mut self) -> anyhow::Result<()> {
        let Some(entry) = self.state.selected_entry().cloned() else {
            return Ok(());
        };
        tracing::debug!("Selected local model '{}'", entry.id);

        if entry.provider_id == "whisper" && !entry.is_downloaded {
            if entry.is_available_in_registry {
                self.state.set_mode(Mode::ConfirmDownload { entry });
            } else {
                self.state.set_mode(Mode::Error {
                    message: "Custom models must be added through [c]".to_string(),
                });
            }
            return Ok(());
        }

        match activate_entry(&entry).await {
            Ok(()) => {
                tracing::info!("Activated local model '{}'", entry.id);
                self.state
                    .toast(Toast::success(format!("Activated {}", entry.name)));
                self.state.refresh(&load_state(), &self.registry)?;
            }
            Err(error) => {
                tracing::error!("Failed to activate local model '{}': {}", entry.id, error);
                self.state.toast(Toast::error(error.to_string()));
            }
        }
        Ok(())
    }

    async fn resolve_custom_url(&mut self) {
        match resolve_custom_model(self.state.url_input.value()).await {
            Ok(entry) => {
                tracing::debug!("Resolved custom local model '{}'", entry.id);
                self.state.id_input = InputState::new(entry.id.clone());
                self.state.name_input = InputState::new(entry.name.clone());
                self.state.set_mode(Mode::CustomDetails {
                    resolved_entry: entry,
                });
            }
            Err(error) => {
                tracing::error!("Failed to resolve custom local model input: {}", error);
                self.state.toast(Toast::error(error.to_string()));
            }
        }
    }

    fn start_custom_download(&mut self, mut entry: RegistryEntry) {
        let id = self.state.id_input.value().trim();
        let name = self.state.name_input.value().trim();
        let error = if !is_safe_model_id(id) {
            Some("Model ID must use lowercase letters, numbers, '.', '_' or '-'".to_string())
        } else if self.state.entries.iter().any(|entry| entry.id == id) {
            Some(format!("Model ID '{id}' already exists"))
        } else if name.is_empty() {
            Some("Model name is required".to_string())
        } else {
            entry.id = id.to_string();
            entry.name = name.to_string();
            validate_custom_model_registration(&entry)
                .err()
                .map(|error| error.to_string())
        };
        if let Some(error) = error {
            self.state.toast(Toast::error(error));
            return;
        }
        tracing::info!("Starting download for custom local model '{}'", entry.id);
        self.start_download(entry, true);
    }

    fn start_download(&mut self, entry: RegistryEntry, is_custom: bool) {
        let running = spawn_download(entry, is_custom);
        sync_download_progress(&mut self.state, &running);
        self.download = Some(running);
    }

    fn cancel_download(&mut self) {
        if let Some(running) = &self.download {
            tracing::info!("Cancelling local model download");
            running.handle.cancel();
            if let Ok(mut state) = running.state.lock() {
                state.status = "Cancelling download".to_string();
            }
        }
    }

    async fn finish_download(&mut self) -> anyhow::Result<()> {
        let Some(running) = &self.download else {
            return Ok(());
        };
        sync_download_progress(&mut self.state, running);
        if !running.task.is_finished() {
            return Ok(());
        }

        let running = self.download.take().expect("running download");
        self.state.set_mode(Mode::Browse);
        match running.task.await? {
            Ok(()) => {
                tracing::info!("Local model download completed");
                self.state.refresh(&load_state(), &self.registry)?;
                self.state.toast(Toast::success("Download complete"));
            }
            Err(error) if error.to_string() == "model download cancelled" => {
                tracing::debug!("Local model download cancelled");
            }
            Err(error) => {
                tracing::error!("Local model download failed: {}", error);
                self.state.set_mode(Mode::Error {
                    message: error.to_string(),
                });
            }
        }
        Ok(())
    }

    fn delete(&mut self, entry: &LocalModelEntry) -> anyhow::Result<()> {
        self.state.set_mode(Mode::Browse);
        match delete_entry(entry) {
            Ok(()) => {
                tracing::info!("Deleted local model '{}'", entry.id);
                if entry.is_daemon_loaded {
                    stop_daemon_for_deleted_model(&entry.id);
                }
                self.state
                    .toast(Toast::success(format!("Deleted {}", entry.name)));
                self.state.refresh(&load_state(), &self.registry)?;
            }
            Err(error) => {
                tracing::error!("Failed to delete local model '{}': {}", entry.id, error);
                self.state.set_mode(Mode::Error {
                    message: error.to_string(),
                });
            }
        }
        Ok(())
    }
}

fn declare(ctx: &mut DeclareCtx<'_, State, Msg>, body: ratatui::layout::Rect, state: &State) {
    match &state.mode {
        Mode::Info { entry } => ctx.paint_widget(info::paragraph(entry), body),
        _ => list::declare(ctx, body),
    }
    if let (Some(id), Some(dialog)) = (state.mode.modal_id(), dialogs::dialog(state)) {
        ctx.modal(id, dialog, ctx.area());
    }
}

/// Confirmation keys answer the open dialog before its buttons see them.
fn dialog_shortcut(mode: &Mode, event: &Event) -> Option<Msg> {
    let Event::Key(key) = event else {
        return None;
    };
    match (mode, key.code) {
        (Mode::ConfirmDownload { .. } | Mode::ConfirmDelete { .. }, KeyCode::Char('y' | 'Y')) => {
            Some(Msg::Accept)
        }
        (Mode::ConfirmDownload { .. } | Mode::ConfirmDelete { .. }, KeyCode::Char('n' | 'N')) => {
            Some(Msg::Dismiss)
        }
        (Mode::Downloading(_), KeyCode::Tab) => Some(Msg::Dismiss),
        _ => None,
    }
}

/// Keys no component wanted.
fn shortcut(mode: &Mode, event: &Event) -> Option<Msg> {
    let Event::Key(key) = event else {
        return None;
    };
    match (mode, key.code) {
        (Mode::Browse, KeyCode::Down) => Some(Msg::Next),
        (Mode::Browse, KeyCode::Up) => Some(Msg::Previous),
        (Mode::Browse, KeyCode::Enter) => Some(Msg::Accept),
        (Mode::Browse, KeyCode::Char('i')) => Some(Msg::Info),
        (Mode::Browse, KeyCode::Char('c')) => Some(Msg::Custom),
        (Mode::Browse, KeyCode::Char('x' | 'd') | KeyCode::Delete) => Some(Msg::Delete),
        (Mode::Info { .. }, _) if session::is_cancel(event) => Some(Msg::Dismiss),
        _ => None,
    }
}

async fn activate_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if entry.provider_id != "whisper" {
        config::save_selected_model(&entry.provider_id, &entry.id)?;
        return Ok(());
    }
    if !model_destination(&entry.registry_entry()).exists() {
        anyhow::bail!("Download first with [d]");
    }
    config::save_selected_model(&entry.provider_id, &entry.id)?;
    reload_daemon_if_running(&entry.id).await
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
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
    daemon_model_id: Option<&str>,
) -> Vec<LocalModelEntry> {
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

fn spawn_download(entry: RegistryEntry, is_custom: bool) -> RunningDownload {
    let state = Arc::new(Mutex::new(DownloadState {
        model_id: entry.id.clone(),
        provider_id: entry.provider_id.clone(),
        downloaded_bytes: 0,
        total_bytes: u64::from(entry.size_mb) * 1024 * 1024,
        progress: 0.0,
        speed_mbps: 0.0,
        status: "Starting download".to_string(),
        is_custom,
    }));
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

fn sync_download_progress(state: &mut State, running: &RunningDownload) {
    if let Ok(download) = running.state.lock() {
        state.set_mode(Mode::Downloading(download.clone()));
    }
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

    let path = model_destination(&entry.registry_entry());
    std::fs::remove_file(&path)?;
    if config::get_selected_model_entry()?.is_some_and(|selected| {
        selected.provider_id == entry.provider_id && selected.model_id == entry.id
    }) {
        config::clear_selected_model()?;
    }
    Ok(())
}

pub(super) fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let tui = State::new(entries);

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

            let entries = build_local_model_entries(&registry, Some(&selected), None);

            assert_eq!(entries.len(), 1);
            assert!(entries[0].is_downloaded);
            assert!(entries[0].is_active);
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
        let mut tui = State::new(entries.clone());

        tui.confirm_delete();
        assert!(matches!(tui.mode, Mode::Browse));

        entries[0].is_downloaded = true;
        let mut tui = State::new(entries);
        tui.confirm_delete();
        assert!(matches!(
            tui.mode,
            Mode::ConfirmDelete { ref entry, .. } if entry.id == "missing"
        ));
    }
}
