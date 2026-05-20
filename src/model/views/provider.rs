use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

use super::{is_ctrl_c, render_footer, render_shell, render_title};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModelProviderChoice {
    Local,
    Cloud,
    Quit,
}

pub(crate) async fn choose_model_provider(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<ModelProviderChoice> {
    let choices = ["Local provider", "Cloud provider"];
    let mut selected = 0_usize;

    loop {
        terminal.draw(|frame| {
            let [body_area, footer_area, _] = render_shell(frame);
            let list_area = render_title(frame, body_area, "Provider");

            let items: Vec<ListItem> = choices
                .iter()
                .map(|choice| ListItem::new(Line::from(choice.to_string())))
                .collect();

            let mut state = ListState::default().with_selected(Some(selected));
            frame.render_stateful_widget(
                List::new(items)
                    .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray)),
                list_area,
                &mut state,
            );

            render_footer(frame, footer_area, "↑↓ select, ↵ confirm, esc/q quit");
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
