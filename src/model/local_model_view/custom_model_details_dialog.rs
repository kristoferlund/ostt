use crate::ui::components::modal::{form_dialog, ModalMessage, ModalViewState};
use ratcn::{runtime::CellOffset, Dialog};

pub(super) struct CustomModelDetailsDialog;

impl CustomModelDetailsDialog {
    pub(super) fn dialog(offset: CellOffset) -> Dialog<ModalViewState, ModalMessage> {
        form_dialog(
            "Download Custom Whisper Model 2/3",
            "Choose how this custom model should appear in OSTT.",
            &["ID", "Name"],
            "Download",
            offset,
        )
    }
}
