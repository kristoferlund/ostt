//! Action dispatcher for processing actions.
//!
//! Provides a unified `execute_action` function that dispatches to the correct
//! executor based on the action type (bash or AI). Also provides
//! `execute_action_with_animation` which wraps the action in an animated
//! progress indicator.

use crate::config::agents::UserAgents;
use crate::config::{ActionDetails, ProcessAction, ProcessConfig};
use crate::transcription::TranscriptionAnimation;
use crate::ui::session;
use ratcn::terminal::{Session, SessionOptions};
use std::path::Path;

/// Executes a processing action on the given transcription text.
///
/// For bash actions: pipes transcription to the command's stdin.
/// For AI actions: resolves inputs, invokes the configured CLI tool, returns response.
///
/// # Arguments
/// * `agents` - The user's agents, for an AI action's agent
/// * `action` - The action configuration to execute
/// * `transcription` - The raw transcription text
/// * `keywords` - The user's keyword list (for AI actions that reference keywords)
/// * `attachment` - Optional image; AI actions send it to the tool, bash actions get its path in `OSTT_ATTACHMENT`
///
/// # Errors
/// - If the action type is bash and the command fails
/// - If the action type is AI and the CLI tool invocation fails
/// - If input resolution fails (missing file, etc.)
pub async fn execute_action(
    agents: &UserAgents,
    action: &ProcessAction,
    transcription: &str,
    keywords: &[String],
    attachment: Option<&Path>,
) -> anyhow::Result<String> {
    let action_type = match &action.details {
        ActionDetails::Bash { .. } => "bash",
        ActionDetails::Ai { .. } => "ai",
    };
    tracing::info!("Dispatching action '{}' (type: {})", action.id, action_type);

    let result = match &action.details {
        ActionDetails::Bash { command } => {
            super::bash::execute_bash_action(command, transcription, attachment).await
        }
        ActionDetails::Ai {
            tool,
            model,
            inputs,
            tool_binary,
            tool_args,
            timeout_secs,
        } => {
            let messages = super::input::resolve_inputs(inputs, transcription, keywords)?;
            let tool = tool
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("AI action '{}' has no tool", action.id))?;
            let model = model
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("AI action '{}' has no model", action.id))?;
            super::ai::execute_ai_action(
                agents.resolve(tool)?,
                model,
                &messages,
                super::ai::RunOptions {
                    tool_binary: tool_binary.as_deref(),
                    tool_args: tool_args.as_deref(),
                    timeout: timeout_secs
                        .map_or(super::ai::DEFAULT_TIMEOUT, std::time::Duration::from_secs),
                    attachment,
                },
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

pub fn select_requested_action<F>(
    process_config: &ProcessConfig,
    process_arg: Option<&str>,
    pick_action_id: F,
) -> anyhow::Result<Option<ProcessAction>>
where
    F: FnOnce(&[ProcessAction]) -> anyhow::Result<Option<String>>,
{
    match process_arg {
        None => Ok(None),
        Some("") => pick_action_id(&process_config.actions)?.map_or(Ok(None), |action_id| {
            find_action(process_config, &action_id).map(Some)
        }),
        Some(action_id) => find_action(process_config, action_id).map(Some),
    }
}

/// A processing action failed, and the unprocessed transcription was output
/// in its place so the words are not lost. ostt exits with status 3 for it,
/// which tells callers that output was still written.
#[derive(Debug, thiserror::Error)]
#[error("Processing failed, so the transcription was output unprocessed")]
pub struct ProcessingFailed(#[source] pub anyhow::Error);

/// Exit status for [`ProcessingFailed`].
pub const PROCESSING_FAILED_EXIT: u8 = 3;

/// Writes the processed text with `write`, or the unprocessed text if
/// processing failed, in which case it returns [`ProcessingFailed`].
pub fn output_processed(
    processed: anyhow::Result<String>,
    unprocessed: &str,
    write: impl FnOnce(&str) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    write(processed.as_deref().unwrap_or(unprocessed))?;
    processed.map(drop).map_err(|e| ProcessingFailed(e).into())
}

pub async fn apply_requested_action(
    process_config: &ProcessConfig,
    transcription: &str,
    keywords: &[String],
    process_arg: Option<&str>,
    attachment: Option<&Path>,
) -> anyhow::Result<String> {
    if process_arg.is_some() && transcription.trim().is_empty() {
        // As in the record flow: nothing to process, so the text is output as is.
        tracing::info!("Transcription is empty; skipping processing");
        return Ok(transcription.to_string());
    }

    let Some(action) = select_requested_action(process_config, process_arg, |actions| {
        match super::process_view::show_action_picker(actions)? {
            super::process_view::PickerResult::Selected(action_id) => Ok(Some(action_id)),
            super::process_view::PickerResult::Cancelled => Ok(None),
        }
    })?
    else {
        return Ok(transcription.to_string());
    };

    if process_arg == Some("") {
        return match execute_action_with_animation(
            &process_config.agents,
            &action,
            transcription,
            keywords,
            attachment,
        )
        .await?
        {
            Some(result) => Ok(result),
            None => Ok(transcription.to_string()),
        };
    }

    execute_action(
        &process_config.agents,
        &action,
        transcription,
        keywords,
        attachment,
    )
    .await
}

/// Fails for a named action that does not exist, so a mistyped `--process`
/// is reported before recording or transcribing rather than after.
pub fn check_requested_action(
    process_config: &ProcessConfig,
    process_arg: Option<&str>,
) -> anyhow::Result<()> {
    match process_arg {
        Some(action_id) if !action_id.is_empty() => {
            find_action(process_config, action_id).map(drop)
        }
        _ => Ok(()),
    }
}

fn find_action(process_config: &ProcessConfig, action_id: &str) -> anyhow::Result<ProcessAction> {
    process_config
        .get_action(action_id)
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown action '{action_id}'. Use 'ostt process list' to see available actions."
            )
        })
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
    agents: &UserAgents,
    action: &ProcessAction,
    transcription: &str,
    keywords: &[String],
    attachment: Option<&Path>,
) -> anyhow::Result<Option<String>> {
    // No mouse: motion reports would wake the loop and advance the animation.
    let mut session = Session::open(SessionOptions::new().adaptive())?;

    let mut animation = TranscriptionAnimation::new(80);
    animation.set_status_label("Processing...");

    // Clone data for the spawned task
    let agents_clone = agents.clone();
    let action_clone = action.clone();
    let transcription_clone = transcription.to_string();
    let keywords_clone = keywords.to_vec();
    let attachment_clone = attachment.map(Path::to_path_buf);

    let task_handle = tokio::spawn(async move {
        execute_action(
            &agents_clone,
            &action_clone,
            &transcription_clone,
            &keywords_clone,
            attachment_clone.as_deref(),
        )
        .await
    });

    let mut cancelled = false;
    loop {
        // Render animation frame
        let theme = session.theme();
        session.terminal_mut().draw(|frame| {
            let area = frame.area();
            animation.update();
            animation.draw_themed(frame, area, &theme);
        })?;

        // Check if task finished
        if task_handle.is_finished() {
            break;
        }

        if session::next_event(&mut session, Some(std::time::Duration::from_millis(50)))?
            .as_ref()
            .is_some_and(session::is_cancel)
        {
            tracing::info!("Processing cancelled by user");
            task_handle.abort();
            cancelled = true;
            break;
        }
    }

    // Restore terminal before returning
    drop(session);

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

    /// An action whose agent the user defined but broke must say so, not
    /// claim the agent is unknown or run a built-in one instead.
    #[tokio::test]
    async fn action_with_invalid_agent_reports_why() {
        let config: ProcessConfig = toml::from_str(
            r#"
            [agents.claude-code]
            name = "My Claude"
            comand = ["/opt/claude"]

            [actions.a]
            name = "A"
            type = "ai"
            tool = "claude-code"
            model = "m"
            inputs = [{ role = "user", source = "transcription" }]
            "#,
        )
        .unwrap();
        let err = execute_action(&config.agents, &config.actions[0], "words", &[], None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("not valid"), "{err}");
        assert!(err.contains("command"), "{err}");
    }

    /// An action's own timeout bounds a hung agent, so dictation does not
    /// wait the default five minutes.
    #[tokio::test]
    async fn action_timeout_stops_a_hung_agent() {
        let config: ProcessConfig = toml::from_str(
            r#"
            [agents.slow]
            name = "Slow"
            command = ["sh", "-c", "sleep 10"]

            [actions.a]
            name = "A"
            type = "ai"
            tool = "slow"
            model = "m"
            timeout_secs = 1
            inputs = [{ role = "user", source = "transcription" }]
            "#,
        )
        .unwrap();
        let started = std::time::Instant::now();
        let err = execute_action(&config.agents, &config.actions[0], "words", &[], None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("timed out after 1s"), "{err}");
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    /// A failed action must not lose the words: the unprocessed text is
    /// written, and the error is one main turns into exit status 3.
    #[test]
    fn failed_processing_outputs_the_unprocessed_text() {
        let mut written = String::new();
        let result = output_processed(Err(anyhow::anyhow!("agent down")), "raw words", |text| {
            written = text.to_string();
            Ok(())
        });
        assert_eq!(written, "raw words");
        let err = result.unwrap_err();
        assert!(err.is::<ProcessingFailed>());
        assert!(format!("{err:?}").contains("agent down"), "{err:?}");

        let mut written = String::new();
        output_processed(Ok("processed".into()), "raw words", |text| {
            written = text.to_string();
            Ok(())
        })
        .unwrap();
        assert_eq!(written, "processed");
    }

    /// A mistyped action is reported before any recording or transcription;
    /// an empty one means "show the picker" and is not checked.
    #[test]
    fn requested_action_is_checked_up_front() {
        let config: ProcessConfig = toml::from_str(
            "[actions.clean]\nname = \"Clean\"\ntype = \"bash\"\ncommand = \"cat\"\n",
        )
        .unwrap();
        assert!(check_requested_action(&config, Some("clean")).is_ok());
        assert!(check_requested_action(&config, Some("")).is_ok());
        assert!(check_requested_action(&config, None).is_ok());
        let err = check_requested_action(&config, Some("claen"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unknown action 'claen'"), "{err}");
    }

    /// Helper: create a bash ProcessAction.
    fn bash_action(command: &str) -> ProcessAction {
        ProcessAction {
            id: "test".to_string(),
            name: "Test Action".to_string(),
            description: None,
            details: ActionDetails::Bash {
                command: command.to_string(),
            },
        }
    }

    #[tokio::test]
    async fn bash_cat_returns_transcription_as_is() {
        let action = bash_action("cat");
        let result =
            execute_action(&UserAgents::default(), &action, "hello world", &[], None).await;
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn bash_tr_transforms_text() {
        let action = bash_action("tr '[:lower:]' '[:upper:]'");
        let result =
            execute_action(&UserAgents::default(), &action, "hello world", &[], None).await;
        assert_eq!(result.unwrap(), "HELLO WORLD");
    }

    #[tokio::test]
    async fn bash_failure_returns_error() {
        let action = bash_action("echo err >&2; exit 1");
        let result = execute_action(&UserAgents::default(), &action, "", &[], None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Command exited with status"),
            "error should mention exit status, got: {err}"
        );
    }

    /// Silence must not be sent to an action (and, for AI actions, a model).
    #[tokio::test]
    async fn empty_transcription_skips_processing() {
        let config = ProcessConfig {
            actions: vec![bash_action("echo should-not-run")],
            ..Default::default()
        };
        let result = apply_requested_action(&config, "  \n", &[], Some("test"), None).await;
        assert_eq!(result.unwrap(), "  \n");
    }
}
