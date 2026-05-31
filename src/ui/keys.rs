use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

pub(crate) fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub(crate) fn is_cancel_key(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) || is_ctrl_c(key)
}

pub(crate) fn cancel_requested() -> bool {
    if !event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        return false;
    }

    matches!(event::read(), Ok(Event::Key(key)) if is_cancel_key(&key))
}
