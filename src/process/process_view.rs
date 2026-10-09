//! Processing-action pickers: native ratcn management UI and a plain-Ratatui recording popup.

use crate::config::file::ProcessAction;
use crate::ui::components::list::ListView;
use crate::ui::{is_cancel_key, render_app_layout, render_footer, render_title, scroll};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState},
};
use std::io::{self, Stdout};

/// Keep the recording popup independent of ratcn's components and interaction runtime.
pub(crate) fn render_popup_process_view(
    frame: &mut Frame,
    area: Rect,
    actions: &[ProcessAction],
    state: &mut ListState,
) -> Rect {
    let layout = render_app_layout(frame, area);
    render_title(frame, layout.title, "Process action");
    scroll::keep_selected_in_view(state, layout.body.height as usize, actions.len());
    frame.render_stateful_widget(
        List::new(
            actions
                .iter()
                .map(|action| ListItem::new(action.name.clone())),
        )
        .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        layout.body,
        state,
    );
    render_footer(frame, layout.footer, "↑/↓ select, ↵ confirm, esc/q cancel");
    layout.body
}

/// Result of the action picker interaction.
pub enum PickerResult {
    Selected(String),
    Cancelled,
}

/// The recording popup retains its existing plain-Ratatui keyboard and wheel handling.
pub(crate) fn handle_picker_event(
    event: Event,
    actions: &[ProcessAction],
    state: &mut ListState,
) -> Option<PickerResult> {
    match event {
        Event::Key(key) => match key.code {
            _ if is_cancel_key(&key) => Some(PickerResult::Cancelled),
            KeyCode::Up | KeyCode::Char('k') => {
                state.select_previous();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.select_next();
                None
            }
            KeyCode::Enter => state
                .selected()
                .and_then(|index| actions.get(index))
                .map(|action| PickerResult::Selected(action.id.clone())),
            _ => None,
        },
        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::ScrollUp => state.select_previous(),
                MouseEventKind::ScrollDown => state.select_next(),
                _ => {}
            }
            None
        }
        _ => None,
    }
}

struct ProcessView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    actions: Vec<ProcessAction>,
    list: ListView,
    cleaned_up: bool,
}

impl ProcessView {
    fn new(actions: Vec<ProcessAction>) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self {
            terminal,
            actions,
            list: ListView::default(),
            cleaned_up: false,
        })
    }

    fn draw(&mut self) -> Result<()> {
        let items: Vec<_> = self
            .actions
            .iter()
            .map(|action| action.name.clone())
            .collect();
        let list = &mut self.list;
        self.terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "Process action");
            list.render(frame, layout.body, &items, 1);
            render_footer(frame, layout.footer, "↑/↓ select, ↵ confirm, esc/q cancel");
        })?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        if !self.cleaned_up {
            self.cleaned_up = true;
            disable_raw_mode()?;
            execute!(
                self.terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            self.terminal.show_cursor()?;
        }
        Ok(())
    }

    fn run(&mut self) -> Result<PickerResult> {
        let result = loop {
            self.draw()?;
            let event = event::read()?;
            if matches!(&event, Event::Key(key) if is_cancel_key(key)) {
                break PickerResult::Cancelled;
            }
            if let Some(index) = self.list.handle_event(event) {
                break PickerResult::Selected(self.actions[index].id.clone());
            }
        };
        self.cleanup()?;
        Ok(result)
    }
}

impl Drop for ProcessView {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Show the action picker, skipping it when only one action is configured.
pub fn show_action_picker(actions: &[ProcessAction]) -> Result<PickerResult> {
    match actions {
        [] => anyhow::bail!(
            "No processing actions configured. Add actions to ~/.config/ostt/ostt.toml"
        ),
        [action] => Ok(PickerResult::Selected(action.id.clone())),
        _ => ProcessView::new(actions.to_vec())?.run(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::file::ActionDetails;
    use ratatui::backend::TestBackend;

    #[test]
    fn recording_popup_keeps_its_plain_ratatui_selection_and_navigation() {
        let actions: Vec<_> = (0..10)
            .map(|index| ProcessAction {
                id: format!("action-{index}"),
                name: format!("Action {index}"),
                details: ActionDetails::Bash {
                    command: "cat".to_string(),
                },
            })
            .collect();
        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        let mut state = ListState::default().with_selected(Some(0));
        handle_picker_event(Event::Key(KeyCode::Down.into()), &actions, &mut state);
        let mut area = Rect::default();
        terminal
            .draw(|frame| {
                area = render_popup_process_view(frame, frame.area(), &actions, &mut state)
            })
            .unwrap();
        let y = area.y + (state.selected().unwrap() - state.offset()) as u16;
        assert!((area.x..area.right())
            .all(|x| terminal.backend().buffer()[(x, y)].bg == Color::DarkGray));
        assert!(
            matches!(handle_picker_event(Event::Key(KeyCode::Enter.into()), &actions, &mut state), Some(PickerResult::Selected(id)) if id == "action-1")
        );
    }
}
