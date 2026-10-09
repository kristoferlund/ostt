use crate::config::OsttConfig;
use crate::ui::components::list::{clamp_selection, selection_list};
use crate::ui::components::modal::{dialog, form_dialog};
use crate::ui::{render_app_layout, render_themed_footer, render_themed_title, session};
use anyhow::Result;
use ratcn::{
    runtime::{CellOffset, Event, EventResult, FocusState, KeyCode, ModalState, Ratcn},
    terminal::Session,
    Button, Input, InputState,
};
use std::fs;

pub async fn handle_replace() -> Result<()> {
    let config = OsttConfig::load().map_err(|err| anyhow::anyhow!(err.to_string()))?;
    ReplaceView::new(config)?.run()
}

struct State {
    focus: FocusState,
    modals: ModalState,
    selected: Option<usize>,
    config: OsttConfig,
    source_input: InputState,
    target_input: InputState,
    offset: CellOffset,
}

impl State {
    fn new(config: OsttConfig) -> Self {
        Self {
            config,
            focus: FocusState::default(),
            modals: ModalState::default(),
            selected: None,
            source_input: InputState::default(),
            target_input: InputState::default(),
            offset: CellOffset::default(),
        }
    }
}

enum Msg {
    Focus(FocusState),
    Select(usize),
    Source(InputState),
    Target(InputState),
    Move(CellOffset),
    Next,
    Add,
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
            self.draw()?;

            let Some(event) = session::next(&mut self.session, None)? else {
                continue;
            };
            let result = self.ratcn.handle_event(event.clone(), &self.state);
            if let Some(text) = self.ratcn.take_clipboard() {
                self.session.set_clipboard(&text)?;
            } else if session::is_ctrl_c(&event) {
                break;
            }
            match result {
                EventResult::Emit(msg) => self.update(msg)?,
                EventResult::Consumed => {}
                EventResult::Ignored => match event {
                    _ if session::is_cancel(&event) => break,
                    Event::Key(key) => match key.code {
                        KeyCode::Char('a') => {
                            self.state.offset = CellOffset::default();
                            self.state.modals.open("replace", &mut self.state.focus)?;
                        }
                        KeyCode::Char('x') | KeyCode::Delete => {
                            self.delete_selected_replace_rule()?
                        }
                        _ => {}
                    },
                    _ => {}
                },
            }
        }

        Ok(())
    }

    fn update(&mut self, msg: Msg) -> Result<()> {
        match msg {
            Msg::Focus(focus) => self.state.focus = focus,
            Msg::Select(index) => self.state.selected = Some(index),
            Msg::Source(input) => self.state.source_input = input,
            Msg::Target(input) => self.state.target_input = input,
            Msg::Move(offset) => self.state.offset = offset,
            Msg::Next => self.state.focus = FocusState::intent(["replace", "input-1"]),
            Msg::Add => self.add_replace_rule()?,
            Msg::Dismiss => self.reset_input(),
        }
        Ok(())
    }

    fn add_replace_rule(&mut self) -> Result<()> {
        let source = self.state.source_input.value().trim();
        if source.is_empty() {
            return Ok(());
        }

        self.state.config.text.replace.insert(
            source.to_string(),
            self.state.target_input.value().trim().to_string(),
        );
        save_replace_rules(&self.state.config.text.replace)?;
        self.reset_input();
        Ok(())
    }

    fn reset_input(&mut self) {
        self.state.modals.close(&mut self.state.focus);
        self.state.source_input = InputState::default();
        self.state.target_input = InputState::default();
    }

    fn delete_selected_replace_rule(&mut self) -> Result<()> {
        let Some(index) = self.state.selected else {
            return Ok(());
        };
        self.state.config.text.replace.shift_remove_index(index);
        save_replace_rules(&self.state.config.text.replace)?;
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let replace_rules = self
            .state
            .config
            .text
            .replace
            .iter()
            .map(|(source, target)| format!("{source} → {target}"))
            .collect::<Vec<_>>();
        clamp_selection(&mut self.state.selected, replace_rules.len());
        let theme = session::theme(&self.session);
        let state = &self.state;
        let ratcn = &mut self.ratcn;
        self.session
            .terminal_mut()
            .draw(|frame| render_replace(frame, state, &replace_rules, ratcn, &theme))?;
        self.session.set_pointer_shape(self.ratcn.pointer_shape())?;
        Ok(())
    }
}

fn render_replace(
    frame: &mut ratatui::Frame<'_>,
    state: &State,
    replace_rules: &[String],
    ratcn: &mut Ratcn<State, Msg>,
    theme: &ratcn::Theme,
) {
    let area = frame.area();
    session::paint_background(frame, theme);
    let layout = render_app_layout(frame, frame.area());
    render_themed_title(frame, layout.title, "Replace", theme);
    ratcn.render(frame, area, state, theme, |ctx| {
        ctx.component(
            "list",
            selection_list(
                replace_rules,
                1,
                |s: &State| s.selected,
                Msg::Select,
                Msg::Select,
            ),
            layout.body,
        );
        if state.modals.is_open("replace") {
            let form = dialog("New replace", "", Button::new("Add").on_press(|| Msg::Add))
                .offset(state.offset)
                .on_offset_change(Msg::Move)
                .on_dismiss(|| Msg::Dismiss);
            ctx.modal(
                "replace",
                form_dialog(
                    form,
                    "",
                    vec![
                        Input::new()
                            .title("Find")
                            .value(|s: &State| &s.source_input, Msg::Source)
                            .on_submit(|| Msg::Next),
                        Input::new()
                            .title("Replace")
                            .value(|s: &State| &s.target_input, Msg::Target)
                            .on_submit(|| Msg::Add),
                    ],
                ),
                area,
            );
        }
    });
    if state.modals.is_open("replace") {
        render_themed_footer(
            frame,
            layout.footer,
            "↵ next/add, tab switch, esc cancel",
            theme,
        );
    } else {
        render_themed_footer(
            frame,
            layout.footer,
            "↑↓ select, x/del delete, a add, esc/q exit",
            theme,
        );
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
    impl Default for State {
        fn default() -> Self {
            Self::new(OsttConfig::default())
        }
    }
    use indexmap::IndexMap;

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

    fn paint(
        state: &State,
        ratcn: &mut Ratcn<State, Msg>,
        terminal: &mut ratatui::Terminal<ratatui::backend::TestBackend>,
        theme: &ratcn::Theme,
    ) {
        let rows: Vec<_> = state
            .config
            .text
            .replace
            .iter()
            .map(|(source, target)| format!("{source} → {target}"))
            .collect();
        terminal
            .draw(|frame| render_replace(frame, state, &rows, ratcn, theme))
            .unwrap();
    }

    #[test]
    fn form_paste_and_enter_bind_directly_to_the_command_fields() {
        let mut state = State::default();
        state.modals.open("replace", &mut state.focus).unwrap();
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let theme = ratcn::Theme::default_dark();
        paint(&state, &mut ratcn, &mut terminal, &theme);
        let EventResult::Emit(Msg::Source(input)) =
            ratcn.handle_event(Event::Paste("日本語\nsource".into()), &state)
        else {
            panic!("the first input must own paste")
        };
        state.source_input = input;
        paint(&state, &mut ratcn, &mut terminal, &theme);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Enter, &state),
            EventResult::Emit(Msg::Next)
        ));
        state.focus = FocusState::intent(["replace", "input-1"]);
        paint(&state, &mut ratcn, &mut terminal, &theme);
        let EventResult::Emit(Msg::Target(input)) =
            ratcn.handle_event(Event::Paste("replacement".into()), &state)
        else {
            panic!("the second input must own paste after advancing")
        };
        state.target_input = input;
        assert_eq!(state.source_input.value(), "日本語 source");
        paint(&state, &mut ratcn, &mut terminal, &theme);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Enter, &state),
            EventResult::Emit(Msg::Add)
        ));
        assert!(matches!(
            ratcn.handle_event(KeyCode::Esc, &state),
            EventResult::Emit(Msg::Dismiss)
        ));
    }

    #[test]
    fn modal_captures_shortcuts_and_restores_the_real_background_focus() {
        let mut state = State {
            selected: Some(1),
            focus: FocusState::intent(["list"]),
            ..State::default()
        };
        state.config.text.replace.insert("one".into(), "1".into());
        state.config.text.replace.insert("two".into(), "2".into());
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let theme = ratcn::Theme::default_dark();
        paint(&state, &mut ratcn, &mut terminal, &theme);
        state.modals.open("replace", &mut state.focus).unwrap();
        assert!(
            matches!(
                ratcn.handle_event(KeyCode::Down, &state),
                EventResult::Consumed
            ),
            "opening before redraw must not navigate the list"
        );
        paint(&state, &mut ratcn, &mut terminal, &theme);
        assert!(
            matches!(
                ratcn.handle_event(KeyCode::Char('x'), &state),
                EventResult::Emit(Msg::Source(_))
            ),
            "the delete shortcut is text while editing"
        );
        assert_eq!(state.selected, Some(1));
        state.modals.close(&mut state.focus);
        assert_eq!(state.focus, FocusState::intent(["list"]));
        assert!(
            matches!(
                ratcn.handle_event(KeyCode::Enter, &state),
                EventResult::Consumed
            ),
            "closing before redraw must not submit the old form"
        );
        paint(&state, &mut ratcn, &mut terminal, &theme);
        assert!(matches!(
            ratcn.handle_event(KeyCode::Up, &state),
            EventResult::Emit(Msg::Select(0))
        ));
    }

    #[test]
    fn redraw_uses_the_new_adaptive_palette_instead_of_a_cached_dark_theme() {
        use ratatui::style::Color;
        let mut state = State::default();
        state.modals.open("replace", &mut state.focus).unwrap();
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let dark = ratcn::Theme::adaptive(Color::Rgb(26, 27, 38), Color::Rgb(192, 202, 245), None);
        let light =
            ratcn::Theme::adaptive(Color::Rgb(253, 246, 227), Color::Rgb(101, 123, 131), None);
        assert_eq!(dark.name, "Adaptive");
        assert_eq!(light.name, "Adaptive");
        paint(&state, &mut ratcn, &mut terminal, &dark);
        let previous = terminal.backend().buffer().clone();
        paint(&state, &mut ratcn, &mut terminal, &light);
        let buffer = terminal.backend().buffer();
        assert_ne!(&previous, buffer);
        assert!(
            buffer.content.iter().any(|cell| cell.bg == light.surface),
            "the dialog must use the active session palette"
        );
        assert!(
            buffer.content.iter().any(|cell| cell.bg == light.field),
            "inputs must use the same active palette"
        );
    }

    #[test]
    fn moved_dialog_keeps_its_position_and_clickable_action() {
        use ratcn::runtime::{Modifiers, MouseButton, MouseEvent, MouseKind as MouseEventKind};
        let mut state = State::default();
        state.modals.open("replace", &mut state.focus).unwrap();
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let theme = ratcn::Theme::default_dark();
        paint(&state, &mut ratcn, &mut terminal, &theme);
        let corner = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .position(|cell| cell.symbol() == "┌")
            .unwrap();
        let (x, y) = ((corner % 100) as u16, (corner / 100) as u16);
        let mouse = |kind, column, row| {
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: Modifiers::NONE,
            })
        };
        ratcn.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y), &state);
        let EventResult::Emit(Msg::Move(offset)) = ratcn.handle_event(
            mouse(MouseEventKind::Drag(MouseButton::Left), x + 4, y + 2),
            &state,
        ) else {
            panic!("drag must request an app-owned offset")
        };
        state.offset = offset;
        ratcn.handle_event(
            mouse(MouseEventKind::Up(MouseButton::Left), x + 4, y + 2),
            &state,
        );
        paint(&state, &mut ratcn, &mut terminal, &theme);
        assert_ne!(state.offset, CellOffset::default());
        let button = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .rposition(|cell| cell.symbol() == "A")
            .unwrap();
        let (x, y) = ((button % 100) as u16, (button / 100) as u16);
        ratcn.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y), &state);
        assert!(matches!(
            ratcn.handle_event(mouse(MouseEventKind::Up(MouseButton::Left), x, y), &state),
            EventResult::Emit(Msg::Add)
        ));
    }

    #[test]
    fn forms_remain_safe_in_clipped_terminals() {
        let mut state = State {
            source_input: InputState::new("long source 日本語"),
            ..State::default()
        };
        state.modals.open("replace", &mut state.focus).unwrap();
        let mut ratcn = runtime();
        for (width, height) in [(1, 1), (8, 4), (40, 10), (100, 30)] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
            paint(
                &state,
                &mut ratcn,
                &mut terminal,
                &ratcn::Theme::default_dark(),
            );
        }
    }

    #[test]
    fn ctrl_c_with_selected_input_produces_clipboard_text_for_the_session_host() {
        let mut state = State {
            source_input: InputState::new("日本語 source"),
            ..State::default()
        };
        state.modals.open("replace", &mut state.focus).unwrap();
        let mut ratcn = runtime();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let theme = ratcn::Theme::default_dark();
        paint(&state, &mut ratcn, &mut terminal, &theme);
        let ctrl = |code| {
            Event::Key(ratcn::runtime::KeyEvent {
                code,
                modifiers: ratcn::runtime::Modifiers {
                    ctrl: true,
                    ..ratcn::runtime::Modifiers::NONE
                },
            })
        };
        let EventResult::Emit(Msg::Source(input)) = ratcn.handle_event(
            Event::Key(ratcn::runtime::KeyEvent {
                code: KeyCode::Home,
                modifiers: ratcn::runtime::Modifiers {
                    shift: true,
                    ..ratcn::runtime::Modifiers::NONE
                },
            }),
            &state,
        ) else {
            panic!("Shift+Home must select the focused field")
        };
        state.source_input = input;
        paint(&state, &mut ratcn, &mut terminal, &theme);
        ratcn.handle_event(ctrl(KeyCode::Char('c')), &state);
        assert_eq!(
            ratcn.take_clipboard().as_deref(),
            Some("日本語 source"),
            "the host copies selected text instead of treating Ctrl+C as quit"
        );
    }
}
