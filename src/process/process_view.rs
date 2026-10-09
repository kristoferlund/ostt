//! Processing-action pickers: native ratcn management UI and a plain-Ratatui recording popup.

use crate::config::file::ProcessAction;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::session::{self, Chrome, Routed};
use crate::ui::{is_cancel_key, render_app_layout, render_footer, render_title, scroll};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState},
};
use ratcn::runtime::{FocusState, Ratcn};

/// Keep the recording popup independent of ratcn's components and interaction runtime.
pub(crate) fn render_popup_process_view(
    frame: &mut Frame,
    area: Rect,
    actions: &[ProcessAction],
    state: &mut ListState,
) {
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

fn run_picker(actions: &[ProcessAction]) -> Result<PickerResult> {
    let rows: Vec<String> = actions.iter().map(|action| action.name.clone()).collect();
    let mut session = session::open()?;
    let mut ratcn = Ratcn::new().focus(|state: &State| &state.focus, Msg::Focus);
    let mut state = State::default();
    clamp_selection(&mut state.selected, rows.len());
    loop {
        let chrome = Chrome {
            title: Some("Process action"),
            footer: "↑/↓ select, ↵ confirm, esc/q cancel",
            toasts: None,
        };
        session::draw(&mut session, &mut ratcn, &state, chrome, |ctx, body| {
            let list = selection_list(&rows, 1, |s: &State| s.selected, Msg::Select, Msg::Confirm);
            ctx.component("list", list, body);
        })?;
        let Some(event) = session::next_event(&mut session, None)? else {
            continue;
        };
        match session::route(&mut session, &mut ratcn, &state, event)? {
            Routed::Msg(Msg::Focus(focus)) => state.focus = focus,
            Routed::Msg(Msg::Select(index)) => state.selected = Some(index),
            Routed::Msg(Msg::Confirm(index)) => {
                return Ok(PickerResult::Selected(actions[index].id.clone()))
            }
            Routed::Ignored(event) if session::is_cancel(&event) => {
                return Ok(PickerResult::Cancelled)
            }
            Routed::Quit => return Ok(PickerResult::Cancelled),
            Routed::Ignored(_) | Routed::Redraw => {}
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
        _ => run_picker(actions),
    }
}
