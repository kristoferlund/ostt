use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::ui::{render_app_layout, render_footer, render_title};

use super::cloud_model_view::CloudModelEntry;

pub(crate) struct CloudModelInfoView;

impl CloudModelInfoView {
    pub(crate) fn render(frame: &mut Frame<'_>, entry: &CloudModelEntry) {
        let layout = render_app_layout(frame, frame.area());
        render_title(frame, layout.title, "Cloud Model Info");

        let lines = vec![
            Line::from(format!("ID: {}", entry.model_id)),
            Line::from(format!("Name: {}", entry.name)),
            Line::from(""),
            Line::from(entry.description.clone()),
            Line::from(""),
            Line::from(format!(
                "Languages: {}",
                if entry.languages.is_empty() {
                    "unknown".to_string()
                } else {
                    entry.languages.join(", ")
                }
            )),
            Line::from(format!(
                "Active: {}",
                if entry.is_active { "Yes" } else { "No" }
            )),
        ];

        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            layout.body,
        );
        render_footer(frame, layout.footer, "esc/q back");
    }
}
