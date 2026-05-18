use crate::commands::models_tui;
use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    download_model_with_handle, fetch_registry, load_state, mark_downloaded_registry_model,
    model_destination, validate_downloaded_model, DownloadHandle, LocalModelState, RegistryEntry,
};
use crate::transcription::{TranscriptionModel, TranscriptionProvider};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::HashSet;
use std::io::{self, Stdout};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ModelSelectionEntryKind {
    Cloud { model: TranscriptionModel },
    Local { entry: RegistryEntry, is_downloaded: bool },
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

#[derive(Debug, Clone)]
pub struct ModelWizard {
    pub sections: Vec<ModelSelectionSection>,
    pub selected: usize,
    pub mode: ModelWizardMode,
    pub status_message: Option<String>,
    pub local_audio_warning: Option<String>,
}

pub async fn handle_model() -> anyhow::Result<()> {
    loop {
        let authorized_provider_ids = config::get_authorized_providers()?;
        let local_state = load_local_selection_state()?;
        let registry = fetch_registry().await.unwrap_or_default();
        let selected_model = config::get_selected_model_entry()?;
        let sections = build_model_sections(
            &authorized_provider_ids,
            &local_state,
            &registry,
            selected_model.as_ref(),
        );
        let mut wizard = ModelWizard::new(sections, local_audio_warning()?);
        if registry.is_empty() {
            wizard.status_message = Some("Could not load remote Local registry".to_string());
        }

        let action = run_model_wizard(&mut wizard).await?;
        if action == ModelWizardAction::ManageLocalModels {
            models_tui::handle_models_tui().await?;
            continue;
        }
        break;
    }
    Ok(())
}

impl ModelWizard {
    pub fn new(sections: Vec<ModelSelectionSection>, local_audio_warning: Option<String>) -> Self {
        Self {
            sections,
            selected: 0,
            mode: ModelWizardMode::Browse,
            status_message: None,
            local_audio_warning,
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
                self.status_message = Some(format!("Activated {}", entry.name));
                mark_active(&mut self.sections, &entry.provider_id, &entry.model_id);
                Ok(ModelWizardAction::Continue)
            }
            ModelSelectionEntryKind::Local { is_downloaded, .. } if *is_downloaded => {
                save_local_selection(&entry.model_id)?;
                self.status_message = Some(format!("Activated {}", entry.name));
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

struct RunningDownload {
    entry: RegistryEntry,
    state: Arc<Mutex<DownloadState>>,
    handle: DownloadHandle,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

async fn run_model_wizard(wizard: &mut ModelWizard) -> anyhow::Result<ModelWizardAction> {
    let mut terminal = TerminalGuard::new()?;
    let mut running_download: Option<RunningDownload> = None;

    loop {
        if let Some(running) = running_download.as_ref() {
            sync_download_mode(wizard, running);
            if running.task.is_finished() {
                let running = running_download.take().expect("running download");
                match running.task.await? {
                    Ok(()) => {
                        wizard.status_message = Some(format!(
                            "Downloaded {}. Press Enter again to activate.",
                            running.entry.name
                        ));
                        update_local_downloaded_status(wizard, &running.entry.id, true);
                    }
                    Err(error) => wizard.status_message = Some(error.to_string()),
                }
                wizard.mode = ModelWizardMode::Browse;
            }
        }

        terminal.terminal.draw(|frame| render_model_wizard(frame, wizard))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if let Some(running) = running_download.as_ref() {
                    running.handle.cancel();
                    if let Ok(mut state) = running.state.lock() {
                        state.status = "Cancelling download".to_string();
                    }
                    continue;
                }
                return Ok(ModelWizardAction::Quit);
            }

            match (wizard.mode.clone(), key.code) {
                (ModelWizardMode::Browse, KeyCode::Char('q')) => return Ok(ModelWizardAction::Quit),
                (ModelWizardMode::Browse, KeyCode::Esc) => return Ok(wizard.back()),
                (ModelWizardMode::Browse, KeyCode::Down) => wizard.move_down(),
                (ModelWizardMode::Browse, KeyCode::Up) => wizard.move_up(),
                (ModelWizardMode::Browse, KeyCode::Char('m')) => {
                    return Ok(ModelWizardAction::ManageLocalModels)
                }
                (ModelWizardMode::Browse, KeyCode::Enter) => match wizard.select_current()? {
                    ModelWizardAction::ManageLocalModels => {
                        return Ok(ModelWizardAction::ManageLocalModels)
                    }
                    action => {
                        if action != ModelWizardAction::Continue {
                            return Ok(action);
                        }
                    }
                },
                (ModelWizardMode::ConfirmDownload { .. }, KeyCode::Esc) => {
                    wizard.mode = ModelWizardMode::Browse
                }
                (ModelWizardMode::ConfirmDownload { entry }, KeyCode::Enter) => {
                    if let ModelSelectionEntryKind::Local { entry, .. } = entry.kind {
                        let running = start_download(entry);
                        sync_download_mode(wizard, &running);
                        running_download = Some(running);
                    }
                }
                (ModelWizardMode::Downloading(_), KeyCode::Esc | KeyCode::Char('q')) => {}
                _ => {}
            }
        }
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
                    .map(|model| {
                        let model_id = model.id().to_string();
                        ModelSelectionEntry {
                            provider_id: provider.id().to_string(),
                            model_id: model_id.clone(),
                            name: model_id,
                            description: model.description().to_string(),
                            is_active: selected_model
                                .map(|selected| {
                                    selected.provider_id == provider.id() && selected.model_id == model.id()
                                })
                                .unwrap_or(false),
                            kind: ModelSelectionEntryKind::Cloud { model },
                        }
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
                    .map(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
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

pub fn save_cloud_selection(
    provider_id: &str,
    model: &TranscriptionModel,
) -> anyhow::Result<()> {
    config::save_selected_model(provider_id, model.id())
}

fn mark_active(sections: &mut [ModelSelectionSection], provider_id: &str, model_id: &str) {
    for entry in sections.iter_mut().flat_map(|section| section.entries.iter_mut()) {
        entry.is_active = entry.provider_id == provider_id && entry.model_id == model_id;
    }
}

fn update_local_downloaded_status(wizard: &mut ModelWizard, model_id: &str, is_downloaded: bool) {
    for entry in wizard.sections.iter_mut().flat_map(|section| section.entries.iter_mut()) {
        if entry.provider_id == "local" && entry.model_id == model_id {
            if let ModelSelectionEntryKind::Local {
                is_downloaded: downloaded,
                ..
            } = &mut entry.kind
            {
                *downloaded = is_downloaded;
            }
        }
    }
}

pub fn local_audio_warning() -> anyhow::Result<Option<String>> {
    let config = config::OsttConfig::load().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let output = config.audio.output_format.trim();
    if output == "pcm_s16le -ar 16000" && config.audio.sample_rate == 16000 {
        return Ok(None);
    }
    Ok(Some(
        "Local transcription works best with audio.output_format = \"pcm_s16le -ar 16000\" and sample_rate = 16000."
            .to_string(),
    ))
}

fn initial_download_state(entry: &RegistryEntry) -> DownloadState {
    DownloadState {
        model_id: entry.id.clone(),
        downloaded_bytes: 0,
        total_bytes: u64::from(entry.size_mb) * 1024 * 1024,
        progress: 0.0,
        speed_mbps: 0.0,
        status: "Starting download".to_string(),
    }
}

fn start_download(entry: RegistryEntry) -> RunningDownload {
    let state = Arc::new(Mutex::new(initial_download_state(&entry)));
    let progress_state = state.clone();
    let handle = DownloadHandle::new();
    let task_handle = handle.clone();
    let task_entry = entry.clone();
    let task = tokio::spawn(async move {
        let destination = model_destination(&task_entry);
        download_model_with_handle(
            &task_entry.url,
            &destination,
            Some(Box::new(move |downloaded_bytes, total_bytes, speed_mbps| {
                if let Ok(mut state) = progress_state.lock() {
                    state.downloaded_bytes = downloaded_bytes;
                    state.total_bytes = total_bytes;
                    state.speed_mbps = speed_mbps;
                    state.progress = if total_bytes > 0 {
                        downloaded_bytes as f64 / total_bytes as f64
                    } else {
                        0.0
                    };
                    state.status = "Downloading".to_string();
                }
            })),
            Some(task_handle),
        )
        .await?;
        validate_downloaded_model(&task_entry)?;
        mark_downloaded_registry_model(&task_entry)?;
        Ok(())
    });

    RunningDownload {
        entry,
        state,
        handle,
        task,
    }
}

fn sync_download_mode(wizard: &mut ModelWizard, running: &RunningDownload) {
    if let Ok(state) = running.state.lock() {
        wizard.mode = ModelWizardMode::Downloading(state.clone());
    }
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

fn render_model_wizard(frame: &mut Frame<'_>, wizard: &ModelWizard) {
    match &wizard.mode {
        ModelWizardMode::Browse => render_browse(frame, wizard),
        ModelWizardMode::ConfirmDownload { entry } => render_confirm_download(frame, entry),
        ModelWizardMode::Downloading(state) => render_download(frame, state),
    }
}

fn render_browse(frame: &mut Frame<'_>, wizard: &ModelWizard) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(4),
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new("Choose transcription model")
            .block(Block::default().title("OSTT Model").borders(Borders::ALL)),
        chunks[0],
    );

    let mut items = Vec::new();
    let mut display_index = 0_usize;
    let mut selected_display_index = None;
    for section in &wizard.sections {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            section.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )])));
        display_index += 1;
        for entry in &section.entries {
            if wizard.selected_entry().is_some_and(|selected| {
                selected.provider_id == entry.provider_id && selected.model_id == entry.model_id
            }) {
                selected_display_index = Some(display_index);
            }
            items.push(model_list_item(entry));
            display_index += 1;
        }
        items.push(ListItem::new(Line::from("")));
        display_index += 1;
    }

    let mut state = ListState::default().with_selected(selected_display_index);
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> "),
        chunks[1],
        &mut state,
    );

    let mut footer = wizard.status_message.clone().unwrap_or_else(|| {
        "[↑↓] Navigate  [Enter] Select  [m] Manage local  [Esc/q] Quit".to_string()
    });
    if let Some(warning) = &wizard.local_audio_warning {
        footer.push('\n');
        footer.push_str(warning);
    }
    frame.render_widget(
        Paragraph::new(footer)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        chunks[2],
    );
}

fn model_list_item(entry: &ModelSelectionEntry) -> ListItem<'_> {
    let marker = if entry.is_active { "◉" } else { "○" };
    let suffix = match &entry.kind {
        ModelSelectionEntryKind::Local {
            entry: registry_entry,
            is_downloaded,
        } => {
            let status = if *is_downloaded { "downloaded" } else { "available" };
            format!(
                " ({}) - {} [{}]",
                format_bytes(u64::from(registry_entry.size_mb) * 1024 * 1024),
                entry.description,
                status
            )
        }
        ModelSelectionEntryKind::LocalManagement => format!(" - {}", entry.description),
        ModelSelectionEntryKind::Cloud { .. } => format!(" - {}", entry.description),
    };
    ListItem::new(Line::from(format!("  {marker} {}{suffix}", entry.name)))
}

fn render_confirm_download(frame: &mut Frame<'_>, entry: &ModelSelectionEntry) {
    let area = centered_rect(70, 30, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(format!(
            "Download {} before activation?\n\n[Enter] Download  [Esc] Back",
            entry.name
        ))
        .block(Block::default().title("Download Local Model").borders(Borders::ALL)),
        area,
    );
}

fn render_download(frame: &mut Frame<'_>, state: &DownloadState) {
    let area = centered_rect(80, 40, frame.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);
    let eta = if state.speed_mbps > 0.0 && state.total_bytes > state.downloaded_bytes {
        let remaining_mb = (state.total_bytes - state.downloaded_bytes) as f64 / (1024.0 * 1024.0);
        format!("ETA: {:.0}s", remaining_mb / state.speed_mbps)
    } else {
        "ETA: unknown".to_string()
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().title("Downloading").borders(Borders::ALL),
        area,
    );
    frame.render_widget(Paragraph::new(format!("Model: {}", state.model_id)), chunks[0]);
    frame.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(state.progress.clamp(0.0, 1.0)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{} / {}  •  {:.1} MB/s  •  {}",
            format_bytes(state.downloaded_bytes),
            if state.total_bytes == 0 {
                "unknown".to_string()
            } else {
                format_bytes(state.total_bytes)
            },
            state.speed_mbps,
            eta
        )),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(format!("{}  [Ctrl+C] Cancel", state.status)),
        chunks[3],
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
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

    let provider = TranscriptionProvider::from_id(&selected.provider_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown transcription provider: {}", selected.provider_id))?;
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
        previous_models_dir: Option<std::ffi::OsString>,
        dir: std::path::PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let previous_home = std::env::var_os("HOME");
            let previous_models_dir = std::env::var_os("OSTT_MODELS_DIR");
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("ostt-model-command-test-{unique}"));
            fs::create_dir_all(&dir).expect("create temp dir");
            std::env::set_var("HOME", &dir);
            std::env::set_var("OSTT_MODELS_DIR", dir.join("models"));

            Self {
                previous_home,
                previous_models_dir,
                dir,
            }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            if let Some(previous_home) = self.previous_home.take() {
                std::env::set_var("HOME", previous_home);
            } else {
                std::env::remove_var("HOME");
            }

            if let Some(previous_models_dir) = self.previous_models_dir.take() {
                std::env::set_var("OSTT_MODELS_DIR", previous_models_dir);
            } else {
                std::env::remove_var("OSTT_MODELS_DIR");
            }

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
        assert!(missing_cloud_credentials_error(&TranscriptionProvider::OpenAI)
            .to_string()
            .contains("ostt auth login"));
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
        let mut wizard = ModelWizard::new(vec![build_local_section(
            &LocalModelState::default(),
            &[],
            None,
        )], None);
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
        let mut wizard = ModelWizard::new(vec![build_local_section(
            &LocalModelState::default(),
            &[registry_entry("turbo")],
            None,
        )], None);

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

        save_cloud_selection("openai", &TranscriptionModel::Whisper)
            .expect("save cloud selection");
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
        let mut wizard = ModelWizard::new(vec![build_local_section(
            &LocalModelState::default(),
            &registry,
            None,
        )], None);

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
