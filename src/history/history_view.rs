//! Interactive terminal UI for viewing transcription history.

use crate::history::TranscriptionEntry;
use crate::ui::components::list::ListView;
use crate::ui::{render_app_layout, render_footer, render_title, render_toast, Toast};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;

/// Interactive history view for transcription entries.
pub struct HistoryView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    entries: Vec<TranscriptionEntry>,
    rows: Vec<String>,
    list: ListView,
    notification: Option<Toast>,
    cleaned_up: bool,
}

impl HistoryView {
    /// Creates a new history view with the given entries.
    pub fn new(entries: Vec<TranscriptionEntry>) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        let rows = entries
            .iter()
            .map(|entry| {
                format!(
                    "{}\n{}",
                    entry.created_at.format("%Y-%m-%d %H:%M:%S"),
                    entry.text
                )
            })
            .collect();
        Ok(Self {
            terminal,
            entries,
            rows,
            list: ListView::default(),
            notification: None,
            cleaned_up: false,
        })
    }

    /// Runs the interactive history view loop.
    pub fn run(&mut self) -> Result<Option<String>> {
        let mut selected_text = None;
        if !self.entries.is_empty() {
            loop {
                self.draw()?;
                if self.notification.as_ref().is_some_and(Toast::is_expired) {
                    break;
                }
                if !event::poll(Duration::from_millis(50))? {
                    continue;
                }
                let event = event::read()?;
                if matches!(&event, Event::Key(key) if crate::ui::is_cancel_key(key)) {
                    break;
                }
                if let Some(index) = self.list.handle_event(event) {
                    selected_text = Some(self.entries[index].text.clone());
                    self.notification = Some(Toast::success("Copied to clipboard!"));
                }
            }
        }
        self.cleanup()?;
        Ok(selected_text)
    }

    fn draw(&mut self) -> Result<()> {
        let items = &self.rows;
        let list = &mut self.list;
        let notification = &self.notification;
        self.terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "History");
            list.render(frame, layout.body, items, 2);
            render_footer(frame, layout.footer, "↑↓ select, ↵ copy, esc/q exit");
            if let Some(toast) = notification {
                render_toast(frame, toast);
            }
        })?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        if !self.cleaned_up {
            self.cleaned_up = true;
            disable_raw_mode()?;
            execute!(
                self.terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            self.terminal.show_cursor()?;
        }
        Ok(())
    }
}

impl Drop for HistoryView {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
