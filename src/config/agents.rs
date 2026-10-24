//! Agents that AI processing actions run.
//!
//! How to call each agent is data: the built-in agents come from
//! `agents.toml`, and `[process.agents.<id>]` in the user's config adds agents
//! or replaces built-in ones by id.

use indexmap::IndexMap;
use serde::Deserialize;
use std::sync::OnceLock;

/// How ostt calls one agent non-interactively.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentSpec {
    /// Display name.
    pub name: String,
    /// Program and arguments, with `{model}`, `{workdir}` and `{prompt}`
    /// placeholders. Without `{prompt}`, the prompt goes to stdin.
    pub command: Vec<String>,
    /// How the prompt and the answer are encoded.
    #[serde(default)]
    pub format: AgentFormat,
    /// Arguments added when an image is attached, with an `{image}` placeholder.
    #[serde(default)]
    pub image_args: Option<Vec<String>>,
    /// Text added to the end of the prompt to attach an image, with `{image}`.
    #[serde(default)]
    pub image_prompt: Option<String>,
    /// Environment variables for the agent, with `{workdir}`.
    #[serde(default)]
    pub env: IndexMap<String, String>,
    /// Files written into the agent's working directory before it runs, by
    /// relative path.
    #[serde(default)]
    pub files: IndexMap<String, String>,
    /// Lowest version, as printed by `<program> --version`, the arguments work with.
    #[serde(default)]
    pub minimum_version: Option<String>,
}

/// How the prompt goes in and the answer comes out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentFormat {
    /// The prompt as plain text on stdin; the answer is stdout.
    #[default]
    Text,
    /// Claude Code's stream-json: a user message on stdin that can carry an
    /// image, and a result event on stdout.
    ClaudeStreamJson,
}

impl AgentSpec {
    /// The program the agent runs.
    pub fn program(&self) -> &str {
        self.command.first().map(String::as_str).unwrap_or_default()
    }

    /// Whether the agent can receive an attached image.
    pub fn takes_images(&self) -> bool {
        self.format == AgentFormat::ClaudeStreamJson
            || self.image_args.is_some()
            || self.image_prompt.is_some()
    }

    fn validate(&self) -> Result<(), String> {
        if self.program().is_empty() {
            return Err("it needs a command".to_string());
        }
        self.files.keys().try_for_each(|name| check_file_name(name))
    }
}

/// Checks that an agent file's name keeps it inside the working directory.
pub(crate) fn check_file_name(name: &str) -> Result<(), String> {
    let inside = std::path::Path::new(name)
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    if name.is_empty() || !inside {
        return Err(format!(
            "file '{name}' must be a relative path inside its working directory"
        ));
    }
    Ok(())
}

/// What an agent id refers to.
#[derive(Debug, Clone, Copy)]
pub enum AgentLookup<'a> {
    /// No agent has that id.
    Unknown,
    Found(&'a AgentSpec),
    /// The user defines that agent, but not validly; why.
    Invalid(&'a str),
}

/// The user's own agents from `[process.agents]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserAgents {
    pub valid: IndexMap<String, AgentSpec>,
    /// Agents that could not be used, with the reason. They hide a built-in
    /// agent of the same id rather than fall back to it.
    pub invalid: IndexMap<String, String>,
}

impl UserAgents {
    /// Looks up an agent: the user's own first, then the built-in ones.
    pub fn lookup(&self, id: &str) -> AgentLookup<'_> {
        if let Some(reason) = self.invalid.get(id) {
            return AgentLookup::Invalid(reason);
        }
        match self.valid.get(id).or_else(|| builtin_agents().get(id)) {
            Some(spec) => AgentLookup::Found(spec),
            None => AgentLookup::Unknown,
        }
    }
}

/// The built-in agents, in `agents.toml` order.
pub fn builtin_agents() -> &'static IndexMap<String, AgentSpec> {
    static AGENTS: OnceLock<IndexMap<String, AgentSpec>> = OnceLock::new();
    AGENTS.get_or_init(|| {
        toml::from_str(include_str!("agents.toml")).expect("built-in agents.toml is valid")
    })
}

/// All agents, built-in ones first, each replaced by a user agent of the same
/// id, then the user's other agents. The flag says whether it is built in.
pub fn all_agents(user_agents: &IndexMap<String, AgentSpec>) -> Vec<(&str, &AgentSpec, bool)> {
    let mut agents: Vec<(&str, &AgentSpec, bool)> = builtin_agents()
        .iter()
        .map(|(id, spec)| match user_agents.get(id) {
            Some(own) => (id.as_str(), own, false),
            None => (id.as_str(), spec, true),
        })
        .collect();
    agents.extend(
        user_agents
            .iter()
            .filter(|(id, _)| !builtin_agents().contains_key(*id))
            .map(|(id, spec)| (id.as_str(), spec, false)),
    );
    agents
}

/// Parses the user's `[process.agents]`. An agent that is not valid is kept
/// aside with its reason so the rest of the config loads; an action using it
/// fails with that reason.
pub(crate) fn parse_user_agents(raw: IndexMap<String, toml::Value>) -> UserAgents {
    let mut agents = UserAgents::default();
    for (id, value) in raw {
        let spec = value
            .try_into::<AgentSpec>()
            .map_err(|e| e.to_string())
            .and_then(|spec| spec.validate().map(|()| spec));
        match spec {
            Ok(spec) => {
                agents.valid.insert(id, spec);
            }
            Err(reason) => {
                tracing::warn!("Process agent '{id}' is not valid: {reason}");
                agents.invalid.insert(id, reason);
            }
        }
    }
    agents
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped file must parse, and every agent must have a program and
    /// the {model} placeholder: an action's model would otherwise be ignored.
    #[test]
    fn builtin_agents_are_valid_and_take_the_model() {
        let agents = builtin_agents();
        for id in ["claude-code", "codex-cli", "opencode", "gemini-cli"] {
            assert!(agents.contains_key(id), "existing tool id '{id}' must stay");
        }
        for (id, spec) in agents {
            spec.validate().unwrap();
            assert!(
                spec.command.iter().any(|a| a.contains("{model}")),
                "agent '{id}' ignores the model"
            );
        }
    }

    #[test]
    fn user_agent_replaces_builtin_and_adds_new() {
        let raw: IndexMap<String, toml::Value> = toml::from_str(
            r#"
            [opencode]
            name = "My OpenCode"
            command = ["/opt/opencode", "run", "--model", "{model}"]

            [local]
            name = "Local"
            command = ["llm", "-m", "{model}"]
            "#,
        )
        .unwrap();
        let user = parse_user_agents(raw);
        let found = |id| match user.lookup(id) {
            AgentLookup::Found(spec) => spec,
            other => panic!("{id}: {other:?}"),
        };

        assert_eq!(found("opencode").name, "My OpenCode");
        assert_eq!(found("pi").name, "Pi");
        assert_eq!(found("local").program(), "llm");
        assert!(matches!(user.lookup("missing"), AgentLookup::Unknown));

        let all = all_agents(&user.valid);
        let opencode = all.iter().find(|(id, _, _)| *id == "opencode").unwrap();
        assert_eq!(
            (opencode.1.name.as_str(), opencode.2),
            ("My OpenCode", false)
        );
        assert_eq!(all.last().unwrap().0, "local");
        assert_eq!(all.iter().filter(|(id, _, _)| *id == "opencode").count(), 1);
    }

    /// A broken agent must not stop the rest of the config from loading, and
    /// must not silently fall back to a built-in agent of the same id: that
    /// would run a different program than the user configured.
    #[test]
    fn invalid_user_agent_is_reported_not_replaced() {
        let raw: IndexMap<String, toml::Value> = toml::from_str(
            r#"
            [broken]
            name = "Broken"
            command = []

            [claude-code]
            name = "My Claude"
            comand = ["/opt/claude"]

            [escape]
            name = "Escape"
            command = ["x"]
            files = { "../out" = "x" }
            "#,
        )
        .unwrap();
        let user = parse_user_agents(raw);
        assert!(user.valid.is_empty());
        for id in ["broken", "claude-code", "escape"] {
            assert!(
                matches!(user.lookup(id), AgentLookup::Invalid(_)),
                "{id} must be reported invalid"
            );
        }
        let AgentLookup::Invalid(reason) = user.lookup("escape") else {
            unreachable!()
        };
        assert!(reason.contains("relative path"), "{reason}");
    }
}
