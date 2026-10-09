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
