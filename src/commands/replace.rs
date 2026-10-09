use crate::config::OsttConfig;
use crate::ui::components::list::ListView;
use crate::ui::components::modal::{form_dialog, ModalAction, ModalView};
use crate::ui::{render_app_layout, render_footer, render_title};
use anyhow::Result;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratcn::InputState as Input;
use std::fs;
use std::io::{self, Stdout};

pub async fn handle_replace() -> Result<()> {
    let config = OsttConfig::load().map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut view = ReplaceView::new(config)?;
    view.run()
}

struct ReplaceView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    list: ListView,
    config: OsttConfig,
    input_mode: bool,
    source_input: Input,
    target_input: Input,
    modal: ModalView,
    _input_modes: ratcn::crossterm::InputModeGuard,
    cleaned_up: bool,
}

impl ReplaceView {
    fn new(config: OsttConfig) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let input_modes = ratcn::crossterm::InputModes::new().paste().enable()?;
        Ok(Self {
            terminal,
            list: ListView::default(),
            config,
            input_mode: false,
            source_input: Input::default(),
            target_input: Input::default(),
            modal: ModalView::default(),
            _input_modes: input_modes,
            cleaned_up: false,
        })
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.draw()?;

            let event = event::read()?;
            if matches!(&event, Event::Key(key) if crate::ui::is_ctrl_c(key)) {
                break;
            }
            if self.input_mode {
                if let Some(action) = self.modal.handle_event(event) {
                    match action {
                        ModalAction::Changed(index) => {
                            if index == 0 {
                                self.source_input = self.modal.input(index).clone();
                            } else {
                                self.target_input = self.modal.input(index).clone();
                            }
                        }
                        ModalAction::Submit(0) => self.modal.focus_input("replace", 1),
                        ModalAction::Submit(_) | ModalAction::Accept => self.add_replace_rule()?,
                        ModalAction::Dismiss => self.reset_input(),
                    }
                }
                continue;
            }
            if let Event::Key(key) = &event {
                match key.code {
                    _ if crate::ui::is_cancel_key(key) => break,
                    KeyCode::Char('x') | KeyCode::Delete => self.delete_selected_replace_rule()?,
                    KeyCode::Char('a') => self.input_mode = true,
                    _ => {}
                }
            }
            self.list.handle_event(event);
        }

        self.cleanup()?;
        Ok(())
    }

    fn add_replace_rule(&mut self) -> Result<()> {
        let source = self.source_input.value().trim();
        if source.is_empty() {
            return Ok(());
        }

        self.config.text.replace.insert(
            source.to_string(),
            self.target_input.value().trim().to_string(),
        );
        save_replace_rules(&self.config.text.replace)?;
        self.reset_input();
        Ok(())
    }

    fn reset_input(&mut self) {
        self.input_mode = false;
        self.source_input = Input::default();
        self.target_input = Input::default();
    }

    fn delete_selected_replace_rule(&mut self) -> Result<()> {
        let Some(index) = self.list.selected() else {
            return Ok(());
        };
        self.config.text.replace.shift_remove_index(index);
        save_replace_rules(&self.config.text.replace)?;
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let replace_rules = self
            .config
            .text
            .replace
            .iter()
            .map(|(source, target)| format!("{source} → {target}"))
            .collect::<Vec<_>>();
        let input_mode = self.input_mode;
        let inputs = [&self.source_input, &self.target_input];
        self.modal.sync(
            input_mode.then_some("replace"),
            if input_mode { &inputs } else { &[] },
        )?;
        let list = &mut self.list;
        let modal = &mut self.modal;

        self.terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "Replace");
            list.render(frame, layout.body, &replace_rules, 1);

            if input_mode {
                modal.render(frame, |state| {
                    form_dialog("New replace", "", &["Find", "Replace"], "Add", state.offset)
                });
                render_footer(frame, layout.footer, "↵ next/add, tab switch, esc cancel");
            } else {
                render_footer(
                    frame,
                    layout.footer,
                    "↑↓ select, x/del delete, a add, esc/q exit",
                );
            }
        })?;

        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        if self.cleaned_up {
            return Ok(());
        }
        self.cleaned_up = true;
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for ReplaceView {
    fn drop(&mut self) {
        let _ = self.cleanup();
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
}
