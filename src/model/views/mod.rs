pub(crate) mod cloud;
pub(crate) mod local;
pub(crate) mod provider;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}
