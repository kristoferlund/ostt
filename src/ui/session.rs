//! Native host for the management screens, after ratcn's demo host: it opens the
//! adaptive terminal session, paints the shared chrome, and routes input.

use crate::ui::components::{footer::render_footer, title::render_title};
use crate::ui::render_app_layout;
use ratatui::{layout::Rect, style::Style, Frame};
use ratcn::{
    runtime::{DeclareCtx, Event, EventResult, KeyCode, Ratcn},
    terminal::{Session, SessionEvent, SessionOptions},
    Theme, Toast, ToasterState, ToasterWidget,
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

/// Show `toast` in place of any toast still on screen, one at a time as on main.
pub(crate) fn toast(toasts: &mut ToasterState<'static>, toast: Toast<'static>) {
    *toasts = ToasterState::new();
    toasts.push(toast.duration(TOAST_DURATION), now());
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
        render_title(frame, layout.title, title, theme);
    }
    let body = if chrome.title.is_some() {
        layout.body
    } else {
        layout.title.union(layout.body)
    };
    ratcn.render(frame, area, state, theme, |ctx| declare(ctx, body));
    render_footer(frame, layout.footer, chrome.footer, theme);
    if let Some(toasts) = chrome.toasts {
        frame.render_widget(ToasterWidget::new(toasts, now()).themed(theme), area);
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
    let result = ratcn.handle_event(event.clone(), state);
    let copied = ratcn.take_clipboard();
    if let Some(text) = &copied {
        session.set_clipboard(text)?;
    }
    Ok(classify(event, result, copied.is_some()))
}

fn classify<M>(event: Event, result: EventResult<M>, copied: bool) -> Routed<M> {
    if is_ctrl_c(&event) && !copied {
        return Routed::Quit;
    }
    match result {
        EventResult::Emit(msg) => Routed::Msg(msg),
        EventResult::Consumed => Routed::Redraw,
        EventResult::Ignored => Routed::Ignored(event),
    }
}

fn is_ctrl_c(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.code == KeyCode::Char('c') && key.modifiers.ctrl)
}

pub(crate) fn is_cancel(event: &Event) -> bool {
    is_ctrl_c(event)
        || matches!(event, Event::Key(key) if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratcn::runtime::{KeyEvent, Modifiers};

    fn ctrl_c() -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        })
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(code.into())
    }

    /// Ctrl+C must always leave a screen unless it copied a text selection.
    #[test]
    fn ctrl_c_quits_only_when_nothing_was_copied() {
        let routed = classify(ctrl_c(), EventResult::<()>::Consumed, false);
        assert!(matches!(routed, Routed::Quit));
        let routed = classify(ctrl_c(), EventResult::<()>::Consumed, true);
        assert!(matches!(routed, Routed::Redraw));
        let routed = classify(key(KeyCode::Char('c')), EventResult::<()>::Ignored, false);
        assert!(matches!(routed, Routed::Ignored(_)));
    }

    /// A key a component consumed must not reach app shortcuts as well.
    #[test]
    fn consumed_keys_redraw_and_only_ignored_keys_pass_through() {
        assert!(matches!(
            classify(key(KeyCode::Char('x')), EventResult::<()>::Consumed, false),
            Routed::Redraw
        ));
        assert!(matches!(
            classify(key(KeyCode::Char('x')), EventResult::Emit(7), false),
            Routed::Msg(7)
        ));
        assert!(matches!(
            classify(key(KeyCode::Char('x')), EventResult::<()>::Ignored, false),
            Routed::Ignored(Event::Key(k)) if k.code == KeyCode::Char('x')
        ));
    }

    /// A stack of stale toasts would bury the latest one.
    #[test]
    fn a_new_toast_replaces_the_previous_one() {
        let mut toasts = ToasterState::new();
        toast(&mut toasts, Toast::error("first"));
        toast(&mut toasts, Toast::error("second"));

        let titles: Vec<_> = toasts
            .entries()
            .iter()
            .map(|entry| entry.toast().title())
            .collect();
        assert_eq!(titles, ["second"]);
    }

    /// Esc and q leave every management screen; other letters stay shortcuts.
    #[test]
    fn esc_and_q_cancel_but_other_letters_do_not() {
        assert!(is_cancel(&key(KeyCode::Esc)));
        assert!(is_cancel(&key(KeyCode::Char('q'))));
        assert!(!is_cancel(&key(KeyCode::Char('x'))));
    }
}
