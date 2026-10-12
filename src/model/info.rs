//! The details page for one model.

use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use super::types::ModelEntry;
use crate::transcription::local_models::{full_model_id, model_destination};

pub(super) fn paragraph(entry: &ModelEntry) -> Paragraph<'static> {
    let path = model_destination(&entry.registry_entry())
        .display()
        .to_string();
    let mut lines = vec![
        Line::from(format!(
            "ID: {}",
            full_model_id(&entry.provider_id, &entry.id)
        )),
        Line::from(""),
        Line::from(entry.description.clone()),
        Line::from(""),
        Line::from(format!(
            "Recommended hardware: {}",
            entry.recommended_hardware.as_deref().unwrap_or("none")
        )),
        Line::from(""),
        Line::from(format!("Size (MB): {}", entry.size_mb)),
        Line::from(format!(
            "Languages: {}",
            if entry.languages.is_empty() {
                "unknown".to_string()
            } else {
                entry.languages.join(", ")
            }
        )),
        Line::from(format!("Url: {}", entry.url)),
        Line::from(format!(
            "Downloaded: {}",
            if entry.is_downloaded { "Yes" } else { "No" }
        )),
        Line::from(format!(
            "Active: {}",
            if entry.is_active { "Yes" } else { "No" }
        )),
    ];
    if entry.is_downloaded {
        lines.push(Line::from(format!("Local path: {path}")));
    }
    if let Some(sha256) = &entry.sha256 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("SHA256: {sha256}")));
    }

    Paragraph::new(lines).wrap(Wrap { trim: false })
}
