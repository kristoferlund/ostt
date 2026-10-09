use crate::config::OsttConfig;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::components::modal::form_dialog;
use crate::ui::session::{self, Chrome, Routed};
use anyhow::Result;
use ratatui::layout::Rect;
use ratcn::{
    runtime::{CellOffset, DeclareCtx, Event, FocusState, KeyCode, ModalState, Ratcn},
    terminal::Session,
    Button, Dialog, Input, InputState,
};
use std::fs;

const FORM: &str = "replace";

pub async fn handle_replace() -> Result<()> {
    let config = OsttConfig::load().map_err(|err| anyhow::anyhow!(err.to_string()))?;
    ReplaceView::new(config)?.run()
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

struct ReplaceView {
    session: Session,
    state: State,
    ratcn: Ratcn<State, Msg>,
}

impl ReplaceView {
    fn new(config: OsttConfig) -> Result<Self> {
        Ok(Self {
            session: session::open()?,
            state: State::new(config),
            ratcn: runtime(),
        })
    }

    fn run(mut self) -> Result<()> {
        loop {
            let state = &self.state;
            session::draw(
                &mut self.session,
                &mut self.ratcn,
                state,
                chrome(state),
                |ctx, body| declare(ctx, body, state),
            )?;
            let Some(event) = session::next_event(&mut self.session, None)? else {
                continue;
            };
            let msg = match session::route(&mut self.session, &mut self.ratcn, &self.state, event)?
            {
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
            self.update(msg)?;
        }
        Ok(())
    }

    fn update(&mut self, msg: Msg) -> Result<()> {
        let state = &mut self.state;
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
                if !source.is_empty() {
                    let target = state.target_input.value().trim().to_string();
                    state.config.text.replace.insert(source.to_string(), target);
                    save_replace_rules(&state.config.text.replace)?;
                    state.refresh_rules();
                }
                state.close_form();
            }
            Msg::Delete => {
                if let Some(index) = state.selected {
                    state.config.text.replace.shift_remove_index(index);
                    save_replace_rules(&state.config.text.replace)?;
                    state.refresh_rules();
                }
            }
            Msg::Dismiss => state.close_form(),
        }
        Ok(())
    }
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
        state.modals.open(FORM, &mut state.focus).unwrap();
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
        state.focus = FocusState::intent([FORM, "input-1"]);
        paint(&state, &mut ratcn);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Enter, &state),
            EventResult::Emit(Msg::Add)
        ));
    }
}
