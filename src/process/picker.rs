//! Interactive terminal UI for selecting a processing action.
//!
//! Provides a fullscreen ratatui-based picker that lists configured actions
//! and lets the user select one via keyboard navigation.

use crate::config::file::ProcessAction;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph},
};
use std::io::{self, Stdout};

const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(255, 255, 255);
const HIGHLIGHT_BG: Color = Color::Rgb(20, 20, 20);
const HELP_FG: Color = Color::Rgb(100, 100, 100);

/// Result of the action picker interaction.
pub enum PickerResult {
    /// User selected an action — contains the action's ID.
    Selected(String),
    /// User cancelled (Esc/q).
    Cancelled,
}

/// Interactive action picker for selecting a processing action.
struct ActionPicker {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    actions: Vec<ProcessAction>,
    list_state: ListState,
    cleaned_up: bool,
}

impl ActionPicker {
    /// Creates a new action picker with the given actions.
    ///
    /// Sets up the terminal in raw mode with an alternate screen.
    /// The initial selection is set to the first item.
    fn new(actions: Vec<ProcessAction>) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Ok(Self {
            terminal,
            actions,
            list_state,
            cleaned_up: false,
        })
    }

    /// Renders the current state of the action picker.
    fn draw(&mut self) -> Result<()> {
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

            // Build list items from action names
            let items: Vec<ListItem> = self
                .actions
                .iter()
                .map(|action| ListItem::new(action.name.clone()))
                .collect();

            // Render list with title
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" Process action ")
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().bg(HIGHLIGHT_BG))
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always);

            frame.render_stateful_widget(list, list_area, &mut self.list_state);

            // Render help footer
            let help_text = "↑/↓ select, ↵ confirm, esc/q cancel";
            let help_paragraph = Paragraph::new(help_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(HELP_FG));
            frame.render_widget(help_paragraph, footer_area);
        })?;

        Ok(())
    }

    /// Cleans up terminal and restores normal mode.
    fn cleanup(&mut self) -> Result<()> {
        if self.cleaned_up {
            return Ok(());
        }

        self.cleaned_up = true;

        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Runs the interactive action picker loop.
    ///
    /// Draws the UI, reads keyboard events, and dispatches actions.
    /// Returns `PickerResult::Selected(id)` on Enter or `PickerResult::Cancelled` on Esc/q.
    fn run(&mut self) -> Result<PickerResult> {
        let result = loop {
            self.draw()?;

            if let Event::Key(key) = event::read()? {
                if let Some(action) = self.handle_key(key) {
                    match action {
                        PickerAction::Exit => break PickerResult::Cancelled,
                        PickerAction::Select(id) => break PickerResult::Selected(id),
                    }
                }
            }
        };

        self.cleanup()?;
        Ok(result)
    }

    /// Handles keyboard input and returns an optional action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<PickerAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(PickerAction::Exit),
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state.select_previous();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select_next();
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    Some(PickerAction::Select(self.actions[idx].id.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Actions that can result from keyboard input in the picker.
enum PickerAction {
    /// User wants to exit/cancel.
    Exit,
    /// User selected an action — contains the action's ID.
    Select(String),
}

impl Drop for ActionPicker {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Shows the action picker TUI and waits for user selection.
///
/// # Arguments
/// * `actions` - List of available actions to display
///
/// # Returns
/// The ID of the selected action, or `Cancelled` if the user presses Esc/q.
///
/// # Edge cases
/// - Returns an error if `actions` is empty
/// - Returns `Selected(id)` directly if only one action is configured (skips the picker)
///
/// # Errors
/// - If no actions are configured
/// - If terminal initialization fails
/// - If rendering fails
pub fn show_action_picker(actions: &[ProcessAction]) -> Result<PickerResult> {
    if actions.is_empty() {
        anyhow::bail!("No processing actions configured. Add actions to ~/.config/ostt/ostt.toml");
    }

    if actions.len() == 1 {
        return Ok(PickerResult::Selected(actions[0].id.clone()));
    }

    let mut picker = ActionPicker::new(actions.to_vec())?;
    picker.run()
}
