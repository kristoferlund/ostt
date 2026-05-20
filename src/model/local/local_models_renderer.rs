use ratatui::Frame;

use crate::ui::{render_error_dialog, render_toast};

use super::custom_model_details_dialog::CustomModelDetailsDialog;
use super::custom_model_url_input_dialog::CustomModelUrlInputDialog;
use super::local_model_audio_config_confirmation_dialog::LocalModelAudioConfigConfirmationDialog;
use super::local_model_delete_confirmation_dialog::LocalModelDeleteConfirmationDialog;
use super::local_model_download_confirmation_dialog::LocalModelDownloadConfirmationDialog;
use super::local_model_download_progress_dialog::LocalModelDownloadProgressDialog;
use super::local_model_info_view::LocalModelInfoView;
use super::local_model_view::LocalModelView;
use super::types::{LocalModelsMode, LocalModelsTui};

pub(crate) fn render_local_models(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
    match &tui.mode {
        LocalModelsMode::Browse => LocalModelView::render(frame, tui),
        LocalModelsMode::Info { entry } => {
            LocalModelInfoView::render(frame, entry);
        }
        LocalModelsMode::ConfirmDelete {
            entry,
            selected_action,
        } => {
            LocalModelView::render(frame, tui);
            LocalModelDeleteConfirmationDialog::render(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmDownload {
            entry,
            selected_action,
        } => {
            LocalModelView::render(frame, tui);
            LocalModelDownloadConfirmationDialog::render(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmAudioConfig {
            entry,
            selected_action,
        } => {
            LocalModelView::render(frame, tui);
            LocalModelAudioConfigConfirmationDialog::render(frame, entry, *selected_action);
        }
        LocalModelsMode::CustomModelInput {
            input,
            selected_action,
        } => {
            LocalModelView::render(frame, tui);
            CustomModelUrlInputDialog::render(frame, input, *selected_action);
        }
        LocalModelsMode::CustomModelDetails {
            id_input,
            name_input,
            focus,
            selected_action,
            ..
        } => {
            LocalModelView::render(frame, tui);
            CustomModelDetailsDialog::render(frame, id_input, name_input, *focus, *selected_action);
        }
        LocalModelsMode::Downloading(state) => {
            LocalModelView::render(frame, tui);
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
                    LocalModelView::render(frame, tui);
                    CustomModelUrlInputDialog::render(frame, input, *selected_action);
                }
                _ => LocalModelView::render(frame, tui),
            }
            render_error_dialog(frame, "Error", message.clone());
        }
    }

    if let Some(toast) = &tui.toast {
        render_toast(frame, toast);
    }
}
