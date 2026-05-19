use crate::commands::local_models;
use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{load_state, model_destination};
use crate::transcription::{TranscriptionModel, TranscriptionProvider};
use crate::ui::{render_toast, Toast};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Padding, Paragraph};
use ratatui::{Frame, Terminal};
use std::collections::HashSet;
use std::io::{self, Stdout};
use std::time::Duration;

const LOGO: &str = " ┏┓┏╋╋ \n ┗┛┛┗┗ \n";

#[derive(Debug, Clone)]
pub struct CloudModelEntry {
    pub provider_id: String,
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub model: TranscriptionModel,
}

#[derive(Debug, Clone)]
pub struct CloudProviderSection {
    pub provider_id: String,
    pub title: String,
    pub models: Vec<CloudModelEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelProviderChoice {
    Local,
    Cloud,
    Quit,
}

#[derive(Debug)]
pub struct UserQuit;

impl std::fmt::Display for UserQuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("quit")
    }
}

impl std::error::Error for UserQuit {}

pub async fn handle_model() -> anyhow::Result<()> {
    let mut guard = TerminalGuard::new()?;

    loop {
        let choice = choose_model_provider(&mut guard.terminal).await?;
        let result = match choice {
            ModelProviderChoice::Quit => break,
            ModelProviderChoice::Local => {
                local_models::handle_local_models_with_terminal(&mut guard.terminal).await
            }
            ModelProviderChoice::Cloud => run_cloud_model_selector(&mut guard.terminal).await,
        };
        if let Err(e) = result {
            if e.downcast_ref::<UserQuit>().is_some() {
                break;
            }
            return Err(e);
        }
    }
    Ok(())
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn render_shell(frame: &mut Frame<'_>) -> [Rect; 3] {
    let area = frame.area();

    let padding_block = Block::default().padding(Padding::new(1, 1, 1, 0));
    frame.render_widget(&padding_block, area);
    let padded_area = padding_block.inner(area);

    let main_block = Block::default();
    frame.render_widget(&main_block, padded_area);
    let inner_area = main_block.inner(padded_area);

    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(inner_area);

    frame.render_widget(Paragraph::new(LOGO).alignment(Alignment::Left), header_area);

    [body_area, footer_area, area]
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, text: &'static str) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::Black)),
        area,
    );
}

fn render_title(frame: &mut Frame<'_>, area: Rect, title: &'static str) -> Rect {
    let [title_area, content_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let label = format!(" {title} ");
    frame.render_widget(
        Paragraph::new(label.clone()).style(Style::default().fg(Color::Black).bg(Color::Blue)),
        Rect {
            width: label.len() as u16,
            height: 1,
            ..title_area
        },
    );
    content_area
}

async fn choose_model_provider(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<ModelProviderChoice> {
    let choices = ["Local provider", "Cloud provider"];
    let mut selected = 0_usize;

    loop {
        terminal.draw(|frame| {
            let [body_area, footer_area, _] = render_shell(frame);
            let list_area = render_title(frame, body_area, "Provider");

            let items: Vec<ListItem> = choices
                .iter()
                .map(|choice| ListItem::new(Line::from(choice.to_string())))
                .collect();

            let mut state = ListState::default().with_selected(Some(selected));
            frame.render_stateful_widget(
                List::new(items)
                    .highlight_style(Style::default().fg(Color::White).bg(Color::Black))
                    .highlight_symbol("> "),
                list_area,
                &mut state,
            );

            render_footer(frame, footer_area, "↑↓ select, ↵ confirm, esc/q quit");
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(1),
                    KeyCode::Enter => {
                        return Ok(match selected {
                            0 => ModelProviderChoice::Local,
                            _ => ModelProviderChoice::Cloud,
                        })
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(ModelProviderChoice::Quit),
                    _ if is_ctrl_c(&key) => return Ok(ModelProviderChoice::Quit),
                    _ => {}
                }
            }
        }
    }
}

async fn run_cloud_model_selector(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    let authorized_provider_ids = config::get_authorized_providers()?;

    if authorized_provider_ids.is_empty() {
        show_no_cloud_providers_screen(terminal).await?;
        return Ok(());
    }

    let selected_model = config::get_selected_model_entry()?;
    let mut sections =
        build_cloud_provider_sections(&authorized_provider_ids, selected_model.as_ref());
    let model_count = cloud_model_count(&sections);

    if model_count == 0 {
        show_no_cloud_providers_screen(terminal).await?;
        return Ok(());
    }

    let mut toast: Option<Toast> = None;
    let mut selected = active_cloud_model_index(&sections).unwrap_or(0);

    loop {
        if toast.as_ref().is_some_and(Toast::is_expired) {
            toast = None;
        }

        terminal.draw(|frame| {
            let [body_area, footer_area, _] = render_shell(frame);
            let list_area = render_title(frame, body_area, "Cloud Model");
            let (items, selected_display_index) = cloud_model_list_items(&sections, selected);
            let mut state = ListState::default().with_selected(selected_display_index);
            frame.render_stateful_widget(
                List::new(items)
                    .highlight_style(Style::default().fg(Color::White).bg(Color::Black))
                    .highlight_symbol("> "),
                list_area,
                &mut state,
            );

            render_footer(frame, footer_area, "↑↓ select, ↵ activate, esc/q back");

            if let Some(toast) = &toast {
                render_toast(frame, toast);
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(model_count.saturating_sub(1)),
                    KeyCode::Enter => {
                        if let Some(entry) = cloud_model_at(&sections, selected).cloned() {
                            save_cloud_selection(&entry.provider_id, &entry.model)?;
                            mark_active_cloud_model(
                                &mut sections,
                                &entry.provider_id,
                                &entry.model_id,
                            );
                            toast = Some(Toast::success(format!("Activated {}", entry.name)));
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ if is_ctrl_c(&key) => return Err(UserQuit.into()),
                    _ => {}
                }
            }
        }
    }
}

fn cloud_model_list_item(entry: &CloudModelEntry) -> ListItem<'static> {
    let marker = if entry.is_active { "◉" } else { "○" };
    ListItem::new(Line::from(format!("{marker} {}", entry.name)))
}

fn cloud_model_list_items(
    sections: &[CloudProviderSection],
    selected: usize,
) -> (Vec<ListItem<'static>>, Option<usize>) {
    let mut items = Vec::new();
    let mut display_index = 0_usize;
    let mut model_index = 0_usize;
    let mut selected_display_index = None;

    for section in sections {
        items.push(ListItem::new(Line::from(Span::styled(
            section.title.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ))));
        display_index += 1;

        for entry in &section.models {
            if model_index == selected {
                selected_display_index = Some(display_index);
            }
            items.push(cloud_model_list_item(entry));
            display_index += 1;
            model_index += 1;
        }

        items.push(ListItem::new(Line::from("")));
        display_index += 1;
    }

    (items, selected_display_index)
}

fn cloud_model_count(sections: &[CloudProviderSection]) -> usize {
    sections.iter().map(|section| section.models.len()).sum()
}

fn cloud_model_at(sections: &[CloudProviderSection], index: usize) -> Option<&CloudModelEntry> {
    sections
        .iter()
        .flat_map(|section| section.models.iter())
        .nth(index)
}

fn active_cloud_model_index(sections: &[CloudProviderSection]) -> Option<usize> {
    sections
        .iter()
        .flat_map(|section| section.models.iter())
        .position(|entry| entry.is_active)
}

fn mark_active_cloud_model(
    sections: &mut [CloudProviderSection],
    provider_id: &str,
    model_id: &str,
) {
    for entry in sections
        .iter_mut()
        .flat_map(|section| section.models.iter_mut())
    {
        entry.is_active = entry.provider_id == provider_id && entry.model_id == model_id;
    }
}

async fn show_no_cloud_providers_screen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            let [body_area, footer_area, _] = render_shell(frame);
            let content_area = render_title(frame, body_area, "Cloud Provider");

            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        "No cloud providers authenticated.",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("Run `ostt auth login` to add credentials."),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Available providers: OpenAI, Groq",
                        Style::default(),
                    )),
                ])
                .wrap(ratatui::widgets::Wrap { trim: false }),
                content_area,
            );

            render_footer(frame, footer_area, "esc/q back");
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ if is_ctrl_c(&key) => return Err(UserQuit.into()),
                    _ => {}
                }
            }
        }
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

pub fn build_cloud_provider_sections(
    authorized_provider_ids: &[String],
    selected_model: Option<&SelectedModel>,
) -> Vec<CloudProviderSection> {
    let authorized: HashSet<&str> = authorized_provider_ids.iter().map(String::as_str).collect();

    TranscriptionProvider::all()
        .iter()
        .filter(|provider| **provider != TranscriptionProvider::Local)
        .filter(|provider| authorized.contains(provider.id()))
        .filter_map(|provider| {
            let models: Vec<CloudModelEntry> = TranscriptionModel::models_for_provider(provider)
                .into_iter()
                .map(|model| CloudModelEntry {
                    provider_id: provider.id().to_string(),
                    model_id: model.id().to_string(),
                    name: model.description().to_string(),
                    description: String::new(),
                    is_active: selected_model
                        .map(|selected| {
                            selected.provider_id == provider.id() && selected.model_id == model.id()
                        })
                        .unwrap_or(false),
                    model,
                })
                .collect();

            (!models.is_empty()).then(|| CloudProviderSection {
                provider_id: provider.id().to_string(),
                title: provider.name().to_string(),
                models,
            })
        })
        .collect()
}

pub fn save_cloud_selection(provider_id: &str, model: &TranscriptionModel) -> anyhow::Result<()> {
    config::save_selected_model(provider_id, model.id())
}

pub fn no_selected_model_error() -> anyhow::Error {
    anyhow::anyhow!(
        "No transcription model selected.\n\nRun `ostt model` to choose an online or local transcription model.\nRun `ostt auth login` first to add credentials for cloud providers."
    )
}

pub fn missing_cloud_credentials_error(provider: &TranscriptionProvider) -> anyhow::Error {
    anyhow::anyhow!(
        "{} is selected, but no {} API key is configured.\n\nRun `ostt auth login` and choose {}.",
        provider.name(),
        provider.name(),
        provider.name()
    )
}

pub fn missing_local_model_error(model_id: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Local model `{model_id}` is selected but not downloaded.\n\nRun `ostt model` to download or select a local model."
    )
}

pub fn validate_selected_model_is_usable(selected: &SelectedModel) -> anyhow::Result<()> {
    if selected.provider_id == "local" {
        let state = load_state();
        let entry = state
            .custom_models
            .iter()
            .find(|entry| entry.id == selected.model_id);

        if entry.is_none_or(|entry| !model_destination(entry).exists()) {
            return Err(missing_local_model_error(&selected.model_id));
        }

        return Ok(());
    }

    let provider = TranscriptionProvider::from_id(&selected.provider_id).ok_or_else(|| {
        anyhow::anyhow!("Unknown transcription provider: {}", selected.provider_id)
    })?;
    if config::get_api_key(provider.id())?.is_none() {
        return Err(missing_cloud_credentials_error(&provider));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
        crate::transcription::local_models::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct TestEnv {
        previous_home: Option<std::ffi::OsString>,
        dir: std::path::PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let previous_home = std::env::var_os("HOME");
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("ostt-model-command-test-{unique}"));
            fs::create_dir_all(&dir).expect("create temp dir");
            std::env::set_var("HOME", &dir);
            crate::transcription::local_models::set_test_models_dir(Some(dir.join("models")));

            Self { previous_home, dir }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            if let Some(previous_home) = self.previous_home.take() {
                std::env::set_var("HOME", previous_home);
            } else {
                std::env::remove_var("HOME");
            }

            crate::transcription::local_models::set_test_models_dir(None);

            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn cloud_sections_include_only_authenticated_providers() {
        let sections =
            build_cloud_provider_sections(&["openai".to_string(), "local".to_string()], None);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].provider_id, "openai");
        assert!(sections[0]
            .models
            .iter()
            .all(|entry| entry.provider_id == "openai"));
    }

    #[test]
    fn recovery_messages_are_actionable() {
        assert!(no_selected_model_error().to_string().contains("ostt model"));
        assert!(
            missing_cloud_credentials_error(&TranscriptionProvider::OpenAI)
                .to_string()
                .contains("ostt auth login")
        );
        assert!(missing_local_model_error("turbo")
            .to_string()
            .contains("download or select"));
    }

    #[test]
    fn selection_save_helpers_persist_provider_aware_state() {
        let _guard = test_env_lock();
        let _env = TestEnv::new();

        save_cloud_selection("openai", &TranscriptionModel::Whisper).expect("save cloud selection");
        let config = config::OsttConfig::load().expect("load config");
        assert_eq!(config.transcription.provider.as_deref(), Some("openai"));
        assert_eq!(
            config.transcription.model.as_deref(),
            Some(TranscriptionModel::Whisper.id())
        );
        let selected = config::get_selected_model_entry()
            .expect("load selection")
            .expect("selected cloud model");
        assert_eq!(selected.provider_id, "openai");
        assert_eq!(selected.model_id, TranscriptionModel::Whisper.id());
    }
}
