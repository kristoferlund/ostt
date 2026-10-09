use ratcn::runtime::CellOffset;
use ratcn::Dialog;

use super::{model_dialog, Msg};
use crate::transcription::local_models::full_model_id;

use super::local_model_view_helpers::format_bytes;
use super::types::{LocalModelEntry, LocalModelsTui};

pub(super) struct LocalModelDeleteConfirmationDialog;

impl LocalModelDeleteConfirmationDialog {
    pub(super) fn dialog(
        entry: &LocalModelEntry,
        offset: CellOffset,
    ) -> Dialog<LocalModelsTui, Msg> {
        model_dialog(
            "Confirm Delete",
            format!(
                "Delete \"{}\" ({})?\nID: {}\n\nThis cannot be undone.",
                entry.name,
                format_bytes(u64::from(entry.size_mb) * 1024 * 1024),
                full_model_id(&entry.provider_id, &entry.id)
            ),
            "Delete",
            offset,
        )
    }
}
