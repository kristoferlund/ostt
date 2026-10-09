use super::types::{LocalModelsMode, LocalModelsTui};
use super::{model_dialog, Msg};
use crate::ui::components::modal::form_dialog;
use ratcn::{runtime::CellOffset, Dialog, Input};

pub(super) struct CustomModelUrlInputDialog;

impl CustomModelUrlInputDialog {
    pub(super) fn dialog(offset: CellOffset) -> Dialog<LocalModelsTui, Msg> {
        form_dialog(
            model_dialog("Download Custom Whisper Model 1/3", "", "Next", offset),
            "Paste a Hugging Face model page or a direct model file URL.\nSupported files: .gguf and ggml-*.bin.",
            vec![Input::new().title("URL").value(|state: &LocalModelsTui| match &state.mode {
                LocalModelsMode::CustomModelInput { input } => input,
                _ => unreachable!("URL field is only declared in URL mode"),
            }, Msg::Url).on_submit(|| Msg::Accept)],
        )
    }
}
