//! Transcription history with one command-owned ratcn runtime.

use crate::history::TranscriptionEntry;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratcn::{
    runtime::{FocusState, Ratcn},
    terminal::Session,
    Toast, ToasterState,
};

#[derive(Default)]
struct State {
    focus: FocusState,
    selected: Option<usize>,
    toasts: ToasterState<'static>,
}

enum Msg {
    Focus(FocusState),
    Select(usize),
    Copy(usize),
}

/// Interactive history view for transcription entries.
pub struct HistoryView {
    session: Session,
    entries: Vec<TranscriptionEntry>,
    rows: Vec<String>,
    state: State,
    ratcn: Ratcn<State, Msg>,
}

impl HistoryView {
    pub fn new(entries: Vec<TranscriptionEntry>) -> Result<Self> {
        let rows: Vec<String> = entries
            .iter()
            .map(|entry| {
                format!(
                    "{}\n{}",
                    entry.created_at.format("%Y-%m-%d %H:%M:%S"),
                    entry.text
                )
            })
            .collect();
        let mut state = State::default();
        clamp_selection(&mut state.selected, rows.len());
        Ok(Self {
            session: session::open()?,
            entries,
            rows,
            state,
            ratcn: Ratcn::new().focus(|state: &State| &state.focus, Msg::Focus),
        })
    }

    /// Consume the view so the terminal is restored before clipboard/output work.
    /// A copy is confirmed with a toast, and the view closes once it expires.
    pub fn run(mut self) -> Result<Option<String>> {
        let mut copied = None;
        loop {
            self.state.toasts.prune_expired(session::now());
            if copied.is_some() && self.state.toasts.is_empty() {
                break;
            }
            let rows = &self.rows;
            let chrome = Chrome {
                title: Some("History"),
                footer: "↑↓ select, ↵ copy, esc/q exit",
                toasts: Some(&self.state.toasts),
            };
            session::draw(
                &mut self.session,
                &mut self.ratcn,
                &self.state,
                chrome,
                |ctx, body| {
                    let list =
                        selection_list(rows, 2, |s: &State| s.selected, Msg::Select, Msg::Copy);
                    ctx.component("list", list, body);
                },
            )?;
            let timeout = self.state.toasts.time_until_next_expiry(session::now());
            let Some(event) = session::next_event(&mut self.session, timeout)? else {
                continue;
            };
            match session::route(&mut self.session, &mut self.ratcn, &self.state, event)? {
                Routed::Msg(Msg::Focus(focus)) => self.state.focus = focus,
                Routed::Msg(Msg::Select(index)) => self.state.selected = Some(index),
                Routed::Msg(Msg::Copy(index)) => {
                    self.state.selected = Some(index);
                    copied = Some(self.entries[index].text.clone());
                    self.state.toasts.push(
                        Toast::success("Copied to clipboard!").duration(session::TOAST_DURATION),
                        session::now(),
                    );
                }
                Routed::Ignored(event) if session::is_cancel(&event) => break,
                Routed::Quit => break,
                Routed::Ignored(_) | Routed::Redraw => {}
            }
        }
        Ok(copied)
    }
}
