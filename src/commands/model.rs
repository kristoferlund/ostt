use crate::commands::models_tui;
use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    load_state, model_destination, LocalModelState, RegistryEntry,
};
use crate::transcription::{TranscriptionModel, TranscriptionProvider};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph};
use ratatui::{Frame, Terminal};
use std::collections::HashSet;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(255, 255, 255);
const HIGHLIGHT_BG: Color = Color::Rgb(20, 20, 20);
const HELP_FG: Color = Color::Rgb(100, 100, 100);

#[derive(Debug, Clone)]
pub enum ModelSelectionEntryKind {
    Cloud {
        model: TranscriptionModel,
    },
    Local {
        entry: RegistryEntry,
        is_downloaded: bool,
    },
    LocalManagement,
}

#[derive(Debug, Clone)]
pub struct ModelSelectionEntry {
    pub provider_id: String,
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub kind: ModelSelectionEntryKind,
}

#[derive(Debug, Clone)]
pub struct ModelSelectionSection {
    pub provider_id: String,
    pub title: String,
    pub entries: Vec<ModelSelectionEntry>,
}

#[derive(Debug, Clone)]
pub enum ModelWizardMode {
    Browse,
    ConfirmDownload { entry: Box<ModelSelectionEntry> },
    Downloading(DownloadState),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadState {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f64,
    pub speed_mbps: f64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelWizardAction {
    Continue,
    Quit,
    ManageLocalModels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderChoice {
    Local,
    Cloud,
    Quit,
}

#[derive(Debug, Clone)]
pub struct ModelWizard {
    pub sections: Vec<ModelSelectionSection>,
    pub selected: usize,
    pub mode: ModelWizardMode,
    pub status_message: Option<String>,
    pub local_audio_warning: Option<String>,
    pub notification: Option<(String, Instant)>,
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
        let choice = show_provider_picker(&mut guard.terminal).await?;
        let result = match choice {
            ProviderChoice::Quit => break,
            ProviderChoice::Local => models_tui::handle_models_tui_with(&mut guard.terminal).await,
            ProviderChoice::Cloud => run_cloud_selector(&mut guard.terminal).await,
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

async fn show_provider_picker(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<ProviderChoice> {
    let choices = ["Local provider", "Cloud provider"];
    let mut selected = 0_usize;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let padding_block = Block::default()
                .padding(Padding::uniform(1))
                .style(Style::default().bg(BG));
            frame.render_widget(&padding_block, area);
            let padded_area = padding_block.inner(area);

            let main_block = Block::default().style(Style::default().fg(FG).bg(BG));
            frame.render_widget(&main_block, padded_area);
            let inner_area = main_block.inner(padded_area);

            let [header_area, list_area, footer_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .areas(inner_area);

            let header = Paragraph::new(" ┏┓┏╋╋ \n ┗┛┛┗┗ \n")
                .style(Style::default().fg(FG))
                .alignment(Alignment::Left);
            frame.render_widget(header, header_area);

            let items: Vec<ListItem> = choices
                .iter()
                .map(|choice| ListItem::new(Line::from(choice.to_string())))
                .collect();

            let mut state = ListState::default().with_selected(Some(selected));
            frame.render_stateful_widget(
                List::new(items)
                    .block(Block::default().title(" Provider ").borders(Borders::ALL))
                    .highlight_style(Style::default().bg(HIGHLIGHT_BG).fg(FG))
                    .highlight_symbol("> "),
                list_area,
                &mut state,
            );

            frame.render_widget(
                Paragraph::new("↑↓ select, ↵ confirm, esc/q quit")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(HELP_FG)),
                footer_area,
            );
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(1),
                    KeyCode::Enter => {
                        return Ok(match selected {
                            0 => ProviderChoice::Local,
                            _ => ProviderChoice::Cloud,
                        })
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(ProviderChoice::Quit),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(ProviderChoice::Quit)
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn run_cloud_selector(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    let authorized_provider_ids = config::get_authorized_providers()?;

    if authorized_provider_ids.is_empty() {
        show_no_providers_screen(terminal).await?;
        return Ok(());
    }

    let selected_model = config::get_selected_model_entry()?;
    let sections = build_cloud_sections(&authorized_provider_ids, selected_model.as_ref());
    let entries: Vec<&ModelSelectionEntry> =
        sections.iter().flat_map(|s| s.entries.iter()).collect();

    if entries.is_empty() {
        show_no_providers_screen(terminal).await?;
        return Ok(());
    }

    let mut notification: Option<(String, Instant)> = None;
    let mut selected = 0_usize;

    loop {
        if let Some((_, start_time)) = &notification {
            if start_time.elapsed() >= Duration::from_millis(750) {
                notification = None;
            }
        }

        terminal.draw(|frame| {
            let area = frame.area();

            let padding_block = Block::default()
                .padding(Padding::uniform(1))
                .style(Style::default().bg(BG));
            frame.render_widget(&padding_block, area);
            let padded_area = padding_block.inner(area);

            let main_block = Block::default().style(Style::default().fg(FG).bg(BG));
            frame.render_widget(&main_block, padded_area);
            let inner_area = main_block.inner(padded_area);

            let [header_area, list_area, footer_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .areas(inner_area);

            let header = Paragraph::new(" ┏┓┏╋╋ \n ┗┛┛┗┗ \n")
                .style(Style::default().fg(FG))
                .alignment(Alignment::Left);
            frame.render_widget(header, header_area);

            let mut items = Vec::new();
            let mut display_index = 0_usize;
            let mut model_index = 0_usize;
            let mut selected_display_index = None;
            for section in &sections {
                items.push(ListItem::new(Line::from(Span::styled(
                    section.title.to_string(),
                    Style::default().fg(Color::Rgb(120, 120, 120)),
                ))));
                display_index += 1;
                for entry in &section.entries {
                    if model_index == selected {
                        selected_display_index = Some(display_index);
                    }
                    items.push(cloud_list_item(entry));
                    display_index += 1;
                    model_index += 1;
                }
                items.push(ListItem::new(Line::from("")));
                display_index += 1;
            }

            let mut state = ListState::default().with_selected(selected_display_index);
            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .title(" Cloud Model ")
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().bg(HIGHLIGHT_BG).fg(FG))
                    .highlight_symbol("> "),
                list_area,
                &mut state,
            );

            frame.render_widget(
                Paragraph::new("↑↓ select, ↵ activate, esc/q back")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(HELP_FG)),
                footer_area,
            );

            if let Some((message, _)) = &notification {
                render_notification(frame, area, message);
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(entries.len().saturating_sub(1)),
                    KeyCode::Enter => {
                        if let Some(entry) = entries.get(selected) {
                            if let ModelSelectionEntryKind::Cloud { model } = &entry.kind {
                                save_cloud_selection(&entry.provider_id, model)?;
                                notification =
                                    Some((format!("Activated {}", entry.name), Instant::now()));
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Err(UserQuit.into())
                    }
                    _ => {}
                }
            }
        }
    }
}

fn cloud_list_item(entry: &ModelSelectionEntry) -> ListItem<'static> {
    let marker = if entry.is_active { "◉" } else { "○" };
    ListItem::new(Line::from(format!("{marker} {}", entry.name)))
}

async fn show_no_providers_screen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let padding_block = Block::default()
                .padding(Padding::uniform(1))
                .style(Style::default().bg(BG));
            frame.render_widget(&padding_block, area);
            let padded_area = padding_block.inner(area);

            let main_block = Block::default().style(Style::default().fg(FG).bg(BG));
            frame.render_widget(&main_block, padded_area);
            let inner_area = main_block.inner(padded_area);

            let [header_area, content_area, footer_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .areas(inner_area);

            let header = Paragraph::new(" ┏┓┏╋╋ \n ┗┛┛┗┗ \n")
                .style(Style::default().fg(FG))
                .alignment(Alignment::Left);
            frame.render_widget(header, header_area);

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
                        Style::default().fg(HELP_FG),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Cloud Provider ")
                        .borders(Borders::ALL),
                ),
                content_area,
            );

            frame.render_widget(
                Paragraph::new("esc/q back")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(HELP_FG)),
                footer_area,
            );
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Err(UserQuit.into())
                    }
                    _ => {}
                }
            }
        }
    }
}

impl ModelWizard {
    pub fn new(sections: Vec<ModelSelectionSection>, local_audio_warning: Option<String>) -> Self {
        Self {
            sections,
            selected: 0,
            mode: ModelWizardMode::Browse,
            status_message: None,
            local_audio_warning,
            notification: None,
        }
    }

    pub fn selectable_entries(&self) -> Vec<&ModelSelectionEntry> {
        self.sections
            .iter()
            .flat_map(|section| section.entries.iter())
            .collect()
    }

    pub fn selected_entry(&self) -> Option<&ModelSelectionEntry> {
        self.selectable_entries().get(self.selected).copied()
    }

    pub fn move_down(&mut self) {
        let len = self.selectable_entries().len();
        if self.selected + 1 < len {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn back(&mut self) -> ModelWizardAction {
        match self.mode {
            ModelWizardMode::Browse => ModelWizardAction::Quit,
            _ => {
                self.mode = ModelWizardMode::Browse;
                ModelWizardAction::Continue
            }
        }
    }

    pub fn select_current(&mut self) -> anyhow::Result<ModelWizardAction> {
        let entry = match self.selected_entry().cloned() {
            Some(entry) => entry,
            None => return Ok(ModelWizardAction::Continue),
        };

        match &entry.kind {
            ModelSelectionEntryKind::Cloud { model } => {
                save_cloud_selection(&entry.provider_id, model)?;
                self.notification = Some((format!("Activated {}", entry.name), Instant::now()));
                mark_active(&mut self.sections, &entry.provider_id, &entry.model_id);
                Ok(ModelWizardAction::Continue)
            }
            ModelSelectionEntryKind::Local { is_downloaded, .. } if *is_downloaded => {
                save_local_selection(&entry.model_id)?;
                self.notification = Some((format!("Activated {}", entry.name), Instant::now()));
                mark_active(&mut self.sections, "local", &entry.model_id);
                Ok(ModelWizardAction::Continue)
            }
            ModelSelectionEntryKind::Local { .. } => {
                self.mode = ModelWizardMode::ConfirmDownload {
                    entry: Box::new(entry),
                };
                Ok(ModelWizardAction::Continue)
            }
            ModelSelectionEntryKind::LocalManagement => Ok(ModelWizardAction::ManageLocalModels),
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

pub fn build_model_sections(
    authorized_provider_ids: &[String],
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<ModelSelectionSection> {
    let mut sections = build_cloud_sections(authorized_provider_ids, selected_model);
    sections.push(build_local_section(local_state, registry, selected_model));
    sections
}

pub fn build_cloud_sections(
    authorized_provider_ids: &[String],
    selected_model: Option<&SelectedModel>,
) -> Vec<ModelSelectionSection> {
    let authorized: HashSet<&str> = authorized_provider_ids.iter().map(String::as_str).collect();

    TranscriptionProvider::all()
        .iter()
        .filter(|provider| **provider != TranscriptionProvider::Local)
        .filter(|provider| authorized.contains(provider.id()))
        .filter_map(|provider| {
            let entries: Vec<ModelSelectionEntry> =
                TranscriptionModel::models_for_provider(provider)
                    .into_iter()
                    .map(|model| ModelSelectionEntry {
                        provider_id: provider.id().to_string(),
                        model_id: model.id().to_string(),
                        name: model.description().to_string(),
                        description: String::new(),
                        is_active: selected_model
                            .map(|selected| {
                                selected.provider_id == provider.id()
                                    && selected.model_id == model.id()
                            })
                            .unwrap_or(false),
                        kind: ModelSelectionEntryKind::Cloud { model },
                    })
                    .collect();

            (!entries.is_empty()).then(|| ModelSelectionSection {
                provider_id: provider.id().to_string(),
                title: provider.name().to_string(),
                entries,
            })
        })
        .collect()
}

pub fn build_local_section(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> ModelSelectionSection {
    let mut seen = HashSet::new();
    let mut entries: Vec<ModelSelectionEntry> = registry
        .iter()
        .chain(local_state.custom_models.iter())
        .filter(|entry| seen.insert(entry.id.as_str()))
        .map(|entry| {
            let is_downloaded = model_destination(entry).exists();
            ModelSelectionEntry {
                provider_id: "local".to_string(),
                model_id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                is_active: selected_model
                    .map(|selected| {
                        selected.provider_id == "local" && selected.model_id == entry.id
                    })
                    .unwrap_or(false),
                kind: ModelSelectionEntryKind::Local {
                    entry: entry.clone(),
                    is_downloaded,
                },
            }
        })
        .collect();

    entries.push(ModelSelectionEntry {
        provider_id: "local".to_string(),
        model_id: "__manage_local_models__".to_string(),
        name: "Manage local models...".to_string(),
        description: "Download, inspect, or remove local models".to_string(),
        is_active: false,
        kind: ModelSelectionEntryKind::LocalManagement,
    });

    ModelSelectionSection {
        provider_id: "local".to_string(),
        title: "Local".to_string(),
        entries,
    }
}

pub fn load_local_selection_state() -> anyhow::Result<LocalModelState> {
    Ok(load_state())
}

pub fn save_local_selection(model_id: &str) -> anyhow::Result<()> {
    config::save_selected_model("local", model_id)
}

pub fn save_cloud_selection(provider_id: &str, model: &TranscriptionModel) -> anyhow::Result<()> {
    config::save_selected_model(provider_id, model.id())
}

fn mark_active(sections: &mut [ModelSelectionSection], provider_id: &str, model_id: &str) {
    for entry in sections
        .iter_mut()
        .flat_map(|section| section.entries.iter_mut())
    {
        entry.is_active = entry.provider_id == provider_id && entry.model_id == model_id;
    }
}

fn render_notification(frame: &mut Frame<'_>, screen_area: Rect, message: &str) {
    let modal_width = (message.len() as u16).saturating_add(4);
    let modal_height = 3;

    let modal_x = screen_area.x + (screen_area.width.saturating_sub(modal_width)) / 2;
    let modal_y = screen_area.y + (screen_area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width.min(screen_area.width),
        height: modal_height,
    };

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Green).fg(Color::Black));

    frame.render_widget(Clear, modal_area);
    frame.render_widget(&modal_block, modal_area);

    let inner_area = modal_block.inner(modal_area);
    let notification_text = Paragraph::new(message)
        .style(Style::default().bg(Color::Green).fg(Color::Black))
        .alignment(Alignment::Center);

    frame.render_widget(notification_text, inner_area);
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

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: id.to_string(),
            description: format!("{id} description"),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: None,
            sha256: None,
            category: None,
        }
    }

    #[test]
    fn cloud_sections_include_only_authenticated_providers() {
        let sections = build_cloud_sections(&["openai".to_string(), "local".to_string()], None);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].provider_id, "openai");
        assert!(sections[0]
            .entries
            .iter()
            .all(|entry| entry.provider_id == "openai"));
    }

    #[test]
    fn grouped_model_data_includes_cloud_and_local_sections() {
        let registry = vec![registry_entry("turbo")];
        let state = LocalModelState::default();

        let sections = build_model_sections(&["groq".to_string()], &state, &registry, None);

        assert_eq!(sections[0].provider_id, "groq");
        assert_eq!(sections[1].provider_id, "local");
        assert!(sections[1]
            .entries
            .iter()
            .any(|entry| entry.model_id == "turbo"));
    }

    #[test]
    fn local_section_marks_downloaded_status_and_management_row() {
        let _guard = test_env_lock();
        let _env = TestEnv::new();
        let registry = vec![registry_entry("turbo"), registry_entry("small")];
        fs::create_dir_all(crate::transcription::local_models::model_files_dir())
            .expect("create model files dir");
        fs::write(model_destination(&registry[0]), b"model").expect("write model file");

        let section = build_local_section(&LocalModelState::default(), &registry, None);

        assert!(matches!(
            section.entries[0].kind,
            ModelSelectionEntryKind::Local {
                is_downloaded: true,
                ..
            }
        ));
        assert!(matches!(
            section.entries[1].kind,
            ModelSelectionEntryKind::Local {
                is_downloaded: false,
                ..
            }
        ));
        assert!(matches!(
            section.entries.last().expect("management row").kind,
            ModelSelectionEntryKind::LocalManagement
        ));
    }

    #[test]
    fn active_selection_is_provider_aware() {
        let selected = SelectedModel {
            provider_id: "local".to_string(),
            model_id: "whisper".to_string(),
        };
        let sections = build_model_sections(
            &["openai".to_string()],
            &LocalModelState::default(),
            &[registry_entry("whisper")],
            Some(&selected),
        );

        let openai_whisper = sections[0]
            .entries
            .iter()
            .find(|entry| entry.model_id == "whisper")
            .expect("openai whisper");
        let local_whisper = sections[1]
            .entries
            .iter()
            .find(|entry| entry.model_id == "whisper")
            .expect("local whisper");

        assert!(!openai_whisper.is_active);
        assert!(local_whisper.is_active);
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
    fn model_wizard_navigation_and_back_quit_are_bounded() {
        let sections = vec![ModelSelectionSection {
            provider_id: "local".to_string(),
            title: "Local".to_string(),
            entries: vec![
                ModelSelectionEntry {
                    provider_id: "local".to_string(),
                    model_id: "turbo".to_string(),
                    name: "Turbo".to_string(),
                    description: String::new(),
                    is_active: false,
                    kind: ModelSelectionEntryKind::Local {
                        entry: registry_entry("turbo"),
                        is_downloaded: false,
                    },
                },
                ModelSelectionEntry {
                    provider_id: "local".to_string(),
                    model_id: "__manage_local_models__".to_string(),
                    name: "Manage local models...".to_string(),
                    description: String::new(),
                    is_active: false,
                    kind: ModelSelectionEntryKind::LocalManagement,
                },
            ],
        }];
        let mut wizard = ModelWizard::new(sections, None);

        wizard.move_up();
        assert_eq!(wizard.selected, 0);
        wizard.move_down();
        wizard.move_down();
        assert_eq!(wizard.selected, 1);
        assert_eq!(wizard.back(), ModelWizardAction::Quit);
    }

    #[test]
    fn selecting_management_row_routes_to_local_management() {
        let mut wizard = ModelWizard::new(
            vec![build_local_section(&LocalModelState::default(), &[], None)],
            None,
        );
        wizard.selected = 0;

        assert_eq!(
            wizard.select_current().expect("select management"),
            ModelWizardAction::ManageLocalModels
        );
    }

    #[test]
    fn missing_local_selection_enters_download_confirmation_without_activation() {
        let _guard = test_env_lock();
        let _env = TestEnv::new();
        let mut wizard = ModelWizard::new(
            vec![build_local_section(
                &LocalModelState::default(),
                &[registry_entry("turbo")],
                None,
            )],
            None,
        );

        assert_eq!(
            wizard.select_current().expect("select missing local"),
            ModelWizardAction::Continue
        );
        assert!(matches!(
            wizard.mode,
            ModelWizardMode::ConfirmDownload { .. }
        ));
    }

    #[test]
    fn selection_save_helpers_persist_provider_aware_state() {
        let _guard = test_env_lock();
        let _env = TestEnv::new();

        save_local_selection("turbo").expect("save local selection");
        let selected = config::get_selected_model_entry()
            .expect("load selection")
            .expect("selected local model");
        assert_eq!(selected.provider_id, "local");
        assert_eq!(selected.model_id, "turbo");

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

    #[test]
    fn downloaded_local_selection_marks_active_after_save() {
        let _guard = test_env_lock();
        let _env = TestEnv::new();
        let registry = vec![registry_entry("turbo")];
        fs::create_dir_all(crate::transcription::local_models::model_files_dir())
            .expect("create model files dir");
        fs::write(model_destination(&registry[0]), b"model").expect("write model file");
        let mut wizard = ModelWizard::new(
            vec![build_local_section(
                &LocalModelState::default(),
                &registry,
                None,
            )],
            None,
        );

        assert_eq!(
            wizard.select_current().expect("select downloaded local"),
            ModelWizardAction::Continue
        );

        let selected = config::get_selected_model_entry()
            .expect("load selection")
            .expect("selected model");
        assert_eq!(selected.provider_id, "local");
        assert_eq!(selected.model_id, "turbo");
        assert!(wizard.sections[0].entries[0].is_active);
    }
}
