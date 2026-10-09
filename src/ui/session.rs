//! Native host for the management screens, after ratcn's demo host: it opens the
//! adaptive terminal session, paints the shared chrome, and routes input.

use crate::ui::components::app_layout::app_layout;
use crate::ui::{render_app_layout, render_themed_footer, render_themed_title};
use ratatui::{layout::Rect, style::Style, Frame};
use ratcn::{
    runtime::{DeclareCtx, Event, EventResult, KeyCode, Ratcn},
    terminal::{Session, SessionEvent, SessionOptions},
    Theme, ToasterState, ToasterWidget,
};
use std::{
    io,
    sync::OnceLock,
    time::{Duration, Instant},
};

pub(crate) const TOAST_DURATION: Duration = Duration::from_secs(2);

pub(crate) fn open() -> io::Result<Session> {
    Session::open(SessionOptions::new().mouse().paste().adaptive())
}

/// Monotonic time for toast deadlines.
pub(crate) fn now() -> Duration {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed()
}

/// What a screen shows around the body it declares.
pub(crate) struct Chrome<'a> {
    /// Without a title, the body also takes the title row.
    pub title: Option<&'a str>,
    pub footer: &'a str,
    pub toasts: Option<&'a ToasterState<'static>>,
}

/// Draw one frame with the terminal's current palette.
pub(crate) fn draw<S, M>(
    session: &mut Session,
    ratcn: &mut Ratcn<S, M>,
    state: &S,
    chrome: Chrome<'_>,
    declare: impl FnMut(&mut DeclareCtx<'_, S, M>, Rect),
) -> io::Result<()> {
    let theme = session.theme();
    session
        .terminal_mut()
        .draw(|frame| render(frame, ratcn, state, &theme, chrome, declare))?;
    session.set_pointer_shape(ratcn.pointer_shape())
}

pub(crate) fn render<S, M>(
    frame: &mut Frame<'_>,
    ratcn: &mut Ratcn<S, M>,
    state: &S,
    theme: &Theme,
    chrome: Chrome<'_>,
    mut declare: impl FnMut(&mut DeclareCtx<'_, S, M>, Rect),
) {
    let area = frame.area();
    frame.buffer_mut().set_style(
        area,
        Style::default().fg(theme.foreground).bg(theme.background),
    );
    let layout = render_app_layout(frame, area);
    if let Some(title) = chrome.title {
        render_themed_title(frame, layout.title, title, theme);
    }
    let body = body_area(area, chrome.title.is_some());
    ratcn.render(frame, area, state, theme, |ctx| declare(ctx, body));
    render_themed_footer(frame, layout.footer, chrome.footer, theme);
    if let Some(toasts) = chrome.toasts {
        frame.render_widget(ToasterWidget::new(toasts, now()).themed(theme), area);
    }
}

/// The area a screen declares its body in.
pub(crate) fn body_area(area: Rect, titled: bool) -> Rect {
    let layout = app_layout(area);
    if titled {
        layout.body
    } else {
        layout.title.union(layout.body)
    }
}

/// Wait for input. Theme changes, resizes, and timeouts only owe a redraw.
pub(crate) fn next_event(
    session: &mut Session,
    timeout: Option<Duration>,
) -> io::Result<Option<Event>> {
    match session.next(timeout)? {
        Some(SessionEvent::Input(event)) => Ok(Event::try_from(event).ok()),
        _ => Ok(None),
    }
}

pub(crate) enum Routed<M> {
    Msg(M),
    /// No component wanted the event; app shortcuts may act on it.
    Ignored(Event),
    Quit,
    Redraw,
}

/// A text field copies its selection on Ctrl+C; only a Ctrl+C that copied
/// nothing quits.
pub(crate) fn route<S, M>(
    session: &mut Session,
    ratcn: &mut Ratcn<S, M>,
    state: &S,
    event: Event,
) -> io::Result<Routed<M>> {
    let quit = is_ctrl_c(&event);
    let result = ratcn.handle_event(event.clone(), state);
    match ratcn.take_clipboard() {
        Some(text) => session.set_clipboard(&text)?,
        None if quit => return Ok(Routed::Quit),
        None => {}
    }
    Ok(match result {
        EventResult::Emit(msg) => Routed::Msg(msg),
        EventResult::Consumed => Routed::Redraw,
        EventResult::Ignored => Routed::Ignored(event),
    })
}

fn is_ctrl_c(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.code == KeyCode::Char('c') && key.modifiers.ctrl)
}

pub(crate) fn is_cancel(event: &Event) -> bool {
    is_ctrl_c(event)
        || matches!(event, Event::Key(key) if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')))
}
