//! Keyword management: one runtime for the list and the add form.

use crate::keywords::KeywordsManager;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::components::modal::form_dialog;
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratatui::layout::Rect;
use ratcn::{
    runtime::{CellOffset, DeclareCtx, Event, FocusState, KeyCode, ModalState, Ratcn},
    Input, InputState,
};

const FORM: &str = "keyword";

#[derive(Default)]
struct State {
    focus: FocusState,
    modals: ModalState,
    form_offset: CellOffset,
    keywords: Vec<String>,
    selected: Option<usize>,
    input: InputState,
}

enum Msg {
    Focus(FocusState),
    Select(usize),
    Input(InputState),
    FormMoved(CellOffset),
    Open,
    Add,
    Delete,
    Dismiss,
}

fn runtime() -> Ratcn<State, Msg> {
    Ratcn::new()
        .focus(|state: &State| &state.focus, Msg::Focus)
        .modals(|state| &state.modals)
}

/// Interactive keywords view for managing keywords. Every return restores the terminal.
pub fn run(manager: &mut KeywordsManager) -> Result<()> {
    let mut state = State {
        keywords: manager.load_keywords()?,
        ..State::default()
    };
    clamp_selection(&mut state.selected, state.keywords.len());
    let mut session = session::open()?;
    let mut ratcn = runtime();
    loop {
        session::draw(
            &mut session,
            &mut ratcn,
            &state,
            chrome(&state),
            |ctx, body| declare(ctx, body, &state),
        )?;
        let Some(event) = session::next_event(&mut session, None)? else {
            continue;
        };
        let msg = match session::route(&mut session, &mut ratcn, &state, event)? {
            Routed::Msg(msg) => msg,
            Routed::Ignored(event) if session::is_cancel(&event) => break,
            Routed::Ignored(Event::Key(key)) => match key.code {
                KeyCode::Char('a') => Msg::Open,
                KeyCode::Char('x') | KeyCode::Delete => Msg::Delete,
                _ => continue,
            },
            Routed::Ignored(_) | Routed::Redraw => continue,
            Routed::Quit => break,
        };
        update(&mut state, manager, msg)?;
    }
    Ok(())
}

fn update(state: &mut State, manager: &mut KeywordsManager, msg: Msg) -> Result<()> {
    match msg {
        Msg::Focus(focus) => state.focus = focus,
        Msg::Select(index) => state.selected = Some(index),
        Msg::Input(input) => state.input = input,
        Msg::FormMoved(offset) => state.form_offset = offset,
        Msg::Open => {
            state.form_offset = CellOffset::default();
            state.modals.open(FORM, &mut state.focus)?;
        }
        Msg::Add => {
            let value = state.input.value().trim();
            if !value.is_empty() {
                manager.add_keyword(value.to_string())?;
                state.keywords = manager.load_keywords()?;
            }
            state.close_form();
        }
        Msg::Delete => {
            if let Some(index) = state.selected {
                manager.remove_keyword(index)?;
                state.keywords = manager.load_keywords()?;
            }
        }
        Msg::Dismiss => state.close_form(),
    }
    clamp_selection(&mut state.selected, state.keywords.len());
    Ok(())
}

impl State {
    fn close_form(&mut self) {
        self.modals.close(&mut self.focus);
        self.input = InputState::default();
    }
}

fn chrome(state: &State) -> Chrome<'static> {
    Chrome {
        title: Some("Keywords"),
        footer: if state.modals.is_open(FORM) {
            "↵ add, esc cancel"
        } else {
            "↑↓ select, x/del delete, a add, esc/q exit"
        },
        toasts: None,
    }
}

fn declare(ctx: &mut DeclareCtx<'_, State, Msg>, body: Rect, state: &State) {
    ctx.component(
        "list",
        selection_list(
            &state.keywords,
            1,
            |s: &State| s.selected,
            Msg::Select,
            Msg::Select,
        ),
        body,
    );
    if state.modals.is_open(FORM) {
        let input = Input::new()
            .value(|s: &State| &s.input, Msg::Input)
            .on_submit(|| Msg::Add);
        let form = form_dialog("New keyword", "", vec![("Keyword", input)], "Add", || {
            Msg::Add
        })
        .offset(state.form_offset)
        .on_offset_change(Msg::FormMoved)
        .on_dismiss(|| Msg::Dismiss);
        ctx.modal(FORM, form, ctx.area());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use ratcn::runtime::EventResult;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn with_manager(test: impl FnOnce(&mut KeywordsManager)) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-keywords-view-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        test(&mut KeywordsManager::new(&dir).unwrap());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Adding from the form must persist the keyword and leave a fresh, closed
    /// form. Typed whitespace is trimmed, so re-adding a keyword never duplicates it.
    #[test]
    fn add_saves_the_keyword_and_closes_the_form() {
        with_manager(|manager| {
            manager.save_keywords(&["ostt".into()]).unwrap();
            let mut state = State::default();
            update(&mut state, manager, Msg::Open).unwrap();
            update(&mut state, manager, Msg::Input(InputState::new(" ostt "))).unwrap();
            update(&mut state, manager, Msg::Add).unwrap();
            update(&mut state, manager, Msg::Open).unwrap();
            update(&mut state, manager, Msg::Input(InputState::new("api"))).unwrap();
            update(&mut state, manager, Msg::Add).unwrap();

            assert_eq!(manager.load_keywords().unwrap(), ["ostt", "api"]);
            assert_eq!(state.keywords, ["ostt", "api"]);
            assert!(!state.modals.is_open(FORM));
            assert_eq!(state.input.value(), "");
        });
    }

    /// Deleting removes the selected keyword, and the selection must not point
    /// past the shortened list or the next delete would miss.
    #[test]
    fn delete_removes_the_selected_keyword_and_keeps_selection_in_range() {
        with_manager(|manager| {
            manager
                .save_keywords(&["one".into(), "two".into()])
                .unwrap();
            let mut state = State {
                keywords: manager.load_keywords().unwrap(),
                selected: Some(1),
                ..State::default()
            };
            update(&mut state, manager, Msg::Delete).unwrap();

            assert_eq!(manager.load_keywords().unwrap(), ["one"]);
            assert_eq!(state.keywords, ["one"]);
            assert_eq!(state.selected, Some(0));
        });
    }

    /// Shortcut letters are text while the form is open, and closing it hands
    /// the keys back to the list the user left.
    #[test]
    fn open_form_captures_shortcut_keys_and_returns_focus_to_the_list() {
        let mut state = State {
            keywords: vec!["one".into(), "two".into()],
            selected: Some(1),
            focus: FocusState::intent(["list"]),
            ..State::default()
        };
        let mut ratcn = runtime();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut paint = |state: &State, ratcn: &mut Ratcn<State, Msg>| {
            terminal
                .draw(|frame| {
                    session::render(
                        frame,
                        ratcn,
                        state,
                        &ratcn::Theme::default_dark(),
                        chrome(state),
                        |ctx, body| declare(ctx, body, state),
                    )
                })
                .unwrap();
        };
        state.modals.open(FORM, &mut state.focus).unwrap();
        paint(&state, &mut ratcn);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Char('x'), &state),
            EventResult::Emit(Msg::Input(_))
        ));
        with_manager(|manager| update(&mut state, manager, Msg::Dismiss).unwrap());
        assert_eq!(state.input.value(), "");
        paint(&state, &mut ratcn);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Up, &state),
            EventResult::Emit(Msg::Select(0))
        ));
    }
}
