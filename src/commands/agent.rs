//! Lists the agents AI processing actions can run.

use crate::config;
use crate::config::agents::{all_agents, AgentSpec};
use std::path::Path;

/// Whether the agent's program can be run: an executable file at its path, or
/// one of that name on `PATH`.
fn is_installed(agent: &AgentSpec) -> bool {
    let program = agent.program();
    if program.contains('/') {
        return is_executable(Path::new(program));
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| is_executable(&dir.join(program)))
    })
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
        let agents: Vec<_> = agents
            .iter()
            .map(|(id, agent, builtin)| {
                serde_json::json!({
                    "id": id,
                    "name": agent.name,
                    "installed": is_installed(agent),
                    "images": agent.takes_images(),
                    "builtin": builtin,
                })
            })
            .chain(
                invalid
                    .iter()
                    .map(|(id, reason)| serde_json::json!({ "id": id, "error": reason })),
            )
            .collect();
        println!("{}", serde_json::to_string_pretty(&agents)?);
        return Ok(());
    }

    for (id, agent, _) in &agents {
        let mut notes = Vec::new();
        if !is_installed(agent) {
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
