//! Bash action executor for bash processing actions.
//!
//! Executes a shell command with text piped to stdin and captures stdout as the result.

/// Executes a bash command with the given input piped to stdin.
///
/// Returns the command's stdout as a trimmed string.
///
/// # Arguments
/// * `command` - The shell command to execute (passed to `sh -c`)
/// * `input` - Text to pipe into the command's stdin
///
/// # Errors
/// - If the command cannot be spawned
/// - If stdin cannot be written
/// - If the command exits with non-zero status
/// - If stdout cannot be read
pub async fn execute_bash_action(command: &str, input: &str) -> anyhow::Result<String> {
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;
    use tokio::time::timeout;

    const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

    tracing::debug!("Executing bash command: {}", command);

    // Spawn `sh -c <command>` with stdin piped and stdout/stderr captured
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            tracing::error!("Bash command failed to start: {e}");
            anyhow::anyhow!(
                "Command failed to start: {e}. Make sure the command is installed."
            )
        })?;

    // Write input to stdin, then close it to signal EOF
    let mut stdin = child.stdin.take().expect("stdin was piped");
    stdin.write_all(input.as_bytes()).await?;
    drop(stdin);

    // Wait for the child to complete with timeout
    let output = timeout(COMMAND_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| {
            tracing::error!("Bash command timed out after 30 seconds: {}", command);
            anyhow::anyhow!("Command timed out after 30 seconds")
        })?
        .map_err(|e| anyhow::anyhow!("Failed to wait for command: {e}"))?;

    // Check exit status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);
        if code == 127 {
            tracing::error!("Bash command not found: {}", command);
            anyhow::bail!(
                "Command not found. Make sure the command is installed.\nShell output: {}",
                stderr.trim()
            );
        }
        tracing::error!("Bash command exited with status {}: {}", code, stderr.trim());
        anyhow::bail!("Command exited with status {code}:\n{}", stderr.trim());
    }

    // Return trimmed stdout on success
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    tracing::debug!("Bash command completed successfully ({} bytes)", stdout.len());
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════
    // 3.1 — Bash action executor tests
    // ═══════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn tr_uppercase_transforms_input() {
        let result = execute_bash_action("tr '[:lower:]' '[:upper:]'", "hello").await;
        assert_eq!(result.unwrap(), "HELLO");
    }

    #[tokio::test]
    async fn cat_passes_input_through() {
        let result = execute_bash_action("cat", "pass through").await;
        assert_eq!(result.unwrap(), "pass through");
    }

    #[tokio::test]
    async fn non_zero_exit_returns_error_with_stderr() {
        let result = execute_bash_action("echo err >&2; exit 1", "").await;
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Command exited with status"),
            "error should mention exit status, got: {err}"
        );
        assert!(
            err.contains("err"),
            "error should contain stderr content, got: {err}"
        );
    }

    #[tokio::test]
    async fn nonexistent_command_returns_clear_error() {
        let result = execute_bash_action("nonexistent_command_xyz", "").await;
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Command not found"),
            "error should mention 'Command not found', got: {err}"
        );
    }
}
