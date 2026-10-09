use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use ratcn::runtime::DeclareCtx;

use crate::transcription::local_models::{full_model_id, model_destination, RegistryEntry};

use super::types::{LocalModelEntry, LocalModelsMode, LocalModelsTui};
use super::Msg;

pub(crate) struct LocalModelInfoView;

impl LocalModelInfoView {
    pub(super) fn declare(ctx: &mut DeclareCtx<'_, LocalModelsTui, Msg>, area: Rect) {
        ctx.paint(move |paint| {
            let LocalModelsMode::Info { entry } = &paint.state().mode else {
                return;
            };
            let path = local_model_path(entry);
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

            paint.widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
        });
    }
}

fn local_model_path(entry: &LocalModelEntry) -> String {
    model_destination(&RegistryEntry {
        id: entry.id.clone(),
        provider_id: entry.provider_id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        languages: entry.languages.clone(),
        size_mb: entry.size_mb,
        url: entry.url.clone(),
        recommended_hardware: entry.recommended_hardware.clone(),
        sha256: entry.sha256.clone(),
        category: entry.category.clone(),
        group_id: entry.group_id.clone(),
    })
    .display()
    .to_string()
}
