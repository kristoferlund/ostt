use crate::transcription::local_models::{DownloadHandle, LocalModelState, RegistryEntry};
use crate::ui::DialogAction;
use std::sync::{Arc, Mutex};
use tui_input::Input;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalModelEntry {
    pub id: String,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DownloadState {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f64,
    pub speed_mbps: f64,
    pub status: String,
    pub is_complete: bool,
    pub is_custom: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum LocalModelsMode {
    Browse,
    CustomModelInput {
        input: Input,
        selected_action: DialogAction,
    },
    CustomModelDetails {
        resolved_entry: RegistryEntry,
        id_input: Input,
        name_input: Input,
        focus: CustomModelDetailsFocus,
        selected_action: DialogAction,
    },
    ConfirmDownload {
        entry: LocalModelEntry,
        selected_action: DialogAction,
    },
    Downloading(DownloadState),
    Info {
        entry: LocalModelEntry,
    },
    ConfirmDelete {
        entry: LocalModelEntry,
        selected_action: DialogAction,
    },
    ConfirmAudioConfig {
        entry: LocalModelEntry,
        selected_action: DialogAction,
    },
    ErrorDialog {
        message: String,
        return_mode: Box<LocalModelsMode>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CustomModelDetailsFocus {
    Id,
    Name,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalModelsTui {
    pub entries: Vec<LocalModelEntry>,
    pub selected: usize,
    pub mode: LocalModelsMode,
    pub downloaded_model_disk_usage_bytes: u64,
    pub toast: Option<crate::ui::Toast>,
}

impl LocalModelsTui {
    pub(crate) fn new(
        entries: Vec<LocalModelEntry>,
        downloaded_model_disk_usage_bytes: u64,
    ) -> Self {
        Self {
            entries,
            selected: 0,
            mode: LocalModelsMode::Browse,
            downloaded_model_disk_usage_bytes,
            toast: None,
        }
    }

    pub(crate) fn selected_entry(&self) -> Option<&LocalModelEntry> {
        self.display_entries().get(self.selected).copied()
    }

    pub(crate) fn display_entries(&self) -> Vec<&LocalModelEntry> {
        self.entries.iter().collect()
    }

    pub(crate) fn move_selection_down(&mut self) {
        if self.selected + 1 < self.display_entries().len() {
            self.selected += 1;
        }
    }

    pub(crate) fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub(crate) fn show_info(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            self.mode = LocalModelsMode::Info { entry };
        }
    }

    pub(crate) fn confirm_delete(&mut self) {
        if let Some(entry) = self
            .selected_entry()
            .filter(|entry| entry.is_downloaded)
            .cloned()
        {
            self.mode = LocalModelsMode::ConfirmDelete {
                entry,
                selected_action: DialogAction::Cancel,
            };
        }
    }

    pub(crate) fn show_error_dialog(&mut self, message: String) {
        self.mode = LocalModelsMode::ErrorDialog {
            message,
            return_mode: Box::new(self.mode.clone()),
        };
    }

    pub(crate) fn close_error_dialog(&mut self) {
        if let LocalModelsMode::ErrorDialog { return_mode, .. } = self.mode.clone() {
            self.mode = *return_mode;
        }
    }

    pub(crate) fn back_to_browse(&mut self) {
        self.mode = LocalModelsMode::Browse;
    }

    pub(crate) fn show_custom_input(&mut self) {
        self.mode = LocalModelsMode::CustomModelInput {
            input: Input::default(),
            selected_action: DialogAction::Ok,
        };
    }

    pub(crate) fn refresh(
        &mut self,
        local_state: &LocalModelState,
        registry: &[RegistryEntry],
    ) -> anyhow::Result<()> {
        let selected_model = crate::config::get_selected_model_entry()?;
        self.entries =
            super::build_local_model_entries(local_state, registry, selected_model.as_ref());
        self.downloaded_model_disk_usage_bytes =
            super::downloaded_model_disk_usage_bytes(&self.entries);
        let display_len = self.display_entries().len();
        if self.selected >= display_len {
            self.selected = display_len.saturating_sub(1);
        }
        Ok(())
    }
}

pub(crate) struct RunningDownload {
    pub state: Arc<Mutex<DownloadState>>,
    pub handle: DownloadHandle,
    pub task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
