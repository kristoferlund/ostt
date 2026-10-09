use crate::transcription::local_models::{DownloadHandle, LocalModelState, RegistryEntry};
use crate::ui::session;
use ratcn::runtime::{CellOffset, FocusState, ModalState};
use ratcn::{InputState, Toast, ToasterState};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalModelEntry {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub description: String,
    pub size_mb: u32,
    pub is_downloaded: bool,
    pub is_active: bool,
    /// True when the local model daemon is running and has this model loaded.
    pub is_daemon_loaded: bool,
    pub is_available_in_registry: bool,
    pub languages: Vec<String>,
    pub url: String,
    pub recommended_hardware: Option<String>,
    pub category: Option<String>,
    pub sha256: Option<String>,
    pub group_id: Option<String>,
}

impl LocalModelEntry {
    pub(crate) fn registry_entry(&self) -> RegistryEntry {
        RegistryEntry {
            id: self.id.clone(),
            provider_id: self.provider_id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            languages: self.languages.clone(),
            size_mb: self.size_mb,
            url: self.url.clone(),
            recommended_hardware: self.recommended_hardware.clone(),
            sha256: self.sha256.clone(),
            category: self.category.clone(),
            group_id: self.group_id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DownloadState {
    pub model_id: String,
    pub provider_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f64,
    pub speed_mbps: f64,
    pub status: String,
    pub is_custom: bool,
}

/// What the screen shows. Every mode except `Browse` and `Info` is a dialog,
/// and [`State::set_mode`] keeps the runtime's modal stack in step with it.
#[derive(Clone, Debug)]
pub(crate) enum Mode {
    Browse,
    Info { entry: LocalModelEntry },
    CustomUrl,
    CustomDetails { resolved_entry: RegistryEntry },
    ConfirmDownload { entry: LocalModelEntry },
    Downloading(DownloadState),
    ConfirmDelete { entry: LocalModelEntry },
    Error { message: String },
}

impl Mode {
    pub(crate) fn modal_id(&self) -> Option<&'static str> {
        match self {
            Mode::Browse | Mode::Info { .. } => None,
            Mode::CustomUrl => Some("url"),
            Mode::CustomDetails { .. } => Some("details"),
            Mode::ConfirmDownload { .. } => Some("download"),
            Mode::Downloading(_) => Some("progress"),
            Mode::ConfirmDelete { .. } => Some("delete"),
            Mode::Error { .. } => Some("error"),
        }
    }
}

pub(crate) struct State {
    pub focus: FocusState,
    pub modals: ModalState,
    pub dialog_offset: CellOffset,
    pub mode: Mode,
    pub entries: Vec<LocalModelEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub url_input: InputState,
    pub id_input: InputState,
    pub name_input: InputState,
    pub toasts: ToasterState<'static>,
    /// Model ID currently loaded in the daemon, if any.
    pub daemon_model_id: Option<String>,
}

impl State {
    pub(crate) fn new(entries: Vec<LocalModelEntry>) -> Self {
        let selected = entries
            .iter()
            .position(|entry| entry.is_active)
            .unwrap_or(0);
        Self {
            focus: FocusState::default(),
            modals: ModalState::default(),
            dialog_offset: CellOffset::default(),
            mode: Mode::Browse,
            entries,
            selected,
            scroll_offset: 0,
            url_input: InputState::default(),
            id_input: InputState::default(),
            name_input: InputState::default(),
            toasts: ToasterState::default(),
            daemon_model_id: None,
        }
    }

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        let id = mode.modal_id();
        if self.modals.top().map(|top| top.as_str()) != id {
            self.modals.close(&mut self.focus);
            self.dialog_offset = CellOffset::default();
            if let Some(id) = id {
                self.modals
                    .open(id, &mut self.focus)
                    .expect("model dialogs never nest");
            }
        }
        self.mode = mode;
    }

    pub(crate) fn toast(&mut self, toast: Toast<'static>) {
        self.toasts
            .push(toast.duration(session::TOAST_DURATION), session::now());
    }

    /// Update cached daemon status and reflect it on each entry's `is_daemon_loaded`.
    pub(crate) fn update_daemon_status(&mut self, loaded_model_id: Option<&str>) {
        self.daemon_model_id = loaded_model_id.map(str::to_owned);
        for entry in &mut self.entries {
            entry.is_daemon_loaded = loaded_model_id == Some(entry.id.as_str());
        }
    }

    pub(crate) fn selected_entry(&self) -> Option<&LocalModelEntry> {
        self.entries.get(self.selected)
    }

    pub(crate) fn move_selection_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub(crate) fn move_selection_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub(crate) fn show_info(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            self.set_mode(Mode::Info { entry });
        }
    }

    pub(crate) fn confirm_delete(&mut self) {
        if let Some(entry) = self
            .selected_entry()
            .filter(|entry| entry.provider_id == "whisper" && entry.is_downloaded)
            .cloned()
        {
            self.set_mode(Mode::ConfirmDelete { entry });
        }
    }

    pub(crate) fn show_custom_url(&mut self) {
        self.url_input = InputState::default();
        self.set_mode(Mode::CustomUrl);
    }

    pub(crate) fn refresh(
        &mut self,
        local_state: &LocalModelState,
        registry: &[RegistryEntry],
    ) -> anyhow::Result<()> {
        let selected_model = crate::config::get_selected_model_entry()?;
        let config = crate::config::OsttConfig::load()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let authorized_provider_ids = crate::config::get_authorized_providers()?;
        self.entries = super::build_model_entries(
            &config,
            &authorized_provider_ids,
            local_state,
            registry,
            selected_model.as_ref(),
            self.daemon_model_id.as_deref(),
        );
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        Ok(())
    }
}

pub(crate) struct RunningDownload {
    pub state: Arc<Mutex<DownloadState>>,
    pub handle: DownloadHandle,
    pub task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
