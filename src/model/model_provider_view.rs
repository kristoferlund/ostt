use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Terminal,
};
use std::{io::Stdout, time::Duration};

use crate::{
    config,
    ui::{render_app_layout, render_footer, render_title},
};

use super::is_ctrl_c;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModelProviderChoice {
    Local,
    Cloud,
    Quit,
}

pub(crate) async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<ModelProviderChoice> {
    tracing::debug!("Model provider picker opened");
    let choices = ["Local models", "Cloud models"];
    let mut selected = 0_usize;

    loop {
        let current_model = config::get_selected_model_entry()?
            .map(|selected| format!("{}/{}", selected.provider_id, selected.model_id))
            .unwrap_or_else(|| "None".to_string());

        terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "Model");

            let mut items = vec![
                ListItem::new(Line::from(format!("Current model: {current_model}"))),
                ListItem::new(Line::from("")),
                ListItem::new(Line::from("Select model:")),
            ];
            items.extend(choices.iter().enumerate().map(|(i, choice)| {
                let style = if i == selected {
                    Style::default().fg(Color::White).bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(choice.to_string(), style)))
            }));

            frame.render_widget(List::new(items), layout.body);

            render_footer(frame, layout.footer, "↑↓ select, ↵ confirm, esc/q quit");
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(1),
                    KeyCode::Enter => {
                        let choice = match selected {
                            0 => ModelProviderChoice::Local,
                            _ => ModelProviderChoice::Cloud,
                        };
                        tracing::debug!("Selected model provider: {:?}", choice);
                        return Ok(choice);
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        tracing::debug!("Model provider picker cancelled");
                        return Ok(ModelProviderChoice::Quit);
                    }
                    _ if is_ctrl_c(&key) => {
                        tracing::debug!("Model provider picker cancelled via Ctrl+C");
                        return Ok(ModelProviderChoice::Quit);
                    }
                    _ => {}
                }
            }
        }
    }
}
