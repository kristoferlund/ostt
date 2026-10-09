//! Transcription history with one command-owned ratcn runtime.

use crate::history::TranscriptionEntry;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::{
    render_app_layout, render_themed_footer, render_themed_title, render_toast, session, Toast,
};
use anyhow::Result;
use ratcn::{
    runtime::{EventResult, FocusState, Ratcn},
    terminal::Session,
};
use std::time::Duration;

#[derive(Default)]
struct State {
    focus: FocusState,
    selected: Option<usize>,
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
    notification: Option<Toast>,
}

impl HistoryView {
    pub fn new(entries: Vec<TranscriptionEntry>) -> Result<Self> {
        let rows = entries
            .iter()
            .map(|entry| {
                format!(
                    "{}\n{}",
                    entry.created_at.format("%Y-%m-%d %H:%M:%S"),
                    entry.text
                )
            })
            .collect();
        Ok(Self {
            session: session::open()?,
            entries,
            rows,
            state: State::default(),
            ratcn: Ratcn::new().focus(|state: &State| &state.focus, Msg::Focus),
            notification: None,
        })
    }

    /// Consume the view so the terminal is restored before clipboard/output work.
    pub fn run(mut self) -> Result<Option<String>> {
        let mut selected_text = None;
        if !self.entries.is_empty() {
            loop {
                self.draw()?;
                if self.notification.as_ref().is_some_and(Toast::is_expired) {
                    break;
                }
                let timeout = self
                    .notification
                    .as_ref()
                    .map(|_| Duration::from_millis(50));
                let Some(event) = session::next(&mut self.session, timeout)? else {
                    continue;
                };
                if session::is_cancel(&event) {
                    break;
                }
                match self.ratcn.handle_event(event, &self.state) {
                    EventResult::Emit(Msg::Focus(focus)) => self.state.focus = focus,
                    EventResult::Emit(Msg::Select(index)) => self.state.selected = Some(index),
                    EventResult::Emit(Msg::Copy(index)) => {
                        self.state.selected = Some(index);
                        selected_text = Some(self.entries[index].text.clone());
                        self.notification = Some(Toast::success("Copied to clipboard!"));
                    }
                    _ => {}
                }
            }
        }
        Ok(selected_text)
    }

    fn draw(&mut self) -> Result<()> {
        clamp_selection(&mut self.state.selected, self.rows.len());
        let theme = session::theme(&self.session);
        let state = &self.state;
        let ratcn = &mut self.ratcn;
        let rows = &self.rows;
        let notification = &self.notification;
        self.session.terminal_mut().draw(|frame| {
            session::paint_background(frame, &theme);
            let layout = render_app_layout(frame, frame.area());
            render_themed_title(frame, layout.title, "History", &theme);
            ratcn.render(frame, layout.body, state, &theme, |ctx| {
                ctx.component(
                    "list",
                    selection_list(rows, 2, |s: &State| s.selected, Msg::Select, Msg::Copy),
                    layout.body,
                );
            });
            render_themed_footer(
                frame,
                layout.footer,
                "↑↓ select, ↵ copy, esc/q exit",
                &theme,
            );
            if let Some(toast) = notification {
                render_toast(frame, toast, &theme);
            }
        })?;
        self.session.set_pointer_shape(self.ratcn.pointer_shape())?;
        Ok(())
    }
}
