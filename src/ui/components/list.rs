//! List declarations. The calling command owns state, messages, and the runtime.

use ratatui::text::Text;
use ratcn::{List, ListItem};

pub(crate) fn selection_list<S: 'static, M: 'static>(
    items: &[String],
    row_height: u16,
    selected: fn(&S) -> Option<usize>,
    on_focus: fn(usize) -> M,
    on_select: fn(usize) -> M,
) -> List<usize, S, M> {
    let mut list = List::new(
        items
            .iter()
            .enumerate()
            .map(|(index, label)| ListItem::new(index, label.clone())),
    )
    .item_focus(selected, move |index, _| on_focus(index))
    .selection(selected, on_select)
    .row_height(row_height);
    if row_height > 1 {
        list = list.paint_item(|_, row| Text::from(row.label.to_string()));
    }
    list
}

pub(crate) fn clamp_selection(selected: &mut Option<usize>, len: usize) {
    *selected = (len > 0).then(|| selected.unwrap_or(0).min(len - 1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use ratcn::{
        runtime::{
            Event, EventResult, FocusState, KeyCode, Modifiers, MouseButton, MouseEvent,
            MouseKind as MouseEventKind, Ratcn,
        },
        Theme,
    };

    #[derive(Default)]
    struct State {
        focus: FocusState,
        selected: Option<usize>,
    }
    enum Msg {
        Focus(FocusState),
        Select(usize),
        Commit(usize),
    }

    fn paint(
        state: &mut State,
        runtime: &mut Ratcn<State, Msg>,
        terminal: &mut Terminal<TestBackend>,
        items: &[String],
        height: u16,
    ) {
        clamp_selection(&mut state.selected, items.len());
        terminal
            .draw(|frame| {
                runtime.render(frame, frame.area(), state, &Theme::default_dark(), |ctx| {
                    ctx.component(
                        "list",
                        selection_list(
                            items,
                            height,
                            |s: &State| s.selected,
                            Msg::Select,
                            Msg::Commit,
                        ),
                        ctx.area(),
                    );
                });
            })
            .unwrap();
    }

    fn runtime() -> Ratcn<State, Msg> {
        Ratcn::new().focus(|s: &State| &s.focus, Msg::Focus)
    }
    fn apply(state: &mut State, result: EventResult<Msg>) -> Option<usize> {
        match result {
            EventResult::Emit(Msg::Focus(focus)) => state.focus = focus,
            EventResult::Emit(Msg::Select(index)) => state.selected = Some(index),
            EventResult::Emit(Msg::Commit(index)) => {
                state.selected = Some(index);
                return Some(index);
            }
            _ => {}
        }
        None
    }

    #[test]
    fn long_list_keeps_the_row_visible_before_enter_commits_it() {
        let items: Vec<_> = (0..40).map(|index| format!("Row {index}")).collect();
        let mut state = State::default();
        let mut runtime = runtime();
        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        paint(&mut state, &mut runtime, &mut terminal, &items, 1);
        let result = runtime.handle_event(KeyCode::End, &state);
        apply(&mut state, result);
        paint(&mut state, &mut runtime, &mut terminal, &items, 1);
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(screen.contains("Row 39"));
        let result = runtime.handle_event(KeyCode::Enter, &state);
        assert_eq!(apply(&mut state, result), Some(39));
    }

    #[test]
    fn history_text_click_and_subsequent_key_navigation_target_the_correct_entry() {
        let items = vec![
            "first date\nfirst text".into(),
            "second date\nsecond text".into(),
        ];
        let mut state = State::default();
        let mut runtime = runtime();
        let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
        paint(&mut state, &mut runtime, &mut terminal, &items, 2);
        let mouse = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 8,
                row: 3,
                modifiers: Modifiers::NONE,
            })
        };
        let result = runtime.handle_event(mouse(MouseEventKind::Down(MouseButton::Left)), &state);
        apply(&mut state, result);
        let result = runtime.handle_event(mouse(MouseEventKind::Up(MouseButton::Left)), &state);
        assert_eq!(apply(&mut state, result), Some(1));
        paint(&mut state, &mut runtime, &mut terminal, &items, 2);
        let result = runtime.handle_event(KeyCode::Up, &state);
        apply(&mut state, result);
        let result = runtime.handle_event(KeyCode::Enter, &state);
        assert_eq!(apply(&mut state, result), Some(0));
    }

    #[test]
    fn deletion_and_empty_to_nonempty_transitions_leave_no_phantom_selection() {
        let mut state = State::default();
        let mut runtime = runtime();
        for (width, height) in [(1, 1), (30, 6)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            paint(
                &mut state,
                &mut runtime,
                &mut terminal,
                &["one".into(), "two".into()],
                1,
            );
            let result = runtime.handle_event(KeyCode::End, &state);
            apply(&mut state, result);
            paint(&mut state, &mut runtime, &mut terminal, &["one".into()], 1);
            assert_eq!(state.selected, Some(0));
            paint(&mut state, &mut runtime, &mut terminal, &[], 1);
            assert_eq!(state.selected, None);
            assert!(!matches!(
                runtime.handle_event(KeyCode::Enter, &state),
                EventResult::Emit(Msg::Commit(_))
            ));
        }
    }

    #[test]
    fn wheel_scroll_persists_across_redraws_without_changing_the_delete_target() {
        let items: Vec<_> = (0..20).map(|index| format!("Row {index}")).collect();
        let mut state = State::default();
        let mut runtime = runtime();
        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        paint(&mut state, &mut runtime, &mut terminal, &items, 1);
        let result = runtime.handle_event(
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Scroll(ratcn::runtime::ScrollDirection::Down),
                column: 8,
                row: 2,
                modifiers: Modifiers::NONE,
            }),
            &state,
        );
        apply(&mut state, result);
        paint(&mut state, &mut runtime, &mut terminal, &items, 1);
        let scrolled = terminal.backend().buffer().clone();
        paint(&mut state, &mut runtime, &mut terminal, &items, 1);
        assert_eq!(&scrolled, terminal.backend().buffer());
        assert_eq!(state.selected, Some(0));
        let screen: String = scrolled.content.iter().map(|cell| cell.symbol()).collect();
        assert!(!screen.contains("Row 0"));
    }
}
