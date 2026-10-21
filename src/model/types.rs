use crate::transcription::local_models::{DownloadHandle, LocalModelState, RegistryEntry};
use crate::ui::components::list::clamp_selection;
use crate::ui::session;
use ratcn::runtime::{CellOffset, FocusState, ModalState};
use ratcn::{InputState, Toast, ToasterState};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelEntry {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub description: String,
    pub size_mb: u32,
    pub is_downloaded: bool,
    pub is_active: bool,
    pub is_available_in_registry: bool,
    pub languages: Vec<String>,
    pub url: String,
    pub recommended_hardware: Option<String>,
    pub category: Option<String>,
    pub sha256: Option<String>,
    pub group_id: Option<String>,
}

impl ModelEntry {
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
    Info { entry: ModelEntry },
    CustomUrl,
    CustomDetails { resolved_entry: RegistryEntry },
    ConfirmDownload { entry: ModelEntry },
    Downloading(DownloadState),
    ConfirmDelete { entry: ModelEntry },
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
    pub entries: Vec<ModelEntry>,
    pub selected: Option<usize>,
    pub url_input: InputState,
    pub id_input: InputState,
    pub name_input: InputState,
    pub toasts: ToasterState<'static>,
    pub daemon: DaemonStatus,
}

/// The local model daemon as the footer shows it. A running daemon follows the
/// selected local model, so its state belongs to the screen, not to a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DaemonStatus {
    Off,
    Starting,
    Stopping,
    On {
        model_id: String,
        backend: Option<String>,
    },
}

/// What the daemon key does in the current state.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DaemonToggle {
    Start(String),
    Stop,
    /// A start or stop is already in progress.
    Busy,
    /// The daemon hosts local models only.
    NoLocalModel,
}

impl DaemonStatus {
    pub(crate) fn from_probe(
        info: Option<&crate::transcription::daemon_client::DaemonInfo>,
    ) -> Self {
        match info {
            Some(info) => Self::On {
                model_id: info.model_id.clone(),
                backend: info.backend.clone(),
            },
            None => Self::Off,
        }
    }

    pub(crate) fn loaded_model_id(&self) -> Option<&str> {
        match self {
            Self::On { model_id, .. } => Some(model_id),
            _ => None,
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            Self::Off => "daemon off".to_string(),
            Self::Starting => "daemon starting…".to_string(),
            Self::Stopping => "daemon stopping…".to_string(),
            Self::On {
                model_id,
                backend: Some(backend),
            } => format!("daemon on · {model_id} · {backend}"),
            Self::On {
                model_id,
                backend: None,
            } => format!("daemon on · {model_id}"),
        }
    }
}

impl State {
    pub(crate) fn new(entries: Vec<ModelEntry>) -> Self {
        let mut selected = entries.iter().position(|entry| entry.is_active);
        clamp_selection(&mut selected, entries.len());
        Self {
            focus: FocusState::default(),
            modals: ModalState::default(),
            dialog_offset: CellOffset::default(),
            mode: Mode::Browse,
            entries,
            selected,
            url_input: InputState::default(),
            id_input: InputState::default(),
            name_input: InputState::default(),
            toasts: ToasterState::default(),
            daemon: DaemonStatus::Off,
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
        session::toast(&mut self.toasts, toast);
    }

    pub(crate) fn daemon_toggle(&self) -> DaemonToggle {
        match &self.daemon {
            DaemonStatus::Starting | DaemonStatus::Stopping => DaemonToggle::Busy,
            DaemonStatus::On { .. } => DaemonToggle::Stop,
            DaemonStatus::Off => match self.active_local_model_id() {
                Some(model_id) => DaemonToggle::Start(model_id.to_string()),
                None => DaemonToggle::NoLocalModel,
            },
        }
    }

    /// The model a running daemon should reload to follow the selection.
    /// Selecting a cloud model leaves the daemon loaded, so switching back to
    /// local is instant.
    pub(crate) fn daemon_follow_target(&self) -> Option<String> {
        let loaded = self.daemon.loaded_model_id()?;
        self.active_local_model_id()
            .filter(|active| *active != loaded)
            .map(str::to_string)
    }

    fn active_local_model_id(&self) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.is_active)
            .filter(|entry| {
                crate::transcription::TranscriptionProvider::is_local_id(&entry.provider_id)
            })
            .map(|entry| entry.id.as_str())
    }

    pub(crate) fn selected_entry(&self) -> Option<&ModelEntry> {
        self.entries.get(self.selected?)
    }

    pub(crate) fn show_info(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            self.set_mode(Mode::Info { entry });
        }
    }

    pub(crate) fn confirm_delete(&mut self) {
        if let Some(entry) = self
            .selected_entry()
            .filter(|entry| {
                crate::transcription::TranscriptionProvider::is_local_id(&entry.provider_id)
                    && entry.is_downloaded
            })
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
        );
        clamp_selection(&mut self.selected, self.entries.len());
        Ok(())
    }
}

pub(crate) struct RunningDownload {
    pub state: Arc<Mutex<DownloadState>>,
    pub handle: DownloadHandle,
    pub task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
