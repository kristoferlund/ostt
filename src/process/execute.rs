//! Action dispatcher for processing actions.
//!
//! Provides a unified `execute_action` function that dispatches to the correct
//! executor based on the action type (bash or AI).

use crate::config::{ActionDetails, ProcessAction};

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
    match &action.details {
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
