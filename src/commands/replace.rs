use crate::config::OsttConfig;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::components::modal::form_dialog;
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratatui::layout::Rect;
use ratcn::{
    runtime::{CellOffset, DeclareCtx, Event, FocusState, KeyCode, ModalState, Ratcn},
    Button, Dialog, Input, InputState,
};
use std::fs;

const FORM: &str = "replace";

pub async fn handle_replace() -> Result<()> {
    let config = OsttConfig::load().map_err(|err| anyhow::anyhow!(err.to_string()))?;
    run(config)
}

struct State {
    focus: FocusState,
    modals: ModalState,
    form_offset: CellOffset,
    config: OsttConfig,
    rules: Vec<String>,
    selected: Option<usize>,
    source_input: InputState,
    target_input: InputState,
}

impl State {
    fn new(config: OsttConfig) -> Self {
        let mut state = Self {
            focus: FocusState::default(),
            modals: ModalState::default(),
            form_offset: CellOffset::default(),
            config,
            rules: Vec::new(),
            selected: None,
            source_input: InputState::default(),
            target_input: InputState::default(),
        };
        state.refresh_rules();
        state
    }

    fn refresh_rules(&mut self) {
        self.rules = self
            .config
            .text
            .replace
            .iter()
            .map(|(source, target)| format!("{source} → {target}"))
            .collect();
        clamp_selection(&mut self.selected, self.rules.len());
    }

    fn close_form(&mut self) {
        self.modals.close(&mut self.focus);
        self.source_input = InputState::default();
        self.target_input = InputState::default();
    }
}

enum Msg {
    Focus(FocusState),
    Select(usize),
    Source(InputState),
    Target(InputState),
    FormMoved(CellOffset),
    Open,
    Next,
    Add,
    Delete,
    Dismiss,
}

fn runtime() -> Ratcn<State, Msg> {
    Ratcn::new()
        .focus(|state: &State| &state.focus, Msg::Focus)
        .modals(|state| &state.modals)
}

/// Every return restores the terminal.
fn run(config: OsttConfig) -> Result<()> {
    let mut state = State::new(config);
    let mut session = session::open()?;
    let mut ratcn = runtime();
    loop {
        session::draw(
            &mut session,
            &mut ratcn,
            &state,
            chrome(&state),
            |ctx, body| declare(ctx, body, &state),
        )?;
        let Some(event) = session::next_event(&mut session, None)? else {
            continue;
        };
        let msg = match session::route(&mut session, &mut ratcn, &state, event)? {
            Routed::Msg(msg) => msg,
            Routed::Ignored(event) if session::is_cancel(&event) => break,
            Routed::Ignored(Event::Key(key)) => match key.code {
                KeyCode::Char('a') => Msg::Open,
                KeyCode::Char('x') | KeyCode::Delete => Msg::Delete,
                _ => continue,
            },
            Routed::Ignored(_) | Routed::Redraw => continue,
            Routed::Quit => break,
        };
        if update(&mut state, msg)? {
            save_replace_rules(&state.config.text.replace)?;
        }
    }
    Ok(())
}

/// Returns whether the rules changed and need saving. Unlike keywords, which
/// take their manager, saving here writes the real config path, so `run` does it.
fn update(state: &mut State, msg: Msg) -> Result<bool> {
    match msg {
        Msg::Focus(focus) => state.focus = focus,
        Msg::Select(index) => state.selected = Some(index),
        Msg::Source(input) => state.source_input = input,
        Msg::Target(input) => state.target_input = input,
        Msg::FormMoved(offset) => state.form_offset = offset,
        Msg::Open => {
            state.form_offset = CellOffset::default();
            state.modals.open(FORM, &mut state.focus)?;
        }
        Msg::Next => state.focus = FocusState::intent([FORM, "input-1"]),
        Msg::Add => {
            let source = state.source_input.value().trim();
            if source.is_empty() {
                return Ok(false);
            }
            let target = state.target_input.value().trim().to_string();
            state.config.text.replace.insert(source.to_string(), target);
            state.refresh_rules();
            state.close_form();
            return Ok(true);
        }
        Msg::Delete => {
            if let Some(index) = state.selected {
                state.config.text.replace.shift_remove_index(index);
                state.refresh_rules();
                return Ok(true);
            }
        }
        Msg::Dismiss => state.close_form(),
    }
    Ok(false)
}

fn chrome(state: &State) -> Chrome<'static> {
    Chrome {
        title: Some("Replace"),
        footer: if state.modals.is_open(FORM) {
            "↵ next/add, tab switch, esc cancel"
        } else {
            "↑↓ select, x/del delete, a add, esc/q exit"
        },
        toasts: None,
    }
}

fn declare(ctx: &mut DeclareCtx<'_, State, Msg>, body: Rect, state: &State) {
    ctx.component(
        "list",
        selection_list(
            &state.rules,
            1,
            |s: &State| s.selected,
            Msg::Select,
            Msg::Select,
        ),
        body,
    );
    if state.modals.is_open(FORM) {
        let form = Dialog::new()
            .title("New replace")
            .offset(state.form_offset)
            .on_offset_change(Msg::FormMoved)
            .on_dismiss(|| Msg::Dismiss)
            .action("add", Button::new("Add").on_press(|| Msg::Add));
        let inputs = vec![
            Input::new()
                .title("Find")
                .value(|s: &State| &s.source_input, Msg::Source)
                .on_submit(|| Msg::Next),
            Input::new()
                .title("Replace")
                .value(|s: &State| &s.target_input, Msg::Target)
                .on_submit(|| Msg::Add),
        ];
        ctx.modal(FORM, form_dialog(form, "", inputs), ctx.area());
    }
}

fn save_replace_rules(replace_rules: &indexmap::IndexMap<String, String>) -> Result<()> {
    let config_path = crate::app_dirs::config_path()?;
    let content = fs::read_to_string(&config_path)?;
    let updated = replace_text_section(&content, replace_rules);
    fs::write(config_path, updated)?;
    Ok(())
}

fn replace_text_section(
    content: &str,
    replace_rules: &indexmap::IndexMap<String, String>,
) -> String {
    let without_text = remove_top_level_section(content, "text");
    if replace_rules.is_empty() {
        return ensure_trailing_newline(&without_text);
    }

    let section = render_text_replace_section(replace_rules);
    let mut lines = without_text.lines().collect::<Vec<_>>();
    let insert_at = lines
        .iter()
        .position(|line| line.trim() == "[process]")
        .or_else(|| lines.iter().position(|line| line.trim() == "[popup]"))
        .unwrap_or(lines.len());
    let section_lines = section.lines().collect::<Vec<_>>();
    lines.splice(insert_at..insert_at, section_lines);
    ensure_trailing_newline(&trim_extra_blank_lines(&lines.join("\n")))
}

fn render_text_replace_section(replace_rules: &indexmap::IndexMap<String, String>) -> String {
    let mut section = String::from("[text.replace]\n");
    for (source, target) in replace_rules {
        section.push_str(&format!(
            "{} = {}\n",
            toml_basic_string(source),
            toml_basic_string(target)
        ));
    }
    section.push('\n');
    section
}

fn remove_top_level_section(content: &str, section: &str) -> String {
    let header = format!("[{section}]");
    let subsection_prefix = format!("[{section}.");
    let mut output = Vec::new();
    let mut skipping = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == header || trimmed.starts_with(&subsection_prefix) {
            skipping = true;
            continue;
        }
        if skipping && trimmed.starts_with('[') && trimmed.ends_with(']') {
            let still_section = trimmed == header || trimmed.starts_with(&subsection_prefix);
            if !still_section {
                skipping = false;
            }
        }
        if !skipping {
            output.push(line);
        }
    }

    trim_extra_blank_lines(&output.join("\n"))
}

fn trim_extra_blank_lines(content: &str) -> String {
    let mut output = Vec::new();
    let mut previous_blank = false;
    for line in content.lines() {
        let blank = line.trim().is_empty();
        if blank && previous_blank {
            continue;
        }
        output.push(line);
        previous_blank = blank;
    }
    output.join("\n").trim().to_string()
}

fn ensure_trailing_newline(content: &str) -> String {
    format!("{}\n", content.trim_end())
}

fn toml_basic_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use ratcn::runtime::EventResult;

    fn replace_rules(entries: &[(&str, &str)]) -> IndexMap<String, String> {
        entries
            .iter()
            .map(|(source, target)| (source.to_string(), target.to_string()))
            .collect()
    }

    #[test]
    fn replacing_text_section_preserves_transcription_section() {
        let content = r#"[audio]
device = "default"

[transcription]
provider = "deepgram"
model = "nova-3"

[text.replace]
"api" = "API"

[popup]
width = 90
"#;

        let updated = replace_text_section(content, &replace_rules(&[("ostt", "OSTT")]));

        assert!(updated.contains("provider = \"deepgram\""));
        assert!(updated.contains("model = \"nova-3\""));
        assert!(updated.contains("[text.replace]\n\"ostt\" = \"OSTT\""));
        assert!(updated.contains("\"ostt\" = \"OSTT\"\n\n[popup]"));
        assert!(!updated.contains("\"api\" = \"API\""));
    }

    #[test]
    fn empty_replace_rules_remove_text_section_only() {
        let content = r#"[audio]
device = "default"

[transcription]
provider = "deepgram"
model = "nova-3"

[text]

[text.replace]
"api" = "API"

[popup]
width = 90
"#;

        let updated = replace_text_section(content, &IndexMap::new());

        assert!(updated.contains("[transcription]"));
        assert!(!updated.contains("[text]"));
        assert!(!updated.contains("[text.replace]"));
        assert!(updated.contains("[popup]"));
    }

    /// Enter on Find moves to Replace instead of saving a rule with an empty
    /// replacement; Enter there saves.
    #[test]
    fn enter_advances_from_find_to_replace_before_adding() {
        let mut state = State::new(OsttConfig::default());
        update(&mut state, Msg::Open).unwrap();
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let mut paint = |state: &State, ratcn: &mut Ratcn<State, Msg>| {
            terminal
                .draw(|frame| {
                    session::render(
                        frame,
                        ratcn,
                        state,
                        &ratcn::Theme::default_dark(),
                        chrome(state),
                        |ctx, body| declare(ctx, body, state),
                    )
                })
                .unwrap();
        };
        paint(&state, &mut ratcn);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Enter, &state),
            EventResult::Emit(Msg::Next)
        ));
        assert!(!update(&mut state, Msg::Next).unwrap());
        paint(&state, &mut ratcn);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Enter, &state),
            EventResult::Emit(Msg::Add)
        ));
    }

    /// A rule with a Find text is added, reported for saving, and the form closes.
    #[test]
    fn add_with_find_inserts_the_rule_and_closes_the_form() {
        let mut state = State::new(OsttConfig::default());
        update(&mut state, Msg::Open).unwrap();
        update(&mut state, Msg::Source(InputState::new("api"))).unwrap();
        update(&mut state, Msg::Target(InputState::new("API"))).unwrap();

        assert!(update(&mut state, Msg::Add).unwrap());
        assert_eq!(state.config.text.replace, replace_rules(&[("api", "API")]));
        assert_eq!(state.rules, ["api → API"]);
        assert!(!state.modals.is_open(FORM));
    }

    /// Delete must remove the selected rule and report it for saving, and the
    /// selection must stay on an existing rule.
    #[test]
    fn delete_removes_the_selected_rule_and_keeps_selection_in_range() {
        let mut config = OsttConfig::default();
        config.text.replace = replace_rules(&[("api", "API"), ("ostt", "OSTT")]);
        let mut state = State::new(config);
        state.selected = Some(1);

        assert!(update(&mut state, Msg::Delete).unwrap());
        assert_eq!(state.config.text.replace, replace_rules(&[("api", "API")]));
        assert_eq!(state.rules, ["api → API"]);
        assert_eq!(state.selected, Some(0));
    }

    /// Cancelling the form closes it and forgets what was typed.
    #[test]
    fn dismiss_closes_the_form_and_clears_both_inputs() {
        let mut state = State::new(OsttConfig::default());
        update(&mut state, Msg::Open).unwrap();
        update(&mut state, Msg::Source(InputState::new("api"))).unwrap();
        update(&mut state, Msg::Target(InputState::new("API"))).unwrap();

        assert!(!update(&mut state, Msg::Dismiss).unwrap());
        assert!(!state.modals.is_open(FORM));
        assert_eq!(state.source_input.value(), "");
        assert_eq!(state.target_input.value(), "");
    }

    /// An empty Find adds nothing, so nothing is written to the config, and
    /// the form stays open with the typed text so the user can finish it.
    #[test]
    fn add_without_find_changes_nothing_and_keeps_the_form() {
        let mut state = State::new(OsttConfig::default());
        update(&mut state, Msg::Open).unwrap();
        update(&mut state, Msg::Target(InputState::new("API"))).unwrap();

        assert!(!update(&mut state, Msg::Add).unwrap());
        assert!(state.config.text.replace.is_empty());
        assert!(state.modals.is_open(FORM));
        assert_eq!(state.target_input.value(), "API");
    }
}
