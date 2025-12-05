//! Interactive terminal UI for managing keywords.
//!
//! Provides a scrollable list of keywords with keyboard navigation,
//! mouse support, selection, and inline editing.

use crate::keywords::KeywordsManager;
use anyhow::Result;
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph},
};
use std::io::{self, Stdout};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

/// Common colors/styles.
const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(255, 255, 255);
const HIGHLIGHT_BG: Color = Color::Rgb(20, 20, 20);
const HELP_FG: Color = Color::Rgb(100, 100, 100);

/// Interactive keywords viewer for managing keywords.
pub struct KeywordsViewer {
    /// Terminal interface
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// List state for managing selection and scroll
    list_state: ListState,
    /// List of keywords
    keywords: Vec<String>,
    /// Whether in input mode
    input_mode: bool,
    /// Text input widget
    input: Input,
    /// Whether cleanup has been performed
    cleaned_up: bool,
}

impl KeywordsViewer {
    /// Creates a new keywords viewer with the given keywords.
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

        let mut list_state = ListState::default();
        if !keywords.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            terminal,
            list_state,
            keywords,
            input_mode: false,
            input: Input::default(),
            cleaned_up: false,
        })
    }

    /// Runs the interactive keywords viewer loop.
    pub fn run(&mut self, manager: &mut KeywordsManager) -> Result<()> {
        loop {
            self.draw()?;

            match event::read()? {
                Event::Key(key) => {
                    if self.input_mode {
                        if self.handle_input_mode_key(manager, key)? {
                            break;
                        }
                    } else if self.handle_normal_mode_key(manager, key)? {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    if !self.input_mode {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => {
                                self.list_state.select_previous();
                            }
                            MouseEventKind::ScrollDown => {
                                self.list_state.select_next();
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        self.cleanup()?;
        Ok(())
    }

    /// Handle key events while *not* in input mode.
    ///
    /// Returns `Ok(true)` if the UI should quit.
    fn handle_normal_mode_key(
        &mut self,
        manager: &mut KeywordsManager,
        key: KeyEvent,
    ) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Up => {
                self.list_state.select_previous();
            }
            KeyCode::Down => {
                self.list_state.select_next();
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                self.delete_selected_keyword(manager)?;
            }
            KeyCode::Char('a') => {
                self.input_mode = true;
            }
            _ => {}
        }
        Ok(false)
    }

    /// Handle key events while in input mode.
    ///
    /// Returns `Ok(true)` if the UI should quit (never happens here, but
    /// kept for symmetry with `handle_normal_mode_key`).
    fn handle_input_mode_key(
        &mut self,
        manager: &mut KeywordsManager,
        key: KeyEvent,
    ) -> Result<bool> {
        match key.code {
            KeyCode::Enter => {
                let value = self.input.value().trim();
                if !value.is_empty() {
                    manager.add_keyword(value.to_string())?;
                    self.refresh_keywords(manager)?;
                }
                self.input_mode = false;
                self.input = Input::default();
            }
            KeyCode::Esc => {
                self.input_mode = false;
                self.input = Input::default();
            }
            _ => {
                // Handle all other keys with tui_input
                let ev = Event::Key(key);
                self.input.handle_event(&ev);
            }
        }
        Ok(false)
    }

    /// Refreshes the local keywords list from the manager and adjusts selection.
    fn refresh_keywords(&mut self, manager: &mut KeywordsManager) -> Result<()> {
        self.keywords = manager.load_keywords()?;
        if self.keywords.is_empty() {
            self.list_state.select(None);
        } else {
            // Keep a valid selection (default to first item if none).
            let idx = self
                .list_state
                .selected()
                .unwrap_or(0)
                .min(self.keywords.len().saturating_sub(1));
            self.list_state.select(Some(idx));
        }
        Ok(())
    }

    /// Deletes the currently selected keyword and keeps selection in a valid state.
    fn delete_selected_keyword(&mut self, manager: &mut KeywordsManager) -> Result<()> {
        if self.keywords.is_empty() {
            return Ok(());
        }

        if let Some(idx) = self.list_state.selected() {
            manager.remove_keyword(idx)?;
            self.keywords = manager.load_keywords()?;

            if self.keywords.is_empty() {
                self.list_state.select(None);
            } else if idx >= self.keywords.len() && idx > 0 {
                self.list_state.select(Some(idx - 1));
            } else {
                self.list_state
                    .select(Some(idx.min(self.keywords.len() - 1)));
            }
        }

        Ok(())
    }

    /// Renders the current state of the keywords viewer.
    fn draw(&mut self) -> Result<()> {
        // Extract data before the closure to avoid borrow conflicts
        let input_mode = self.input_mode;
        let input_value = self.input.value().to_string();
        let input_cursor = self.input.cursor();
        let keywords = self.keywords.clone();
        let list_state = &mut self.list_state;

        self.terminal.draw(|frame| {
            let area = frame.area();

            let padding_block = Block::default()
                .padding(Padding::uniform(1))
                .style(Style::default().bg(BG));
            frame.render_widget(&padding_block, area);
            let padded_area = padding_block.inner(area);

            let main_block = Block::default().style(Style::default().fg(FG).bg(BG));
            frame.render_widget(&main_block, padded_area);
            let inner_area = main_block.inner(padded_area);

            // Split into header and content
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(inner_area);

            let header_area = layout[0];
            let content_area = layout[1];

            // Header
            let header_text = " ┏┓┏╋╋ \n ┗┛┛┗┗ \n";
            let header_paragraph = Paragraph::new(header_text)
                .style(Style::default().fg(FG))
                .alignment(Alignment::Left);
            frame.render_widget(header_paragraph, header_area);

            if input_mode {
                Self::draw_with_input(
                    frame,
                    content_area,
                    &keywords,
                    &input_value,
                    input_cursor,
                    list_state,
                );
            } else {
                Self::draw_normal(frame, content_area, &keywords, list_state);
            }
        })?;

        Ok(())
    }

    /// Draws the UI when *not* in input mode.
    fn draw_normal(frame: &mut Frame, area: Rect, keywords: &[String], list_state: &mut ListState) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let list_area = layout[0];
        let help_area = layout[1];

        Self::render_keywords_list(frame, list_area, keywords, list_state);

        let help_text = "↑↓ select, x/del remove, a add, q quit";
        let help_paragraph = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(HELP_FG));
        frame.render_widget(help_paragraph, help_area);
    }

    /// Draws the UI when in input mode.
    fn draw_with_input(
        frame: &mut Frame,
        area: Rect,
        keywords: &[String],
        input_value: &str,
        input_cursor: usize,
        list_state: &mut ListState,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let list_area = layout[0];
        let input_area = layout[1];

        Self::render_keywords_list(frame, list_area, keywords, list_state);

        let input_block = Block::default().title("New Keyword").borders(Borders::ALL);
        frame.render_widget(&input_block, input_area);
        let input_inner = input_block.inner(input_area);

        let input_widget = Paragraph::new(input_value).style(Style::default().fg(Color::Rgb(255, 255, 255)));
        frame.render_widget(input_widget, input_inner);

        // Cursor position based on tui_input cursor
        let cursor_x = input_area.x + input_cursor as u16 + 1;
        let cursor_y = input_area.y + 1;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }

    /// Renders the keywords list with selection.
    fn render_keywords_list(
        frame: &mut Frame,
        area: Rect,
        keywords: &[String],
        list_state: &mut ListState,
    ) {
        let items: Vec<ListItem> = keywords
            .iter()
            .map(|keyword| ListItem::new(keyword.clone()))
            .collect();

        let list = List::new(items)
            .block(Block::default().title(" Keywords ").borders(Borders::ALL))
            .highlight_style(Style::default().bg(HIGHLIGHT_BG).fg(FG));

        frame.render_stateful_widget(list, area, list_state);
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

impl Drop for KeywordsViewer {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
