use ratcn::runtime::CellOffset;
use ratcn::Dialog;

use super::{model_dialog, Msg};
use crate::transcription::local_models::full_model_id;

use super::local_model_view_helpers::format_bytes;
use super::types::{LocalModelEntry, LocalModelsTui};

pub(super) struct LocalModelDownloadConfirmationDialog;

impl LocalModelDownloadConfirmationDialog {
    pub(super) fn dialog(
        entry: &LocalModelEntry,
        offset: CellOffset,
    ) -> Dialog<LocalModelsTui, Msg> {
        model_dialog(
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
