use ratatui::widgets::ListState;

pub(crate) const DEFAULT_SCROLL_MARGIN: usize = 3;

pub(crate) fn keep_selected_in_view(
    list_state: &mut ListState,
    viewport_item_count: usize,
    item_count: usize,
) {
    let selected = list_state.selected();
    update_scroll_offset(
        list_state.offset_mut(),
        selected,
        viewport_item_count,
        item_count,
        DEFAULT_SCROLL_MARGIN,
    );
}

pub(crate) fn update_scroll_offset(
    offset: &mut usize,
    selected_index: Option<usize>,
    viewport_item_count: usize,
    item_count: usize,
    scroll_margin: usize,
) {
    let Some(selected_index) = selected_index else {
        *offset = 0;
        return;
    };
    if viewport_item_count == 0 || item_count <= viewport_item_count {
        *offset = 0;
        return;
    }

    let max_offset = item_count.saturating_sub(viewport_item_count);
    let margin = scroll_margin.min(viewport_item_count.saturating_sub(1) / 2);
    let top_margin = offset.saturating_add(margin);
    let bottom_margin = offset
        .saturating_add(viewport_item_count)
        .saturating_sub(1)
        .saturating_sub(margin);

    if selected_index < top_margin {
        *offset = selected_index.saturating_sub(margin).min(max_offset);
    } else if selected_index > bottom_margin {
        *offset = selected_index
            .saturating_add(margin)
            .saturating_add(1)
            .saturating_sub(viewport_item_count)
            .min(max_offset);
    } else {
        *offset = (*offset).min(max_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::update_scroll_offset;

    #[test]
    fn scroll_offset_keeps_selection_inside_margin_when_moving_down() {
        let mut offset = 0;

        update_scroll_offset(&mut offset, Some(6), 10, 30, 3);

        assert_eq!(offset, 0);

        update_scroll_offset(&mut offset, Some(7), 10, 30, 3);

        assert_eq!(offset, 1);
    }

    #[test]
    fn scroll_offset_keeps_selection_inside_margin_when_moving_up() {
        let mut offset = 10;

        update_scroll_offset(&mut offset, Some(13), 10, 30, 3);

        assert_eq!(offset, 10);

        update_scroll_offset(&mut offset, Some(12), 10, 30, 3);

        assert_eq!(offset, 9);
    }

    #[test]
    fn scroll_offset_clamps_to_list_bounds() {
        let mut offset = 20;

        update_scroll_offset(&mut offset, Some(29), 10, 30, 3);

        assert_eq!(offset, 20);

        update_scroll_offset(&mut offset, Some(0), 10, 30, 3);

        assert_eq!(offset, 0);
    }

    #[test]
    fn scroll_offset_resets_when_list_fits_viewport() {
        let mut offset = 5;

        update_scroll_offset(&mut offset, Some(3), 10, 8, 3);

        assert_eq!(offset, 0);
    }
}
