use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::transcription::local_models::{model_destination, RegistryEntry};
use crate::ui::{render_app_layout, render_footer, render_title};

use super::types::LocalModelEntry;

pub(crate) struct LocalModelInfoView;

impl LocalModelInfoView {
    pub(crate) fn render(frame: &mut Frame<'_>, entry: &LocalModelEntry) {
        let layout = render_app_layout(frame, frame.area());
        render_title(frame, layout.title, &entry.name);

        let path = local_model_path(entry);
        let mut lines = vec![
            Line::from(format!("ID: {}", entry.id)),
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

        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            layout.body,
        );
        render_footer(frame, layout.footer, "esc/q back");
    }
}

fn local_model_path(entry: &LocalModelEntry) -> String {
    model_destination(&RegistryEntry {
        id: entry.id.clone(),
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
