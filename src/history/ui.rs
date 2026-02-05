//! Interactive terminal UI for viewing transcription history.
//!
//! Provides a scrollable list of transcriptions with keyboard navigation,
//! mouse support, selection, and clipboard integration.

use crate::history::TranscriptionEntry;
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph},
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(255, 255, 255);
const TIMESTAMP_FG: Color = Color::Rgb(100, 100, 100);
const HIGHLIGHT_BG: Color = Color::Rgb(20, 20, 20);
const HELP_FG: Color = Color::Rgb(100, 100, 100);

/// Interactive history viewer for transcription entries.
pub struct HistoryViewer {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    entries: Vec<TranscriptionEntry>,
    list_state: ListState,
    notification: Option<(String, Instant)>,
    pending_click: Option<(usize, Instant)>,
}

impl HistoryViewer {
    /// Creates a new history viewer with the given entries.
    pub fn new(entries: Vec<TranscriptionEntry>) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            terminal,
            entries,
            list_state,
            notification: None,
            pending_click: None,
        })
    }

    /// Runs the interactive history viewer loop.
    pub fn run(&mut self) -> Result<Option<String>> {
        if self.entries.is_empty() {
            self.cleanup()?;
            return Ok(None);
        }

        tracing::debug!("History viewer started with {} entries", self.entries.len());

        let mut selected_text: Option<String> = None;

        loop {
            self.draw()?;

            // Check if notification has expired
            if let Some((_, start_time)) = self.notification {
                if start_time.elapsed() >= Duration::from_millis(500) {
                    self.notification = None;
                    if selected_text.is_some() {
                        break; // Exit after showing notification
                    }
                }
            }

            // Check if pending click should be processed
            if let Some((entry_index, click_time)) = self.pending_click {
                if click_time.elapsed() >= Duration::from_millis(200) {
                    selected_text = Some(self.entries[entry_index].text.clone());
                    self.pending_click = None;
                    self.notification = Some(("Copied to clipboard!".to_string(), Instant::now()));
                    tracing::info!("Clicked item copied to clipboard");
                }
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if let Some(action) = self.handle_key(key) {
                            match action {
                                InputAction::Exit => break,
                                InputAction::Select(text) => {
                                    selected_text = Some(text);
                                    self.notification =
                                        Some(("Copied to clipboard!".to_string(), Instant::now()));
                                }
                            }
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    _ => {}
                }
            }
        }

        self.cleanup()?;
        Ok(selected_text)
    }

    /// Handles keyboard input.
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<InputAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                tracing::debug!("History viewer exited via Escape/q");
                Some(InputAction::Exit)
            }
            KeyCode::Up => {
                self.list_state.select_previous();
                None
            }
            KeyCode::Down => {
                self.list_state.select_next();
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    tracing::debug!("Entry selected via Enter");
                    Some(InputAction::Select(self.entries[idx].text.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handles mouse events.
    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.list_state.select_previous();
            }
            MouseEventKind::ScrollDown => {
                self.list_state.select_next();
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                if let Some(selected) = self.list_state.selected() {
                    self.pending_click = Some((selected, Instant::now()));
                    tracing::debug!("Item clicked, showing selection feedback");
                }
            }
            _ => {}
        }
    }

    /// Renders the current state of the history viewer.
    fn draw(&mut self) -> Result<()> {
        let notification = self.notification.clone();

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

            // Split into header, list, and footer areas
            let [header_area, list_area, footer_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner_area);

            // Render ostt logo header
            let header = Paragraph::new(" ┏┓┏╋╋ \n ┗┛┛┗┗ \n")
                .style(Style::default().fg(FG))
                .alignment(Alignment::Left);
            frame.render_widget(header, header_area);

            // Build list items with styled timestamp and text
            let items: Vec<ListItem> = self
                .entries
                .iter()
                .map(|entry| {
                    let timestamp = Line::styled(
                        entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                        Style::default().fg(TIMESTAMP_FG),
                    );
                    let text = Line::styled(entry.text.clone(), Style::default().fg(FG));
                    ListItem::new(vec![timestamp, text])
                })
                .collect();

            // Render list with History title
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" History ")
                        .borders(Borders::ALL)
                        .padding(Padding::bottom(1)),
                )
                .highlight_style(Style::default().bg(HIGHLIGHT_BG))
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always);

            frame.render_stateful_widget(list, list_area, &mut self.list_state);

            // Render help footer
            let help_text = "↑↓ select, ↵ copy, esc/q exit";
            let help_paragraph = Paragraph::new(help_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(HELP_FG));
            frame.render_widget(help_paragraph, footer_area);

            // Render notification modal if active
            if let Some((message, _)) = notification {
                Self::render_notification(frame, area, &message);
            }
        })?;

        Ok(())
    }

    /// Renders a centered notification modal.
    fn render_notification(frame: &mut Frame, screen_area: Rect, message: &str) {
        let modal_width = (message.len() as u16).saturating_add(4);
        let modal_height = 3;

        let modal_x = screen_area.x + (screen_area.width.saturating_sub(modal_width)) / 2;
        let modal_y = screen_area.y + (screen_area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width.min(screen_area.width),
            height: modal_height,
        };

        let modal_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Green).fg(Color::Black));

        frame.render_widget(&modal_block, modal_area);

        let inner_area = modal_block.inner(modal_area);
        let notification_text = Paragraph::new(message)
            .style(Style::default().bg(Color::Green).fg(Color::Black))
            .alignment(Alignment::Center);

        frame.render_widget(notification_text, inner_area);
    }

    /// Cleans up terminal and restores normal mode.
    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        tracing::debug!("History viewer terminal cleanup complete");
        Ok(())
    }
}

/// Actions that can result from user input.
enum InputAction {
    Exit,
    Select(String),
}

impl Drop for HistoryViewer {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
