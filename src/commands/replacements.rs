use crate::config::OsttConfig;
use crate::ui::{render_app_layout, render_footer, render_title};
use anyhow::Result;
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};
use std::fs;
use std::io::{self, Stdout};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

pub async fn handle_replacements() -> Result<()> {
    let config = OsttConfig::load().map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut view = ReplacementsView::new(config)?;
    view.run()
}

enum InputField {
    Source,
    Replacement,
}

struct ReplacementsView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    list_state: ListState,
    config: OsttConfig,
    input_mode: bool,
    active_field: InputField,
    source_input: Input,
    replacement_input: Input,
    cleaned_up: bool,
}

impl ReplacementsView {
    fn new(config: OsttConfig) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let mut list_state = ListState::default();
        if !config.text.replacements.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            terminal,
            list_state,
            config,
            input_mode: false,
            active_field: InputField::Source,
            source_input: Input::default(),
            replacement_input: Input::default(),
            cleaned_up: false,
        })
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.draw()?;

            match event::read()? {
                Event::Key(key) => {
                    if self.input_mode {
                        self.handle_input_mode_key(key)?;
                    } else if self.handle_normal_mode_key(key)? {
                        break;
                    }
                }
                Event::Mouse(mouse) if !self.input_mode => match mouse.kind {
                    MouseEventKind::ScrollUp => self.list_state.select_previous(),
                    MouseEventKind::ScrollDown => self.list_state.select_next(),
                    _ => {}
                },
                _ => {}
            }
        }

        self.cleanup()?;
        Ok(())
    }

    fn handle_normal_mode_key(&mut self, key: KeyEvent) -> Result<bool> {
        if crate::ui::is_ctrl_c(&key) {
            return Ok(true);
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Up => self.list_state.select_previous(),
            KeyCode::Down => self.list_state.select_next(),
            KeyCode::Char('x') | KeyCode::Delete => self.delete_selected_replacement()?,
            KeyCode::Char('a') => {
                self.input_mode = true;
                self.active_field = InputField::Source;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_input_mode_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => match self.active_field {
                InputField::Source => self.active_field = InputField::Replacement,
                InputField::Replacement => self.add_replacement()?,
            },
            KeyCode::Tab | KeyCode::BackTab => self.toggle_active_field(),
            KeyCode::Esc => self.reset_input(),
            _ => match self.active_field {
                InputField::Source => {
                    self.source_input.handle_event(&Event::Key(key));
                }
                InputField::Replacement => {
                    self.replacement_input.handle_event(&Event::Key(key));
                }
            },
        }
        Ok(())
    }

    fn toggle_active_field(&mut self) {
        self.active_field = match self.active_field {
            InputField::Source => InputField::Replacement,
            InputField::Replacement => InputField::Source,
        };
    }

    fn add_replacement(&mut self) -> Result<()> {
        let source = self.source_input.value().trim();
        if source.is_empty() {
            return Ok(());
        }

        self.config.text.replacements.insert(
            source.to_string(),
            self.replacement_input.value().trim().to_string(),
        );
        save_replacements(&self.config.text.replacements)?;
        self.select_valid_index();
        self.reset_input();
        Ok(())
    }

    fn reset_input(&mut self) {
        self.input_mode = false;
        self.active_field = InputField::Source;
        self.source_input = Input::default();
        self.replacement_input = Input::default();
    }

    fn delete_selected_replacement(&mut self) -> Result<()> {
        let Some(index) = self.list_state.selected() else {
            return Ok(());
        };
        self.config.text.replacements.shift_remove_index(index);
        save_replacements(&self.config.text.replacements)?;
        self.select_valid_index();
        Ok(())
    }

    fn select_valid_index(&mut self) {
        let len = self.config.text.replacements.len();
        if len == 0 {
            self.list_state.select(None);
            return;
        }
        let index = self.list_state.selected().unwrap_or(0).min(len - 1);
        self.list_state.select(Some(index));
    }

    fn draw(&mut self) -> Result<()> {
        let replacements = self
            .config
            .text
            .replacements
            .iter()
            .map(|(source, replacement)| (source.clone(), replacement.clone()))
            .collect::<Vec<_>>();
        let input_mode = self.input_mode;
        let source_value = self.source_input.value().to_string();
        let replacement_value = self.replacement_input.value().to_string();
        let source_cursor = self.source_input.cursor();
        let replacement_cursor = self.replacement_input.cursor();
        let source_active = matches!(self.active_field, InputField::Source);
        let list_state = &mut self.list_state;

        self.terminal.draw(|frame| {
            let layout = render_app_layout(frame, frame.area());
            render_title(frame, layout.title, "Replacements");
            Self::render_replacements_list(frame, layout.body, &replacements, list_state);

            if input_mode {
                Self::render_add_dialog(
                    frame,
                    &source_value,
                    &replacement_value,
                    source_cursor,
                    replacement_cursor,
                    source_active,
                );
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

    fn render_replacements_list(
        frame: &mut Frame,
        area: Rect,
        replacements: &[(String, String)],
        list_state: &mut ListState,
    ) {
        let items = replacements
            .iter()
            .map(|(source, replacement)| ListItem::new(format!("{source} → {replacement}")))
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::default())
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_stateful_widget(list, area, list_state);
    }

    fn render_add_dialog(
        frame: &mut Frame,
        source: &str,
        replacement: &str,
        source_cursor: usize,
        replacement_cursor: usize,
        source_active: bool,
    ) {
        let area = crate::ui::components::dialog::centered_fixed_rect(70, 11, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::DarkGray)),
            area,
        );

        let inner = Rect {
            x: area.x.saturating_add(2),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };
        let title = "New replacement";
        let escape = "esc";
        let spacer_width = inner
            .width
            .saturating_sub((title.len() + escape.len()) as u16);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(title, Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::raw(" ".repeat(spacer_width as usize)),
                Span::styled(escape, Style::default().fg(Color::Gray)),
            ]))
            .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
            inner,
        );

        let source_label_area = Rect {
            y: inner.y.saturating_add(2),
            height: 1,
            ..inner
        };
        let source_input_area = Rect {
            y: inner.y.saturating_add(3),
            height: 1,
            ..inner
        };
        let replacement_label_area = Rect {
            y: inner.y.saturating_add(5),
            height: 1,
            ..inner
        };
        let replacement_input_area = Rect {
            y: inner.y.saturating_add(6),
            height: 1,
            ..inner
        };

        Self::render_label(frame, source_label_area, "Find");
        Self::render_input(frame, source_input_area, source, source_active);
        Self::render_label(frame, replacement_label_area, "Replace");
        Self::render_input(frame, replacement_input_area, replacement, !source_active);

        let action_area = Rect {
            y: inner.y.saturating_add(8),
            height: 1,
            ..inner
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "<Add>",
                Style::default().fg(Color::Black).bg(Color::White),
            )))
            .style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .alignment(Alignment::Center),
            action_area,
        );

        let (cursor_area, cursor) = if source_active {
            (source_input_area, source_cursor)
        } else {
            (replacement_input_area, replacement_cursor)
        };
        let cursor_x = cursor_area.x.saturating_add(cursor as u16);
        frame.set_cursor_position(Position::new(cursor_x, cursor_area.y));
    }

    fn render_label(frame: &mut Frame, area: Rect, label: &str) {
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(Color::White).bg(Color::DarkGray)),
            area,
        );
    }

    fn render_input(frame: &mut Frame, area: Rect, value: &str, active: bool) {
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray).bg(Color::Gray)
        };
        frame.render_widget(Paragraph::new(value.to_string()).style(style), area);
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

impl Drop for ReplacementsView {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn save_replacements(replacements: &indexmap::IndexMap<String, String>) -> Result<()> {
    let config_path = crate::app_dirs::config_path()?;
    let content = fs::read_to_string(&config_path)?;
    let updated = replace_text_section(&content, replacements);
    fs::write(config_path, updated)?;
    Ok(())
}

fn replace_text_section(
    content: &str,
    replacements: &indexmap::IndexMap<String, String>,
) -> String {
    let without_text = remove_top_level_section(content, "text");
    if replacements.is_empty() {
        return ensure_trailing_newline(&without_text);
    }

    let section = render_text_replacements_section(replacements);
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

fn render_text_replacements_section(replacements: &indexmap::IndexMap<String, String>) -> String {
    let mut section = String::from("[text.replacements]\n");
    for (source, replacement) in replacements {
        section.push_str(&format!(
            "{} = {}\n",
            toml_basic_string(source),
            toml_basic_string(replacement)
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

    fn replacements(entries: &[(&str, &str)]) -> IndexMap<String, String> {
        entries
            .iter()
            .map(|(source, replacement)| (source.to_string(), replacement.to_string()))
            .collect()
    }

    #[test]
    fn replacing_text_section_preserves_transcription_section() {
        let content = r#"[audio]
device = "default"

[transcription]
provider = "deepgram"
model = "nova-3"

[text.replacements]
"api" = "API"

[popup]
width = 90
"#;

        let updated = replace_text_section(content, &replacements(&[("ostt", "OSTT")]));

        assert!(updated.contains("provider = \"deepgram\""));
        assert!(updated.contains("model = \"nova-3\""));
        assert!(updated.contains("[text.replacements]\n\"ostt\" = \"OSTT\""));
        assert!(updated.contains("\"ostt\" = \"OSTT\"\n\n[popup]"));
        assert!(!updated.contains("\"api\" = \"API\""));
    }

    #[test]
    fn empty_replacements_remove_text_section_only() {
        let content = r#"[audio]
device = "default"

[transcription]
provider = "deepgram"
model = "nova-3"

[text]

[text.replacements]
"api" = "API"

[popup]
width = 90
"#;

        let updated = replace_text_section(content, &IndexMap::new());

        assert!(updated.contains("[transcription]"));
        assert!(!updated.contains("[text]"));
        assert!(!updated.contains("[text.replacements]"));
        assert!(updated.contains("[popup]"));
    }
}
