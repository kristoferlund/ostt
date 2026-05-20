use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

use crate::model::UserQuit;
use crate::ui::{render_app_layout, render_footer, render_title};

use super::is_ctrl_c;

pub(crate) struct NoCloudProvidersView;

impl NoCloudProvidersView {
    pub(crate) async fn run(
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> anyhow::Result<()> {
        tracing::debug!("No-cloud-providers view opened");
        loop {
            terminal.draw(|frame| {
                let layout = render_app_layout(frame, frame.area());
                render_title(frame, layout.title, "Cloud Provider");

                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(Span::styled(
                            "No cloud providers authenticated.",
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from("Run `ostt auth login` to add credentials."),
                    ])
                    .wrap(Wrap { trim: false }),
                    layout.body,
                );

                render_footer(frame, layout.footer, "esc/q back");
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ if is_ctrl_c(&key) => return Err(UserQuit.into()),
                        _ => {}
                    }
                }
            }
        }
    }
}
