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
    /// Models to offer when setting up an action, for agents with a short
    /// fixed list.
    #[serde(default)]
    pub models: Vec<String>,
    /// Command that prints the agent's models, one per line.
    #[serde(default)]
    pub models_command: Option<Vec<String>>,
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
        // stream-json carries the prompt and the image on stdin.
        if self.format == AgentFormat::ClaudeStreamJson
            && self.command.iter().any(|arg| arg.contains("{prompt}"))
        {
            return Err(
                "format claude-stream-json takes the prompt on stdin, not {prompt}".to_string(),
            );
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

/// Why an agent id cannot be run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentError {
    #[error("Unknown agent '{0}'. Use 'ostt agent list' to see available agents.")]
    Unknown(String),
    #[error("Agent '{id}' in [process.agents] is not valid: {reason}")]
    Invalid { id: String, reason: String },
}

/// The user's own agents from `[process.agents]`, in config order. One that
/// is not valid keeps its place with the reason, and hides a built-in agent
/// of the same id rather than fall back to it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserAgents(IndexMap<String, Result<AgentSpec, String>>);

/// One agent in [`UserAgents::all`].
#[derive(Debug)]
pub struct ListedAgent<'a> {
    pub id: &'a str,
    /// The agent, or why the user's agent of this id is not valid.
    pub spec: Result<&'a AgentSpec, &'a str>,
    pub builtin: bool,
}

impl UserAgents {
    /// The agent with this id: the user's own, else the built-in one.
    pub fn resolve(&self, id: &str) -> Result<&AgentSpec, AgentError> {
        match self.0.get(id) {
            Some(Ok(spec)) => Ok(spec),
            Some(Err(reason)) => Err(AgentError::Invalid {
                id: id.to_string(),
                reason: reason.clone(),
            }),
            None => builtin_agents()
                .get(id)
                .ok_or_else(|| AgentError::Unknown(id.to_string())),
        }
    }

    /// Every agent as [`Self::resolve`] sees it: the built-in ones in
    /// `agents.toml` order, each replaced by the user's agent of that id, then
    /// the user's other agents.
    pub fn all(&self) -> Vec<ListedAgent<'_>> {
        let builtin = builtin_agents()
            .iter()
            .map(|(id, spec)| match self.0.get(id) {
                Some(own) => (id, own.as_ref(), false),
                None => (id, Ok(spec), true),
            });
        let own = self
            .0
            .iter()
            .filter(|(id, _)| !builtin_agents().contains_key(*id))
            .map(|(id, own)| (id, own.as_ref(), false));
        builtin
            .chain(own)
            .map(|(id, spec, builtin)| ListedAgent {
                id,
                spec: spec.map_err(String::as_str),
                builtin,
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The built-in agents, in `agents.toml` order.
pub fn builtin_agents() -> &'static IndexMap<String, AgentSpec> {
    static AGENTS: OnceLock<IndexMap<String, AgentSpec>> = OnceLock::new();
    AGENTS.get_or_init(|| {
        toml::from_str(include_str!("agents.toml")).expect("built-in agents.toml is valid")
    })
}

/// Parses the user's `[process.agents]`. An agent that is not valid is kept
/// with its reason so the rest of the config loads; an action using it fails
/// with that reason.
pub(crate) fn parse_user_agents(raw: IndexMap<String, toml::Value>) -> UserAgents {
    UserAgents(
        raw.into_iter()
            .map(|(id, value)| {
                let spec = value
                    .try_into::<AgentSpec>()
                    .map_err(|e| e.to_string())
                    .and_then(|spec| spec.validate().map(|()| spec))
                    .map_err(|reason| {
                        let reason = reason.trim().to_string();
                        tracing::warn!("Process agent '{id}' is not valid: {reason}");
                        reason
                    });
                (id, spec)
            })
            .collect(),
    )
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

        assert_eq!(user.resolve("opencode").unwrap().name, "My OpenCode");
        assert_eq!(user.resolve("pi").unwrap().name, "Pi");
        assert_eq!(user.resolve("local").unwrap().program(), "llm");
        assert_eq!(
            user.resolve("missing"),
            Err(AgentError::Unknown("missing".to_string()))
        );

        let all = user.all();
        let opencode = all.iter().find(|a| a.id == "opencode").unwrap();
        assert_eq!(opencode.spec.unwrap().name, "My OpenCode");
        assert!(!opencode.builtin);
        assert_eq!(all.last().unwrap().id, "local");
        assert_eq!(all.iter().filter(|a| a.id == "opencode").count(), 1);
    }

    /// `ostt agent list` shows the user's agents in the order they are written.
    #[test]
    fn user_agents_keep_config_order() {
        let raw: IndexMap<String, toml::Value> = toml::from_str(
            r#"
            [zeta]
            name = "Zeta"
            command = ["z", "{model}"]

            [alpha]
            name = "Alpha"
            command = []

            [mid]
            name = "Mid"
            command = ["m", "{model}"]
            "#,
        )
        .unwrap();
        let user = parse_user_agents(raw);
        let own: Vec<_> = user
            .all()
            .into_iter()
            .filter(|a| !a.builtin)
            .map(|a| a.id)
            .collect();
        assert_eq!(own, ["zeta", "alpha", "mid"]);
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

            # The image would be dropped: stream-json sends it on stdin.
            [stream]
            name = "Stream"
            command = ["claude", "{prompt}", "{model}"]
            format = "claude-stream-json"
            "#,
        )
        .unwrap();
        let user = parse_user_agents(raw);
        for id in ["broken", "claude-code", "escape", "stream"] {
            assert!(
                matches!(user.resolve(id), Err(AgentError::Invalid { .. })),
                "{id} must be reported invalid"
            );
        }
        let Err(AgentError::Invalid { reason, .. }) = user.resolve("escape") else {
            unreachable!()
        };
        assert!(reason.contains("relative path"), "{reason}");
        // Listed in place of the built-in, with the reason.
        let listed = user.all();
        let claude = listed.iter().find(|a| a.id == "claude-code").unwrap();
        assert!(claude.spec.is_err() && !claude.builtin);
    }
}
