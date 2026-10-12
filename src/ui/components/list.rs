//! List declarations. The calling command owns state, messages, and the runtime.

use ratatui::text::{Line, Text};
use ratcn::{List, ListItem, ListStyle, Theme};

/// Lists sit on the app background in every state; only the cursor row is
/// highlighted.
pub(crate) fn list_style(theme: &Theme) -> ListStyle {
    ListStyle {
        background: theme.background,
        focused_background: theme.background,
        hovered_background: theme.background,
        disabled_background: theme.background,
        ..ListStyle::from_theme(theme)
    }
}

pub(crate) fn selection_list<S: 'static, M: 'static>(
    items: &[String],
    row_height: u16,
    selected: fn(&S) -> Option<usize>,
    on_focus: fn(usize) -> M,
    on_select: fn(usize) -> M,
) -> List<usize, S, M> {
    List::new(
        items
            .iter()
            .enumerate()
            .map(|(index, label)| ListItem::new(index, label.clone())),
    )
    .item_focus(selected, move |index, _| on_focus(index))
    .selection(selected, on_select)
    .style(list_style)
    .row_height(row_height)
    // Enter acts on the row; there is nothing to mark as selected, so rows
    // carry no marker, only a one-cell indent.
    .paint_item(|_, row| {
        Text::from_iter(row.label.lines().map(|line| Line::from(format!(" {line}"))))
    })
}

pub(crate) fn clamp_selection(selected: &mut Option<usize>, len: usize) {
    *selected = (len > 0).then(|| selected.unwrap_or(0).min(len - 1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use ratcn::runtime::{Event, FocusState, Modifiers, MouseEvent, MouseKind, Ratcn};

    /// Lists blend into the app: focus and hover never tint the list itself,
    /// only the row under the cursor stands out.
    #[test]
    fn hovered_list_keeps_the_app_background_except_for_the_cursor_row() {
        struct State {
            focus: FocusState,
            selected: Option<usize>,
        }
        let theme = Theme::default_dark();
        let items: Vec<String> = ["one", "two", "three"].map(String::from).to_vec();
        let mut state = State {
            focus: FocusState::intent(["list"]),
            selected: Some(0),
        };
        let mut ratcn = Ratcn::new().focus(|s: &State| &s.focus, |focus| focus);
        let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
        let mut paint = |state: &State, ratcn: &mut Ratcn<State, FocusState>| {
            terminal
                .draw(|frame| {
                    ratcn.render(frame, frame.area(), state, &theme, |ctx| {
                        let list = selection_list(
                            &items,
                            1,
                            |s: &State| s.selected,
                            |_| FocusState::default(),
                            |_| FocusState::default(),
                        );
                        ctx.component("list", list, ctx.area());
                    });
                })
                .unwrap();
            terminal.backend().buffer().clone()
        };
        paint(&state, &mut ratcn);
        if let ratcn::runtime::EventResult::Emit(focus) = ratcn.handle_event(
            Event::Mouse(MouseEvent {
                kind: MouseKind::Moved,
                column: 5,
                row: 4,
                modifiers: Modifiers::NONE,
            }),
            &state,
        ) {
            state.focus = focus;
        }
        let buffer = paint(&state, &mut ratcn);
        for y in 1..6 {
            for x in 0..20 {
                assert_eq!(buffer[(x, y)].bg, theme.background, "cell ({x}, {y})");
            }
        }
        assert_ne!(
            buffer[(0, 0)].bg,
            theme.background,
            "the cursor row stands out"
        );
    }

    /// Enter and delete act on the selection, so it must always name an
    /// existing row, or none when the list is empty.
    #[test]
    fn clamp_selection_keeps_the_selection_on_an_existing_row() {
        let mut selected = None;
        clamp_selection(&mut selected, 3);
        assert_eq!(selected, Some(0));

        let mut selected = Some(5);
        clamp_selection(&mut selected, 2);
        assert_eq!(selected, Some(1));

        let mut selected = Some(0);
        clamp_selection(&mut selected, 0);
        assert_eq!(selected, None);
    }
}
