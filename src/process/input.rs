//! Input resolution for AI processing actions.
//!
//! Resolves `ActionInput` entries into concrete `ResolvedMessage` values
//! by substituting dynamic sources, reading files, or passing literal content.

use crate::config::{ActionInput, InputContent, InputRole, InputSource};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// A resolved message ready to send to an LLM.
#[derive(Debug)]
pub struct ResolvedMessage {
    pub role: InputRole,
    pub content: String,
}

/// Resolves action inputs into concrete messages.
///
/// # Arguments
/// * `inputs` - The action's input configuration
/// * `transcription` - The transcribed text to substitute for `InputSource::Transcription`
/// * `keywords` - The user's keyword list to substitute for `InputSource::Keywords`
///
/// # Errors
/// - If a `file` path cannot be read
pub fn resolve_inputs(
    inputs: &[ActionInput],
    transcription: &str,
    keywords: &[String],
) -> Result<Vec<ResolvedMessage>> {
    let mut messages = Vec::new();

    for input in inputs {
        match &input.input_content {
            InputContent::Literal { content } => {
                messages.push(ResolvedMessage {
                    role: input.role.clone(),
                    content: content.clone(),
                });
            }
            InputContent::Source { source } => match source {
                InputSource::Transcription => {
                    messages.push(ResolvedMessage {
                        role: input.role.clone(),
                        content: transcription.to_string(),
                    });
                }
                InputSource::Keywords => {
                    if !keywords.is_empty() {
                        messages.push(ResolvedMessage {
                            role: input.role.clone(),
                            content: keywords.join("\n"),
                        });
                    }
                }
            },
            InputContent::File { file } => {
                let path = expand_tilde(file);
                tracing::debug!("Reading input file: {}", path.display());
                let content = fs::read_to_string(&path).with_context(|| {
                    tracing::error!("Failed to read input file: {}", path.display());
                    format!("failed to read input file: {}", path.display())
                })?;
                messages.push(ResolvedMessage {
                    role: input.role.clone(),
                    content,
                });
            }
        }
    }

    tracing::debug!(
        "Resolved {} input(s): {}",
        messages.len(),
        messages
            .iter()
            .map(|m| format!("{:?}", m.role))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(messages)
}

/// Expands a leading `~` in a path to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════
    // 1.2.6 — Basic resolution tests
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn literal_content_resolves_correctly() {
        let inputs = vec![ActionInput {
            role: InputRole::System,
            input_content: InputContent::Literal {
                content: "You are a helpful assistant.".to_string(),
            },
        }];

        let messages = resolve_inputs(&inputs, "", &[]).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0].role, InputRole::System));
        assert_eq!(messages[0].content, "You are a helpful assistant.");
    }

    #[test]
    fn transcription_source_resolves() {
        let inputs = vec![ActionInput {
            role: InputRole::User,
            input_content: InputContent::Source {
                source: InputSource::Transcription,
            },
        }];

        let messages = resolve_inputs(&inputs, "Hello world", &[]).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0].role, InputRole::User));
        assert_eq!(messages[0].content, "Hello world");
    }

    #[test]
    fn keywords_source_resolves_to_newline_joined() {
        let inputs = vec![ActionInput {
            role: InputRole::User,
            input_content: InputContent::Source {
                source: InputSource::Keywords,
            },
        }];

        let keywords = vec![
            "rust".to_string(),
            "audio".to_string(),
            "transcription".to_string(),
        ];
        let messages = resolve_inputs(&inputs, "", &keywords).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "rust\naudio\ntranscription");
    }

    #[test]
    fn empty_keywords_list_is_skipped() {
        let inputs = vec![ActionInput {
            role: InputRole::User,
            input_content: InputContent::Source {
                source: InputSource::Keywords,
            },
        }];

        let messages = resolve_inputs(&inputs, "", &[]).unwrap();
        assert_eq!(messages.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════
    // 1.2.7 — File resolution tests
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn valid_file_reads_correctly() {
        let dir = std::env::temp_dir().join("ostt_test_input_resolve");
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("prompt.txt");
        fs::write(&file_path, "You are a cleaning assistant.").unwrap();

        let inputs = vec![ActionInput {
            role: InputRole::System,
            input_content: InputContent::File {
                file: file_path.to_string_lossy().to_string(),
            },
        }];

        let messages = resolve_inputs(&inputs, "", &[]).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "You are a cleaning assistant.");

        // Cleanup
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tilde_path_expansion_works() {
        // Write a temp file under ~/
        let home = dirs::home_dir().expect("home dir must exist for test");
        let test_dir = home.join(".ostt_test_input_resolve");
        fs::create_dir_all(&test_dir).unwrap();
        let file_path = test_dir.join("prompt.txt");
        fs::write(&file_path, "tilde expanded content").unwrap();

        let inputs = vec![ActionInput {
            role: InputRole::System,
            input_content: InputContent::File {
                file: "~/.ostt_test_input_resolve/prompt.txt".to_string(),
            },
        }];

        let messages = resolve_inputs(&inputs, "", &[]).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "tilde expanded content");

        // Cleanup
        fs::remove_dir_all(&test_dir).ok();
    }

    #[test]
    fn missing_file_returns_error() {
        let inputs = vec![ActionInput {
            role: InputRole::System,
            input_content: InputContent::File {
                file: "/nonexistent/path/to/file.txt".to_string(),
            },
        }];

        let result = resolve_inputs(&inputs, "", &[]);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("failed to read input file"));
    }
}
