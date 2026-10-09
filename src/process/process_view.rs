//! Processing-action pickers: native ratcn management UI and a plain-Ratatui recording popup.

use crate::config::file::ProcessAction;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::{is_cancel_key, render_app_layout, render_footer, render_title, scroll, session};
use crate::ui::{render_themed_footer, render_themed_title};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState},
};
use ratcn::{
    runtime::{EventResult, FocusState, Ratcn},
    terminal::Session,
};

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

#[derive(Default)]
struct State {
    focus: FocusState,
    selected: Option<usize>,
}

enum Msg {
    Focus(FocusState),
    Select(usize),
    Confirm(usize),
}

struct ProcessView {
    session: Session,
    actions: Vec<ProcessAction>,
    rows: Vec<String>,
    state: State,
    ratcn: Ratcn<State, Msg>,
}

impl ProcessView {
    fn new(actions: Vec<ProcessAction>) -> Result<Self> {
        let rows = actions.iter().map(|action| action.name.clone()).collect();
        Ok(Self {
            session: session::open()?,
            actions,
            rows,
            state: State::default(),
            ratcn: Ratcn::new().focus(|state: &State| &state.focus, Msg::Focus),
        })
    }

    fn draw(&mut self) -> Result<()> {
        clamp_selection(&mut self.state.selected, self.rows.len());
        let theme = session::theme(&self.session);
        let state = &self.state;
        let rows = &self.rows;
        let ratcn = &mut self.ratcn;
        self.session.terminal_mut().draw(|frame| {
            session::paint_background(frame, &theme);
            let layout = render_app_layout(frame, frame.area());
            render_themed_title(frame, layout.title, "Process action", &theme);
            ratcn.render(frame, layout.body, state, &theme, |ctx| {
                ctx.component(
                    "list",
                    selection_list(rows, 1, |s: &State| s.selected, Msg::Select, Msg::Confirm),
                    layout.body,
                );
            });
            render_themed_footer(
                frame,
                layout.footer,
                "↑/↓ select, ↵ confirm, esc/q cancel",
                &theme,
            );
        })?;
        self.session.set_pointer_shape(self.ratcn.pointer_shape())?;
        Ok(())
    }

    fn run(mut self) -> Result<PickerResult> {
        loop {
            self.draw()?;
            let Some(event) = session::next(&mut self.session, None)? else {
                continue;
            };
            if session::is_cancel(&event) {
                return Ok(PickerResult::Cancelled);
            }
            match self.ratcn.handle_event(event, &self.state) {
                EventResult::Emit(Msg::Focus(focus)) => self.state.focus = focus,
                EventResult::Emit(Msg::Select(index)) => self.state.selected = Some(index),
                EventResult::Emit(Msg::Confirm(index)) => {
                    return Ok(PickerResult::Selected(self.actions[index].id.clone()))
                }
                _ => {}
            }
        }
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
