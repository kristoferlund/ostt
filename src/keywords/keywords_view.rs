//! Keyword management: one runtime for the list and the add form.

use crate::keywords::KeywordsManager;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::components::modal::form_dialog;
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratatui::layout::Rect;
use ratcn::{
    runtime::{CellOffset, DeclareCtx, Event, FocusState, KeyCode, ModalState, Ratcn},
    terminal::Session,
    Button, Dialog, Input, InputState,
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

/// Interactive keywords view for managing keywords.
pub struct KeywordsView {
    session: Session,
    state: State,
    ratcn: Ratcn<State, Msg>,
}

impl KeywordsView {
    pub fn new(keywords: Vec<String>) -> Result<Self> {
        let mut state = State {
            keywords,
            ..State::default()
        };
        clamp_selection(&mut state.selected, state.keywords.len());
        Ok(Self {
            session: session::open()?,
            state,
            ratcn: runtime(),
        })
    }

    /// Consume the view so every return restores the terminal.
    pub fn run(mut self, manager: &mut KeywordsManager) -> Result<()> {
        loop {
            let state = &self.state;
            session::draw(
                &mut self.session,
                &mut self.ratcn,
                state,
                chrome(state),
                |ctx, body| declare(ctx, body, state),
            )?;
            let Some(event) = session::next_event(&mut self.session, None)? else {
                continue;
            };
            let msg = match session::route(&mut self.session, &mut self.ratcn, &self.state, event)?
            {
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
            self.update(manager, msg)?;
        }
        Ok(())
    }

    fn update(&mut self, manager: &mut KeywordsManager, msg: Msg) -> Result<()> {
        let state = &mut self.state;
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
        let form = Dialog::new()
            .title("New Keyword")
            .offset(state.form_offset)
            .on_offset_change(Msg::FormMoved)
            .on_dismiss(|| Msg::Dismiss)
            .action("add", Button::new("Add").on_press(|| Msg::Add));
        let input = Input::new()
            .title("Keyword")
            .value(|s: &State| &s.input, Msg::Input)
            .on_submit(|| Msg::Add);
        ctx.modal(FORM, form_dialog(form, "", vec![input]), ctx.area());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use ratcn::runtime::EventResult;

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
        state.close_form();
        paint(&state, &mut ratcn);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Up, &state),
            EventResult::Emit(Msg::Select(0))
        ));
    }
}
