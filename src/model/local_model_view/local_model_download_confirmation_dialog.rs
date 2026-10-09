use ratcn::runtime::CellOffset;
use ratcn::Dialog;

use crate::transcription::local_models::full_model_id;
use crate::ui::components::modal::{dialog, ModalMessage, ModalViewState};

use super::local_model_view_helpers::format_bytes;
use super::types::LocalModelEntry;

pub(super) struct LocalModelDownloadConfirmationDialog;

impl LocalModelDownloadConfirmationDialog {
    pub(super) fn dialog(
        entry: &LocalModelEntry,
        offset: CellOffset,
    ) -> Dialog<ModalViewState, ModalMessage> {
        dialog(
            "Start Download",
            format!(
                "Download \"{}\"?\nID: {}\n\nSize: {}",
                entry.name,
                full_model_id(&entry.provider_id, &entry.id),
                format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
            ),
            "Download",
            offset,
        )
    }
}
