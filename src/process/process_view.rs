//! The processing-action picker, shared by `ostt process` and the recording flow.

use crate::config::file::ProcessAction;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratcn::runtime::{FocusState, Ratcn};

/// Result of the action picker interaction.
pub enum PickerResult {
    Selected(String),
    Cancelled,
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
