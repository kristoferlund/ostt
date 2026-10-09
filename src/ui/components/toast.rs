use ratatui::Frame;
use ratcn::{ToasterState, ToasterWidget};
use std::time::{Duration, Instant};

const TOAST_DURATION: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
pub struct Toast {
    toasts: ToasterState<'static>,
    created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>) -> Self {
        Self::success(message)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::from_toast(ratcn::toast::Toast::success(message.into()))
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::from_toast(ratcn::toast::Toast::error(message.into()))
    }

    fn from_toast(toast: ratcn::toast::Toast<'static>) -> Self {
        let mut toasts = ToasterState::default();
        toasts.push(toast.duration(TOAST_DURATION), Duration::ZERO);
        Self {
            toasts,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= TOAST_DURATION
    }

    pub fn message(&self) -> &str {
        self.toasts.entries()[0].toast().title()
    }
}

pub fn render_toast(frame: &mut Frame<'_>, toast: &Toast) {
    let screen = frame.area();
    frame.render_widget(
        ToasterWidget::new(&toast.toasts, toast.created_at.elapsed()),
        screen,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn native_toasts_show_feedback_and_expire_after_two_seconds() {
        for mut toast in [Toast::success("Saved"), Toast::error("Failed")] {
            assert!(!toast.is_expired());
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal.draw(|frame| render_toast(frame, &toast)).unwrap();
            assert!(terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.symbol() != " "));
            toast.created_at -= TOAST_DURATION;
            assert!(toast.is_expired());
            terminal.draw(|frame| render_toast(frame, &toast)).unwrap();
            assert!(terminal
                .backend()
                .buffer()
                .content
                .iter()
                .all(|cell| cell.symbol() == " "));
        }
    }
}
