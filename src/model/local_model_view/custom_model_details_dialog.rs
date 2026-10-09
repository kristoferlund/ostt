use super::types::{LocalModelsMode, LocalModelsTui};
use super::{model_dialog, Msg};
use crate::ui::components::modal::form_dialog;
use ratcn::{runtime::CellOffset, Dialog, Input};

pub(super) struct CustomModelDetailsDialog;

impl CustomModelDetailsDialog {
    pub(super) fn dialog(offset: CellOffset) -> Dialog<LocalModelsTui, Msg> {
        form_dialog(
            model_dialog("Download Custom Whisper Model 2/3", "", "Download", offset),
            "Choose how this custom model should appear in OSTT.",
            vec![
                Input::new()
                    .title("ID")
                    .value(
                        |state: &LocalModelsTui| match &state.mode {
                            LocalModelsMode::CustomModelDetails { id_input, .. } => id_input,
                            _ => unreachable!("ID field is only declared in details mode"),
                        },
                        Msg::Id,
                    )
                    .on_submit(|| Msg::Accept),
                Input::new()
                    .title("Name")
                    .value(
                        |state: &LocalModelsTui| match &state.mode {
                            LocalModelsMode::CustomModelDetails { name_input, .. } => name_input,
                            _ => unreachable!("name field is only declared in details mode"),
                        },
                        Msg::Name,
                    )
                    .on_submit(|| Msg::Accept),
            ],
        )
    }
}
