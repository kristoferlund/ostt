//! Lists the agents AI processing actions can run, and their models.

use crate::config;
use crate::config::agents::{all_agents, AgentLookup, AgentSpec};
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The agent's program as it would run: an executable file at its path, or
/// the first one of that name on `PATH`.
fn program_path(agent: &AgentSpec) -> Option<PathBuf> {
    let program = Path::new(agent.program());
    if program.components().count() > 1 {
        return is_executable(program).then(|| program.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|path| is_executable(path))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

pub fn handle_agent_list(config_data: &config::OsttConfig, json: bool) -> anyhow::Result<()> {
    let invalid = &config_data.process.agents.invalid;
    // An invalid user agent hides the built-in of the same id.
    let agents: Vec<_> = all_agents(&config_data.process.agents.valid)
        .into_iter()
        .filter(|(id, _, _)| !invalid.contains_key(*id))
        .collect();

    if json {
        let agents: Vec<_> =
            agents
                .iter()
                .map(|(id, agent, builtin)| {
                    let path = program_path(agent);
                    serde_json::json!({
                        "id": id,
                        "name": agent.name,
                        "installed": path.is_some(),
                        "path": path,
                        "images": agent.takes_images(),
                        "builtin": builtin,
                    })
                })
                .chain(invalid.iter().map(
                    |(id, reason)| serde_json::json!({ "id": id, "name": id, "error": reason }),
                ))
                .collect();
        println!("{}", serde_json::to_string_pretty(&agents)?);
        return Ok(());
    }

    for (id, agent, _) in &agents {
        let mut notes = Vec::new();
        if program_path(agent).is_none() {
            notes.push("not installed");
        }
        if !agent.takes_images() {
            notes.push("no images");
        }
        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", notes.join(", "))
        };
        println!("{id} — {}{notes}", agent.name);
    }
    for (id, reason) in invalid {
        println!("{id} — not valid: {reason}");
    }
    Ok(())
}

/// How long a command listing an agent's models may take.
const MODELS_TIMEOUT: Duration = Duration::from_secs(15);

/// Prints the models an agent offers, one per line: its fixed list, or what
/// its models command prints. Nothing for an agent with neither.
pub async fn handle_agent_models(config_data: &config::OsttConfig, id: &str) -> anyhow::Result<()> {
    let agent = match config_data.process.agents.lookup(id) {
        AgentLookup::Found(agent) => agent,
        AgentLookup::Invalid(reason) => {
            anyhow::bail!("Agent '{id}' in [process.agents] is not valid: {reason}")
        }
        AgentLookup::Unknown => {
            anyhow::bail!("Unknown agent '{id}'. Use 'ostt agent list' to see available agents.")
        }
    };
    if !agent.models.is_empty() {
        println!("{}", agent.models.join("\n"));
        return Ok(());
    }
    let Some((program, args)) = agent
        .models_command
        .as_deref()
        .and_then(<[String]>::split_first)
    else {
        return Ok(());
    };
    let output = tokio::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(MODELS_TIMEOUT, output)
        .await
        .map_err(|_| {
            anyhow::anyhow!("'{program}' did not list its models within {MODELS_TIMEOUT:?}")
        })?
        .with_context(|| format!("failed to run '{program}'"))?;
    if !output.status.success() {
        anyhow::bail!(
            "'{program}' could not list its models: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}
