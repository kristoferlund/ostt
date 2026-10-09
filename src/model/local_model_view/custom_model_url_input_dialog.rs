use crate::ui::components::modal::{form_dialog, ModalMessage, ModalViewState};
use ratcn::{runtime::CellOffset, Dialog};

pub(super) struct CustomModelUrlInputDialog;

impl CustomModelUrlInputDialog {
    pub(super) fn dialog(offset: CellOffset) -> Dialog<ModalViewState, ModalMessage> {
        form_dialog(
            "Download Custom Whisper Model 1/3",
            "Paste a Hugging Face model page or a direct model file URL.\nSupported files: .gguf and ggml-*.bin.",
            &["URL"], "Next", offset,
        )
    }
}
