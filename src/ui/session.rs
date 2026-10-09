//! Native management-screen hosting, following ratcn's adaptive demo host.

use ratcn::{
    runtime::Event,
    terminal::{Session, SessionEvent, SessionOptions},
    Theme,
};
use std::{io, time::Duration};

pub(crate) fn paint_background(frame: &mut ratatui::Frame<'_>, theme: &Theme) {
    let area = frame.area();
    frame.buffer_mut().set_style(
        area,
        ratatui::style::Style::default()
            .fg(theme.foreground)
            .bg(theme.background),
    );
}

pub(crate) fn open() -> io::Result<Session> {
    Session::open(SessionOptions::new().mouse().paste().adaptive())
}

/// Theme-change and resize events owe a redraw, not an application message.
pub(crate) fn next(session: &mut Session, timeout: Option<Duration>) -> io::Result<Option<Event>> {
    match session.next(timeout)? {
        Some(SessionEvent::Input(event)) => Ok(Event::try_from(event).ok()),
        _ => Ok(None),
    }
}

pub(crate) fn theme(session: &Session) -> Theme {
    // Session queries and tracks the terminal palette; ratcn supplies the fallback
    // only when the terminal cannot report its colors.
    session.theme()
}

pub(crate) fn is_ctrl_c(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.code == ratcn::runtime::KeyCode::Char('c') && key.modifiers.ctrl)
}

pub(crate) fn is_cancel(event: &Event) -> bool {
    is_ctrl_c(event)
        || matches!(event, Event::Key(key) if matches!(key.code, ratcn::runtime::KeyCode::Esc | ratcn::runtime::KeyCode::Char('q')))
}
