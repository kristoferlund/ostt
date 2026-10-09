//! Interactive terminal UI for managing keywords.
//!
//! Provides a scrollable list of keywords with keyboard navigation,
//! mouse support, selection, and inline editing.

use crate::keywords::KeywordsManager;
use crate::ui::components::list::ListView;
use crate::ui::components::modal::{form_dialog, ModalAction, ModalView};
use crate::ui::{render_app_layout, render_footer, render_title};
use anyhow::Result;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratcn::InputState as Input;
use std::io::{self, Stdout};

/// Interactive keywords view for managing keywords.
pub struct KeywordsView {
    /// Terminal interface
    terminal: Terminal<CrosstermBackend<Stdout>>,
    list: ListView,
    /// List of keywords
    keywords: Vec<String>,
    /// Whether in input mode
    input_mode: bool,
    /// Text input widget
    input: Input,
    modal: ModalView,
    _input_modes: ratcn::crossterm::InputModeGuard,
    /// Whether cleanup has been performed
    cleaned_up: bool,
}

impl KeywordsView {
    /// Creates a new keywords view with the given keywords.
    ///
    /// # Arguments
    /// * `keywords` - List of keywords to display
    ///
    /// # Errors
    /// - If terminal cannot be initialized
    pub fn new(keywords: Vec<String>) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let input_modes = ratcn::crossterm::InputModes::new().paste().enable()?;

        Ok(Self {
            terminal,
            list: ListView::default(),
            keywords,
            input_mode: false,
            input: Input::default(),
            modal: ModalView::default(),
            _input_modes: input_modes,
            cleaned_up: false,
        })
    }

    /// Runs the interactive keywords view loop.
    pub fn run(&mut self, manager: &mut KeywordsManager) -> Result<()> {
        loop {
            self.draw()?;

            let event = event::read()?;
            if matches!(&event, Event::Key(key) if crate::ui::is_ctrl_c(key)) {
                break;
            }
            if self.input_mode {
                match self.modal.handle_event(event) {
                    Some(ModalAction::Changed(index)) => {
                        self.input = self.modal.input(index).clone();
                        continue;
                    }
                    Some(ModalAction::Accept | ModalAction::Submit(_)) => {
                        let value = self.input.value().trim();
                        if !value.is_empty() {
                            manager.add_keyword(value.to_string())?;
                            self.keywords = manager.load_keywords()?;
                        }
                        self.reset_input();
                    }
                    Some(ModalAction::Dismiss) => self.reset_input(),
                    _ => {}
                }
                continue;
            }
            if let Event::Key(key) = &event {
                match key.code {
                    _ if crate::ui::is_cancel_key(key) => break,
                    KeyCode::Char('x') | KeyCode::Delete => {
                        self.delete_selected_keyword(manager)?
                    }
                    KeyCode::Char('a') => self.input_mode = true,
                    _ => {}
                }
            }
            self.list.handle_event(event);
        }

        self.cleanup()?;
        Ok(())
    }

    fn reset_input(&mut self) {
        self.input_mode = false;
        self.input = Input::default();
    }

    /// Deletes the currently selected keyword and keeps selection in a valid state.
    fn delete_selected_keyword(&mut self, manager: &mut KeywordsManager) -> Result<()> {
        if let Some(idx) = self.list.selected() {
            manager.remove_keyword(idx)?;
            self.keywords = manager.load_keywords()?;
        }

        Ok(())
    }

    /// Renders the current state of the keywords view.
    fn draw(&mut self) -> Result<()> {
        // Extract data before the closure to avoid borrow conflicts
        let input_mode = self.input_mode;
        let inputs = [&self.input];
        self.modal.sync(
            input_mode.then_some("keyword"),
            if input_mode { &inputs } else { &[] },
        )?;
        let keywords = &self.keywords;
        let list = &mut self.list;
        let modal = &mut self.modal;

        self.terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "Keywords");
            list.render(frame, layout.body, keywords, 1);

            if input_mode {
                modal.render(frame, |state| {
                    form_dialog("New Keyword", "", &["Keyword"], "Add", state.offset)
                });
                render_footer(frame, layout.footer, "↵ add, esc cancel");
            } else {
                render_footer(
                    frame,
                    layout.footer,
                    "↑↓ select, x/del delete, a add, esc/q exit",
                );
            }
        })?;

        Ok(())
    }

    /// Cleans up terminal.
    fn cleanup(&mut self) -> Result<()> {
        if self.cleaned_up {
            return Ok(());
        }

        self.cleaned_up = true;

        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for KeywordsView {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
