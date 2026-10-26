//! Lists the agents AI processing actions can run, and their models.

use crate::config;
use crate::config::agents::AgentSpec;
use crate::process::find_program;
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The agent's program as it would run.
fn program_path(agent: &AgentSpec) -> Option<PathBuf> {
    find_program(Path::new(agent.program()))
}

pub fn handle_agent_list(config_data: &config::OsttConfig, json: bool) -> anyhow::Result<()> {
    let agents = config_data.process.agents.all();

    if json {
        let agents: Vec<_> = agents
            .iter()
            .map(|listed| match listed.spec {
                Ok(agent) => {
                    let path = program_path(agent);
                    serde_json::json!({
                        "id": listed.id,
                        "name": agent.name,
                        "installed": path.is_some(),
                        "path": path,
                        "images": agent.takes_images(),
                        "prompt_in_args": agent.prompt_in_args(),
                        "builtin": listed.builtin,
                    })
                }
                Err(reason) => {
                    serde_json::json!({ "id": listed.id, "name": listed.id, "error": reason })
                }
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&agents)?);
        return Ok(());
    }

    for listed in &agents {
        let agent = match listed.spec {
            Ok(agent) => agent,
            Err(reason) => {
                println!("{} — not valid: {reason}", listed.id);
                continue;
            }
        };
        let mut notes = Vec::new();
        if program_path(agent).is_none() {
            notes.push("not installed");
        }
        if !agent.takes_images() {
            notes.push("no images");
        }
        if agent.prompt_in_args() {
            notes.push("prompt visible to other users");
        }
        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", notes.join(", "))
        };
        println!("{} — {}{notes}", listed.id, agent.name);
    }
    Ok(())
}

/// How long a command listing an agent's models may take.
const MODELS_TIMEOUT: Duration = Duration::from_secs(15);

/// Prints the models an agent offers, one per line: its fixed list, or what
/// its models command prints. Nothing for an agent with neither.
pub async fn handle_agent_models(config_data: &config::OsttConfig, id: &str) -> anyhow::Result<()> {
    let agent = config_data.process.agents.resolve(id)?;
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
