use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::Line,
    widgets::{List, ListItem, ListState},
    Terminal,
};
use std::{io::Stdout, time::Duration};

use crate::ui::{render_app_layout, render_footer, render_title};

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
    let choices = ["Local provider", "Cloud provider"];
    let mut selected = 0_usize;

    loop {
        terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "Provider");

            let items: Vec<ListItem> = choices
                .iter()
                .map(|choice| ListItem::new(Line::from(choice.to_string())))
                .collect();

            let mut state = ListState::default().with_selected(Some(selected));
            frame.render_stateful_widget(
                List::new(items)
                    .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray)),
                layout.body,
                &mut state,
            );

            render_footer(frame, layout.footer, "↑↓ select, ↵ confirm, esc/q quit");
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(1),
                    KeyCode::Enter => {
                        return Ok(match selected {
                            0 => ModelProviderChoice::Local,
                            _ => ModelProviderChoice::Cloud,
                        })
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(ModelProviderChoice::Quit),
                    _ if is_ctrl_c(&key) => return Ok(ModelProviderChoice::Quit),
                    _ => {}
                }
            }
        }
    }
}
