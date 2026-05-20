//! Interactive terminal UI for selecting a processing action.
//!
//! Provides a fullscreen ratatui-based picker that lists configured actions
//! and lets the user select one via keyboard navigation.

use crate::config::file::ProcessAction;
use crate::ui::{render_app_layout, render_footer, render_title};
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState},
};
use std::io::{self, Stdout};

/// Renders the action picker UI into the given frame area.
///
/// Shared rendering logic used by both the standalone `ProcessView`
/// and `OsttTui::render_action_picker()`.
pub fn render_process_view(
    frame: &mut Frame,
    area: Rect,
    actions: &[ProcessAction],
    list_state: &mut ListState,
    hovered_index: Option<usize>,
) -> Rect {
    let layout = render_app_layout(frame, area);
    render_title(frame, layout.title, "Process action");
    let list_area = layout.body;

    // Build list items from action names
    let selected_index = list_state.selected();
    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let mut item = ListItem::new(action.name.clone());
            if Some(i) == hovered_index && Some(i) != selected_index {
                item = item.style(Style::default().fg(Color::White).bg(Color::DarkGray));
            }
            item
        })
        .collect();

    // Render list with title
    let list =
        List::new(items).highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_stateful_widget(list, list_area, list_state);

    render_footer(frame, layout.footer, "↑/↓ select, ↵ confirm, esc/q cancel");

    list_area
}

/// Result of the action picker interaction.
pub enum PickerResult {
    /// User selected an action — contains the action's ID.
    Selected(String),
    /// User cancelled (Esc/q).
    Cancelled,
}

/// Interactive process-action view for selecting a processing action.
struct ProcessView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    actions: Vec<ProcessAction>,
    list_state: ListState,
    cleaned_up: bool,
    hovered_index: Option<usize>,
    list_area: Rect,
}

impl ProcessView {
    /// Creates a new process-action view with the given actions.
    ///
    /// Sets up the terminal in raw mode with an alternate screen.
    /// The initial selection is set to the first item.
    fn new(actions: Vec<ProcessAction>) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Ok(Self {
            terminal,
            actions,
            list_state,
            cleaned_up: false,
            hovered_index: None,
            list_area: Rect::default(),
        })
    }

    /// Renders the current state of the action picker.
    fn draw(&mut self) -> Result<()> {
        let actions = &self.actions;
        let list_state = &mut self.list_state;
        let hovered_index = self.hovered_index;
        let mut computed_list_area = Rect::default();
        self.terminal.draw(|frame| {
            let area = frame.area();
            computed_list_area =
                render_process_view(frame, area, actions, list_state, hovered_index);
        })?;

        self.list_area = computed_list_area;

        Ok(())
    }

    /// Cleans up terminal and restores normal mode.
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

    /// Runs the interactive action picker loop.
    ///
    /// Draws the UI, reads keyboard events, and dispatches actions.
    /// Returns `PickerResult::Selected(id)` on Enter or `PickerResult::Cancelled` on Esc/q.
    fn run(&mut self) -> Result<PickerResult> {
        let result = loop {
            self.draw()?;

            match event::read()? {
                Event::Key(key) => {
                    if let Some(action) = self.handle_key(key) {
                        match action {
                            PickerAction::Exit => {
                                tracing::info!("Action picker cancelled by user");
                                break PickerResult::Cancelled;
                            }
                            PickerAction::Select(id) => {
                                tracing::info!("User selected action '{}'", id);
                                break PickerResult::Selected(id);
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        self.list_state.select_previous();
                    }
                    MouseEventKind::ScrollDown => {
                        self.list_state.select_next();
                    }
                    MouseEventKind::Moved => {
                        let inner_top = self.list_area.y;
                        let inner_bottom = self.list_area.y + self.list_area.height;
                        if mouse.row < inner_top || mouse.row >= inner_bottom {
                            self.hovered_index = None;
                        } else {
                            let relative_y = mouse.row - inner_top;
                            let visible_index = relative_y as usize; // picker items are 1 line tall
                            let actual_index = visible_index + self.list_state.offset();
                            if actual_index < self.actions.len() {
                                self.hovered_index = Some(actual_index);
                            } else {
                                self.hovered_index = None;
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        };

        self.cleanup()?;
        Ok(result)
    }

    /// Handles keyboard input and returns an optional action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<PickerAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(PickerAction::Exit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(PickerAction::Exit)
            }
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

impl Drop for ProcessView {
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
        tracing::debug!(
            "Single action configured, auto-selecting '{}'",
            actions[0].id
        );
        return Ok(PickerResult::Selected(actions[0].id.clone()));
    }

    let mut view = ProcessView::new(actions.to_vec())?;
    view.run()
}
