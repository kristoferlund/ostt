//! Shared ratcn modal interaction for management screens.

use crossterm::event::Event;
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
    Frame,
};
use ratcn::runtime::{CellOffset, EventResult, FocusState, ModalState, Ratcn};
use ratcn::{Button, Dialog, Input, InputState, Theme};

#[derive(Default)]
pub(crate) struct ModalViewState {
    focus: FocusState,
    modals: ModalState,
    pub offset: CellOffset,
    inputs: Vec<InputState>,
}

pub(crate) enum ModalMessage {
    Focus(FocusState),
    Move(CellOffset),
    Change(usize, InputState),
    Submit(usize),
    Accept,
    Dismiss,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ModalAction {
    Changed(usize),
    Submit(usize),
    Accept,
    Dismiss,
}

pub(crate) struct ModalView {
    state: ModalViewState,
    runtime: Ratcn<ModalViewState, ModalMessage>,
    theme: Theme,
}

impl Default for ModalView {
    fn default() -> Self {
        Self {
            state: ModalViewState::default(),
            runtime: Ratcn::new()
                .focus(|state: &ModalViewState| &state.focus, ModalMessage::Focus)
                .modals(|state| &state.modals),
            theme: Theme::default_dark(),
        }
    }
}

impl ModalView {
    pub(crate) fn sync(
        &mut self,
        id: Option<&'static str>,
        inputs: &[&InputState],
    ) -> anyhow::Result<()> {
        if self.state.modals.top().map(|id| id.as_str()) != id {
            self.state.modals.close(&mut self.state.focus);
            self.state.offset = CellOffset::default();
            if let Some(id) = id {
                self.state.modals.open(id, &mut self.state.focus)?;
            }
        }
        self.state.inputs.truncate(inputs.len());
        for (index, input) in inputs.iter().enumerate() {
            if let Some(current) = self.state.inputs.get_mut(index) {
                if current.version() != input.version() {
                    *current = (*input).clone();
                }
            } else {
                self.state.inputs.push((*input).clone());
            }
        }
        Ok(())
    }

    pub(crate) fn input(&self, index: usize) -> &InputState {
        &self.state.inputs[index]
    }

    pub(crate) fn focus_input(&mut self, id: &'static str, index: usize) {
        self.state.focus = FocusState::intent([id.to_string(), format!("input-{index}")]);
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> Option<ModalAction> {
        match self.runtime.handle_event(event, &self.state) {
            EventResult::Emit(ModalMessage::Focus(focus)) => self.state.focus = focus,
            EventResult::Emit(ModalMessage::Move(offset)) => self.state.offset = offset,
            EventResult::Emit(ModalMessage::Change(index, input)) => {
                self.state.inputs[index] = input;
                return Some(ModalAction::Changed(index));
            }
            EventResult::Emit(ModalMessage::Submit(index)) => {
                return Some(ModalAction::Submit(index))
            }
            EventResult::Emit(ModalMessage::Accept) => return Some(ModalAction::Accept),
            EventResult::Emit(ModalMessage::Dismiss) => return Some(ModalAction::Dismiss),
            _ => {}
        }
        None
    }

    pub(crate) fn render(
        &mut self,
        frame: &mut Frame<'_>,
        mut dialog: impl FnMut(&ModalViewState) -> Dialog<ModalViewState, ModalMessage>,
    ) {
        let area = frame.area();
        let id = self
            .state
            .modals
            .top()
            .expect("modal is open before rendering")
            .clone();
        self.runtime
            .render(frame, area, &self.state, &self.theme, |ctx| {
                ctx.modal(id.clone(), dialog(&self.state), area);
            });
    }
}

pub(crate) fn dialog(
    title: impl Into<String>,
    description: String,
    action: &'static str,
    offset: CellOffset,
) -> Dialog<ModalViewState, ModalMessage> {
    Dialog::new()
        .title(title)
        .description(description)
        .offset(offset)
        .on_offset_change(ModalMessage::Move)
        .on_dismiss(|| ModalMessage::Dismiss)
        .action(
            "accept",
            Button::new(action).on_press(|| ModalMessage::Accept),
        )
}

pub(crate) fn form_dialog(
    title: &'static str,
    instructions: &'static str,
    labels: &'static [&'static str],
    action: &'static str,
    offset: CellOffset,
) -> Dialog<ModalViewState, ModalMessage> {
    let instructions_height = if instructions.is_empty() { 0 } else { 3 };
    dialog(title, String::new(), action, offset).content(
        instructions_height + labels.len() as u16 * 3,
        move |ctx| {
            let mut constraints = vec![Constraint::Length(instructions_height)];
            constraints.extend(labels.iter().map(|_| Constraint::Length(3)));
            let areas = Layout::vertical(constraints).split(ctx.area());
            ctx.paint_widget(Paragraph::new(instructions), areas[0]);
            for (index, label) in labels.iter().enumerate() {
                ctx.component(
                    format!("input-{index}"),
                    Input::new()
                        .title(*label)
                        .value(
                            move |state: &ModalViewState| &state.inputs[index],
                            move |value| ModalMessage::Change(index, value),
                        )
                        .on_submit(move || ModalMessage::Submit(index)),
                    areas[index + 1],
                );
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{backend::TestBackend, Terminal};

    fn key(code: KeyCode) -> Event {
        Event::Key(code.into())
    }

    fn paint(view: &mut ModalView, terminal: &mut Terminal<TestBackend>, form: bool) {
        terminal
            .draw(|frame| {
                view.render(frame, |state| {
                    if form {
                        form_dialog("Fields", "", &["Find", "Replace"], "Add", state.offset)
                    } else {
                        dialog("Download", "A model".into(), "Download", state.offset)
                    }
                })
            })
            .unwrap();
    }

    #[test]
    fn confirmation_accepts_and_dismisses_without_leaking_keys_to_background() {
        let mut view = ModalView::default();
        view.sync(Some("download"), &[]).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        paint(&mut view, &mut terminal, false);
        assert_eq!(
            view.handle_event(key(KeyCode::Enter)),
            Some(ModalAction::Accept)
        );
        assert_eq!(
            view.handle_event(key(KeyCode::Esc)),
            Some(ModalAction::Dismiss)
        );
        assert_eq!(view.handle_event(key(KeyCode::Down)), None);
    }

    #[test]
    fn confirmation_action_is_clickable_after_moving_the_dialog() {
        let mut view = ModalView::default();
        view.sync(Some("download"), &[]).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        paint(&mut view, &mut terminal, false);
        let corner = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .position(|cell| cell.symbol() == "┌")
            .unwrap();
        let (x, y) = ((corner % 100) as u16, (corner / 100) as u16);
        let mouse = |kind, column, row| {
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };
        view.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
        view.handle_event(mouse(MouseEventKind::Drag(MouseButton::Left), x + 4, y + 2));
        view.handle_event(mouse(MouseEventKind::Up(MouseButton::Left), x + 4, y + 2));
        paint(&mut view, &mut terminal, false);
        let button = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .rposition(|cell| cell.symbol() == "D")
            .unwrap();
        let (x, y) = ((button % 100) as u16, (button / 100) as u16);
        view.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
        assert_eq!(
            view.handle_event(mouse(MouseEventKind::Up(MouseButton::Left), x, y)),
            Some(ModalAction::Accept)
        );
    }

    #[test]
    fn dragging_retains_position_across_redraws_and_resets_for_new_dialogs() {
        let mut view = ModalView::default();
        view.sync(Some("download"), &[]).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        paint(&mut view, &mut terminal, false);
        let corner = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .position(|cell| cell.symbol() == "┌")
            .unwrap();
        let (x, y) = ((corner % 100) as u16, (corner / 100) as u16);
        let mouse = |kind, column, row| {
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };
        view.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
        view.handle_event(mouse(MouseEventKind::Drag(MouseButton::Left), x + 4, y + 2));
        let offset = view.state.offset;
        assert_ne!(offset, CellOffset::default());
        view.sync(Some("download"), &[]).unwrap();
        paint(&mut view, &mut terminal, false);
        assert_eq!(view.state.offset, offset);
        view.sync(None, &[]).unwrap();
        view.sync(Some("delete"), &[]).unwrap();
        assert_eq!(view.state.offset, CellOffset::default());
    }

    #[test]
    fn form_edits_unicode_and_paste_and_submits_the_focused_field() {
        let mut view = ModalView::default();
        let first = InputState::default();
        let second = InputState::new("replacement");
        view.sync(Some("replace"), &[&first, &second]).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        paint(&mut view, &mut terminal, true);
        assert_eq!(
            view.handle_event(Event::Paste("日本語\ntext".into())),
            Some(ModalAction::Changed(0))
        );
        assert_eq!(view.input(0).value(), "日本語 text");
        let first = view.input(0).clone();
        view.sync(Some("replace"), &[&first, &second]).unwrap();
        paint(&mut view, &mut terminal, true);
        assert_eq!(
            view.handle_event(key(KeyCode::Enter)),
            Some(ModalAction::Submit(0))
        );
        view.focus_input("replace", 1);
        paint(&mut view, &mut terminal, true);
        assert_eq!(
            view.handle_event(key(KeyCode::Enter)),
            Some(ModalAction::Submit(1))
        );
        assert_eq!(
            view.input(0).value(),
            "日本語 text",
            "advancing must retain the first field"
        );
    }

    #[test]
    fn fields_and_dialogs_render_without_panicking_on_small_terminals() {
        let mut view = ModalView::default();
        view.sync(
            Some("replace"),
            &[
                &InputState::new("a very long value 日本語"),
                &InputState::default(),
            ],
        )
        .unwrap();
        for (width, height) in [(1, 1), (8, 4), (40, 10), (100, 30)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            paint(&mut view, &mut terminal, true);
        }
    }
}
