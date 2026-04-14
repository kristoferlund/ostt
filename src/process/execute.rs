//! Action dispatcher for processing actions.
//!
//! Provides a unified `execute_action` function that dispatches to the correct
//! executor based on the action type (bash or AI). Also provides
//! `execute_action_with_animation` which wraps the action in an animated
//! progress indicator.

use crate::config::{ActionDetails, ProcessAction};
use crate::transcription::TranscriptionAnimation;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, Stdout};

/// Executes a processing action on the given transcription text.
///
/// For bash actions: pipes transcription to the command's stdin.
/// For AI actions: resolves inputs, invokes the configured CLI tool, returns response.
///
/// # Arguments
/// * `action` - The action configuration to execute
/// * `transcription` - The raw transcription text
/// * `keywords` - The user's keyword list (for AI actions that reference keywords)
///
/// # Errors
/// - If the action type is bash and the command fails
/// - If the action type is AI and the CLI tool invocation fails
/// - If input resolution fails (missing file, etc.)
pub async fn execute_action(
    action: &ProcessAction,
    transcription: &str,
    keywords: &[String],
) -> anyhow::Result<String> {
    let action_type = match &action.details {
        ActionDetails::Bash { .. } => "bash",
        ActionDetails::Ai { .. } => "ai",
    };
    tracing::info!(
        "Dispatching action '{}' (type: {})",
        action.id,
        action_type
    );

    let result = match &action.details {
        ActionDetails::Bash { command } => {
            super::bash::execute_bash_action(command, transcription).await
        }
        ActionDetails::Ai {
            tool,
            model,
            inputs,
            tool_binary,
            tool_args,
        } => {
            let messages = super::input::resolve_inputs(inputs, transcription, keywords)?;
            super::ai::execute_ai_action(
                tool,
                model,
                &messages,
                tool_binary.as_deref(),
                tool_args.as_deref(),
            )
            .await
        }
    };

    match &result {
        Ok(output) => {
            tracing::info!(
                "Action '{}' completed successfully ({} bytes)",
                action.id,
                output.len()
            );
        }
        Err(e) => {
            tracing::error!("Action '{}' failed: {}", action.id, e);
        }
    }

    result
}

/// Drop-based cleanup guard that ensures the terminal is restored even on
/// panic or early return.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    cleaned_up: bool,
}

impl TerminalGuard {
    /// Creates a new terminal guard, entering raw mode and alternate screen.
    fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            cleaned_up: false,
        })
    }

    /// Restores the terminal to normal mode.
    fn cleanup(&mut self) -> anyhow::Result<()> {
        if self.cleaned_up {
            return Ok(());
        }
        self.cleaned_up = true;
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Executes an action with an animated progress indicator.
///
/// Shows the OSTT logo animation with a "Processing..." label while the action runs.
/// Cancellable via Esc/q/Ctrl+C.
///
/// # Returns
/// - `Ok(Some(result))` — action completed successfully
/// - `Ok(None)` — user cancelled during processing
/// - `Err(...)` — action failed
pub async fn execute_action_with_animation(
    action: &ProcessAction,
    transcription: &str,
    keywords: &[String],
) -> anyhow::Result<Option<String>> {
    let mut guard = TerminalGuard::new()?;

    let mut animation = TranscriptionAnimation::new(80);
    animation.set_status_label("Processing...");

    // Clone data for the spawned task
    let action_clone = action.clone();
    let transcription_clone = transcription.to_string();
    let keywords_clone = keywords.to_vec();

    let task_handle = tokio::spawn(async move {
        execute_action(&action_clone, &transcription_clone, &keywords_clone).await
    });

    let mut cancelled = false;
    loop {
        // Render animation frame
        guard.terminal.draw(|frame| {
            let area = frame.area();
            animation.update();
            animation.draw(frame, area);
        })?;

        // Check if task finished
        if task_handle.is_finished() {
            break;
        }

        // Poll for cancel input (Esc/q/Ctrl+C)
        if event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        tracing::info!("Processing cancelled by user");
                        task_handle.abort();
                        cancelled = true;
                        break;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        tracing::info!("Processing cancelled by user (Ctrl+C)");
                        task_handle.abort();
                        cancelled = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Restore terminal before returning
    guard.cleanup()?;

    if cancelled {
        return Ok(None);
    }

    match task_handle.await {
        Ok(Ok(result)) => Ok(Some(result)),
        Ok(Err(e)) => {
            tracing::error!("Processing action failed: {}", e);
            Err(e)
        }
        Err(e) => {
            tracing::error!("Processing task panicked: {}", e);
            Err(anyhow::anyhow!("Processing task failed: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════
    // 3.2 — Action dispatcher tests
    // ═══════════════════════════════════════════════════════════════════

    /// Helper: create a bash ProcessAction.
    fn bash_action(command: &str) -> ProcessAction {
        ProcessAction {
            id: "test".to_string(),
            name: "Test Action".to_string(),
            details: ActionDetails::Bash {
                command: command.to_string(),
            },
        }
    }

    #[tokio::test]
    async fn bash_cat_returns_transcription_as_is() {
        let action = bash_action("cat");
        let result = execute_action(&action, "hello world", &[]).await;
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn bash_tr_transforms_text() {
        let action = bash_action("tr '[:lower:]' '[:upper:]'");
        let result = execute_action(&action, "hello world", &[]).await;
        assert_eq!(result.unwrap(), "HELLO WORLD");
    }

    #[tokio::test]
    async fn bash_failure_returns_error() {
        let action = bash_action("echo err >&2; exit 1");
        let result = execute_action(&action, "", &[]).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Command exited with status"),
            "error should mention exit status, got: {err}"
        );
    }
}
