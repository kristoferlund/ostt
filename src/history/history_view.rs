//! Transcription history with one command-owned ratcn runtime.

use crate::history::TranscriptionEntry;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratcn::{
    runtime::{FocusState, Ratcn},
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

/// Interactive history view for transcription entries. Every return restores
/// the terminal before clipboard/output work. A copy is confirmed with a toast,
/// and the view closes once it expires.
pub fn run(entries: Vec<TranscriptionEntry>) -> Result<Option<String>> {
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
    let mut session = session::open()?;
    let mut ratcn = Ratcn::new().focus(|state: &State| &state.focus, Msg::Focus);
    let mut copied = None;
    loop {
        state.toasts.prune_expired(session::now());
        if copied.is_some() && state.toasts.is_empty() {
            break;
        }
        let chrome = Chrome {
            title: Some("History"),
            footer: "↑↓ select, ↵ copy, esc/q exit",
            toasts: Some(&state.toasts),
        };
        session::draw(&mut session, &mut ratcn, &state, chrome, |ctx, body| {
            // Lists draw ordinary rows muted; the transcription keeps full
            // contrast on every row so the date always reads as secondary.
            let date = Style::default().fg(ctx.theme.muted_foreground);
            let transcription = Style::default().fg(ctx.theme.foreground);
            let list = selection_list(&rows, 2, |s: &State| s.selected, Msg::Select, Msg::Copy)
                .paint_item(move |_, row| {
                    let (created_at, text) = row.label.split_once('\n').unwrap_or((row.label, ""));
                    Text::from(vec![
                        Line::styled(format!(" {created_at}"), date),
                        Line::styled(
                            format!(" {}", text.lines().next().unwrap_or_default()),
                            transcription,
                        ),
                    ])
                });
            ctx.component("list", list, body);
        })?;
        let timeout = state.toasts.time_until_next_expiry(session::now());
        let Some(event) = session::next_event(&mut session, timeout)? else {
            continue;
        };
        match session::route(&mut session, &mut ratcn, &state, event)? {
            Routed::Msg(Msg::Focus(focus)) => state.focus = focus,
            Routed::Msg(Msg::Select(index)) => state.selected = Some(index),
            Routed::Msg(Msg::Copy(index)) => {
                state.selected = Some(index);
                copied = Some(entries[index].text.clone());
                session::toast(&mut state.toasts, Toast::success("Copied to clipboard!"));
            }
            Routed::Ignored(event) if session::is_cancel(&event) => break,
            Routed::Quit => break,
            Routed::Ignored(_) | Routed::Redraw => {}
        }
    }
    Ok(copied)
}
