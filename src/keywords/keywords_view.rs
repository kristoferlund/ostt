//! Keyword management: app-owned state and messages, one runtime for list and form.

use crate::keywords::KeywordsManager;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::components::modal::{dialog, form_dialog};
use crate::ui::{render_app_layout, render_themed_footer, render_themed_title, session};
use anyhow::Result;
use ratcn::{
    runtime::{CellOffset, Event, EventResult, FocusState, KeyCode, ModalState, Ratcn},
    terminal::Session,
    Button, Input, InputState,
};

#[derive(Default)]
struct State {
    focus: FocusState,
    modals: ModalState,
    selected: Option<usize>,
    keywords: Vec<String>,
    input: InputState,
    offset: CellOffset,
}

enum Msg {
    Focus(FocusState),
    Select(usize),
    Input(InputState),
    Move(CellOffset),
    Add,
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
    /// Open the command's adaptive terminal session.
    pub fn new(keywords: Vec<String>) -> Result<Self> {
        Ok(Self {
            session: session::open()?,
            state: State {
                keywords,
                ..State::default()
            },
            ratcn: runtime(),
        })
    }

    /// Consume the view so every return restores the terminal.
    pub fn run(mut self, manager: &mut KeywordsManager) -> Result<()> {
        loop {
            self.draw()?;
            let Some(event) = session::next(&mut self.session, None)? else {
                continue;
            };
            let result = self.ratcn.handle_event(event.clone(), &self.state);
            if let Some(text) = self.ratcn.take_clipboard() {
                self.session.set_clipboard(&text)?;
            } else if session::is_ctrl_c(&event) {
                break;
            }
            match result {
                EventResult::Emit(msg) => self.update(manager, msg)?,
                EventResult::Consumed => {}
                EventResult::Ignored => match event {
                    _ if session::is_cancel(&event) => break,
                    Event::Key(key) => match key.code {
                        KeyCode::Char('a') => {
                            self.state.offset = CellOffset::default();
                            self.state.modals.open("keyword", &mut self.state.focus)?;
                        }
                        KeyCode::Char('x') | KeyCode::Delete => {
                            if let Some(index) = self.state.selected {
                                manager.remove_keyword(index)?;
                                self.state.keywords = manager.load_keywords()?;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
            }
        }
        Ok(())
    }

    fn update(&mut self, manager: &mut KeywordsManager, msg: Msg) -> Result<()> {
        match msg {
            Msg::Focus(focus) => self.state.focus = focus,
            Msg::Select(index) => self.state.selected = Some(index),
            Msg::Input(input) => self.state.input = input,
            Msg::Move(offset) => self.state.offset = offset,
            Msg::Add => {
                let value = self.state.input.value().trim();
                if !value.is_empty() {
                    manager.add_keyword(value.to_string())?;
                    self.state.keywords = manager.load_keywords()?;
                }
                self.close_form();
            }
            Msg::Dismiss => self.close_form(),
        }
        Ok(())
    }

    fn close_form(&mut self) {
        self.state.modals.close(&mut self.state.focus);
        self.state.input = InputState::default();
    }

    fn draw(&mut self) -> Result<()> {
        clamp_selection(&mut self.state.selected, self.state.keywords.len());
        let theme = session::theme(&self.session);
        let state = &self.state;
        let ratcn = &mut self.ratcn;
        self.session
            .terminal_mut()
            .draw(|frame| render_keywords(frame, state, ratcn, &theme))?;
        self.session.set_pointer_shape(self.ratcn.pointer_shape())?;
        Ok(())
    }
}

fn render_keywords(
    frame: &mut ratatui::Frame<'_>,
    state: &State,
    ratcn: &mut Ratcn<State, Msg>,
    theme: &ratcn::Theme,
) {
    session::paint_background(frame, theme);
    let area = frame.area();
    let layout = render_app_layout(frame, frame.area());
    render_themed_title(frame, layout.title, "Keywords", theme);
    ratcn.render(frame, area, state, theme, |ctx| {
        ctx.component(
            "list",
            selection_list(
                &state.keywords,
                1,
                |s: &State| s.selected,
                Msg::Select,
                Msg::Select,
            ),
            layout.body,
        );
        if state.modals.is_open("keyword") {
            let form = dialog("New Keyword", "", Button::new("Add").on_press(|| Msg::Add))
                .offset(state.offset)
                .on_offset_change(Msg::Move)
                .on_dismiss(|| Msg::Dismiss);
            ctx.modal(
                "keyword",
                form_dialog(
                    form,
                    "",
                    vec![Input::new()
                        .title("Keyword")
                        .value(|s: &State| &s.input, Msg::Input)
                        .on_submit(|| Msg::Add)],
                ),
                area,
            );
        }
    });
    render_themed_footer(
        frame,
        layout.footer,
        if state.modals.is_open("keyword") {
            "↵ add, esc cancel"
        } else {
            "↑↓ select, x/del delete, a add, esc/q exit"
        },
        theme,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn modal_edits_the_app_input_without_moving_the_background_selection() {
        let mut state = State {
            selected: Some(1),
            keywords: vec!["one".into(), "two".into()],
            focus: FocusState::intent(["list"]),
            ..State::default()
        };
        let mut ratcn = runtime();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        state.modals.open("keyword", &mut state.focus).unwrap();
        terminal
            .draw(|frame| render_keywords(frame, &state, &mut ratcn, &ratcn::Theme::default_dark()))
            .unwrap();
        let EventResult::Emit(Msg::Input(input)) =
            ratcn.handle_event(Event::Paste("日本語\nkeyword".into()), &state)
        else {
            panic!("paste must edit the form, not the list")
        };
        state.input = input;
        assert_eq!(state.input.value(), "日本語 keyword");
        assert_eq!(state.selected, Some(1));
        state.modals.close(&mut state.focus);
        assert_eq!(state.focus, FocusState::intent(["list"]));
        terminal
            .draw(|frame| render_keywords(frame, &state, &mut ratcn, &ratcn::Theme::default_dark()))
            .unwrap();
        assert!(matches!(
            ratcn.handle_event(KeyCode::Up, &state),
            EventResult::Emit(Msg::Select(0))
        ));
    }
}
