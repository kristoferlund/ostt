//! Lists the agents AI processing actions can run.

use crate::config;
use crate::config::agents::{all_agents, AgentSpec};
use std::path::Path;

/// Whether the agent's program can be run: an existing file for a path, or a
/// file of that name on `PATH`.
fn is_installed(agent: &AgentSpec) -> bool {
    let program = agent.program();
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

pub fn handle_agent_list(config_data: &config::OsttConfig, json: bool) -> anyhow::Result<()> {
    let agents = all_agents(&config_data.process.agents);

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
            .collect();
        println!("{}", serde_json::to_string_pretty(&agents)?);
        return Ok(());
    }

    for (id, agent, _) in agents {
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
    Ok(())
}
