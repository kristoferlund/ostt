//! Native ratcn list interaction shared by management screens.

use crossterm::event::Event;
use ratatui::{layout::Rect, text::Text, Frame};
use ratcn::runtime::{EventResult, FocusState, Ratcn};
use ratcn::{List, ListItem, Theme};

#[derive(Default)]
struct ListState {
    focus: FocusState,
    item: Option<usize>,
}

enum Message {
    Focus(FocusState),
    Move(usize),
    Select(usize),
}

pub(crate) struct ListView {
    state: ListState,
    runtime: Ratcn<ListState, Message>,
    theme: Theme,
}

impl Default for ListView {
    fn default() -> Self {
        Self {
            state: ListState::default(),
            runtime: Ratcn::new().focus(|state: &ListState| &state.focus, Message::Focus),
            theme: Theme::default_dark(),
        }
    }
}

impl ListView {
    pub(crate) fn selected(&self) -> Option<usize> {
        self.state.item
    }

    /// Return a committed row; focus, navigation, scrolling and hit-testing stay in ratcn.
    pub(crate) fn handle_event(&mut self, event: Event) -> Option<usize> {
        match self.runtime.handle_event(event, &self.state) {
            EventResult::Emit(Message::Focus(focus)) => self.state.focus = focus,
            EventResult::Emit(Message::Move(index)) => self.state.item = Some(index),
            EventResult::Emit(Message::Select(index)) => {
                self.state.item = Some(index);
                return Some(index);
            }
            _ => {}
        }
        None
    }

    pub(crate) fn render(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        items: &[String],
        row_height: u16,
    ) {
        self.state.item =
            (!items.is_empty()).then(|| self.state.item.unwrap_or(0).min(items.len() - 1));
        self.runtime
            .render(frame, area, &self.state, &self.theme, |ctx| {
                let mut list = List::new(
                    items
                        .iter()
                        .enumerate()
                        .map(|(index, label)| ListItem::new(index, label.clone())),
                )
                .item_focus(
                    |state: &ListState| state.item,
                    |index, _| Message::Move(index),
                )
                .selection(|state| state.item, Message::Select)
                .row_height(row_height);
                if row_height > 1 {
                    list = list.paint_item(|_, row| Text::from(row.label.to_string()));
                }
                ctx.component("list", list, area);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{backend::TestBackend, Terminal};

    fn paint(
        view: &mut ListView,
        terminal: &mut Terminal<TestBackend>,
        items: &[String],
        height: u16,
    ) {
        terminal
            .draw(|frame| view.render(frame, frame.area(), items, height))
            .unwrap();
    }

    #[test]
    fn navigation_keeps_long_list_selection_visible_and_enter_commits_it() {
        let items: Vec<_> = (0..40).map(|index| format!("Row {index}")).collect();
        let mut view = ListView::default();
        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        paint(&mut view, &mut terminal, &items, 1);
        view.handle_event(Event::Key(KeyCode::End.into()));
        paint(&mut view, &mut terminal, &items, 1);
        assert_eq!(view.selected(), Some(39));
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            screen.contains("Row 39"),
            "the committed row must be visible before activation"
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyCode::Enter.into())),
            Some(39)
        );
    }

    #[test]
    fn clicking_history_text_commits_the_same_entry_as_its_timestamp() {
        let items = vec![
            "first date\nfirst text".into(),
            "second date\nsecond text".into(),
        ];
        let mut view = ListView::default();
        let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
        paint(&mut view, &mut terminal, &items, 2);
        let mouse = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 8,
                row: 3,
                modifiers: KeyModifiers::NONE,
            })
        };
        view.handle_event(mouse(MouseEventKind::Down(MouseButton::Left)));
        assert_eq!(
            view.handle_event(mouse(MouseEventKind::Up(MouseButton::Left))),
            Some(1)
        );
        paint(&mut view, &mut terminal, &items, 2);
        view.handle_event(Event::Key(KeyCode::Up.into()));
        assert_eq!(
            view.handle_event(Event::Key(KeyCode::Enter.into())),
            Some(0)
        );
    }

    #[test]
    fn native_wheel_scroll_survives_redraw_without_moving_the_cursor() {
        let items: Vec<_> = (0..20).map(|index| format!("Row {index}")).collect();
        let mut view = ListView::default();
        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        paint(&mut view, &mut terminal, &items, 1);
        view.handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 8,
            row: 2,
            modifiers: KeyModifiers::NONE,
        }));
        paint(&mut view, &mut terminal, &items, 1);
        let scrolled = terminal.backend().buffer().clone();
        paint(&mut view, &mut terminal, &items, 1);
        assert_eq!(&scrolled, terminal.backend().buffer());
        assert_eq!(
            view.selected(),
            Some(0),
            "wheel scrolling must not silently change the item a delete action targets"
        );
        let screen: String = scrolled.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            !screen.contains("Row 0"),
            "native scrolling must not snap back to the cursor on redraw"
        );
    }

    #[test]
    fn deleting_selected_rows_never_leaves_an_invalid_or_phantom_selection() {
        let mut view = ListView::default();
        for (width, height) in [(1, 1), (30, 6)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            paint(&mut view, &mut terminal, &["one".into(), "two".into()], 1);
            view.handle_event(Event::Key(KeyCode::End.into()));
            paint(&mut view, &mut terminal, &["one".into()], 1);
            assert_eq!(view.selected(), Some(0));
            paint(&mut view, &mut terminal, &[], 1);
            assert_eq!(view.selected(), None);
            assert_eq!(view.handle_event(Event::Key(KeyCode::Enter.into())), None);
        }
    }
}
