use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    delete_model, download_model_with_handle, fetch_registry, load_state,
    mark_downloaded_registry_model, model_destination, register_custom_model, resolve_custom_model,
    validate_downloaded_model, DownloadHandle, LocalModelState, RegistryEntry,
};
use crossterm::event::{self, Event, KeyCode};
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
use std::fs;
use std::io::{self, Stdout};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuiModelEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_mb: u32,
    pub is_downloaded: bool,
    pub is_active: bool,
    pub is_available_in_registry: bool,
    pub languages: Vec<String>,
    pub url: String,
    pub recommended_hardware: Option<String>,
    pub category: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadState {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f64,
    pub speed_mbps: f64,
    pub status: String,
    pub is_complete: bool,
}

#[derive(Clone, Debug)]
pub enum TuiMode {
    Browse,
    CustomModelInput { input: Input },
    CustomModelConfirm { entry: TuiModelEntry },
    Downloading(DownloadState),
    Info { entry: TuiModelEntry },
    ConfirmDelete { entry: TuiModelEntry },
}

#[derive(Clone, Debug)]
pub struct ModelTui {
    pub entries: Vec<TuiModelEntry>,
    pub selected: usize,
    pub mode: TuiMode,
    pub disk_usage_bytes: u64,
    pub status_message: Option<String>,
}

struct RunningDownload {
    entry: RegistryEntry,
    state: Arc<Mutex<DownloadState>>,
    handle: DownloadHandle,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl ModelTui {
    pub fn new(entries: Vec<TuiModelEntry>, disk_usage_bytes: u64) -> Self {
        Self {
            entries,
            selected: 0,
            mode: TuiMode::Browse,
            disk_usage_bytes,
            status_message: None,
        }
    }

    pub fn selected_entry(&self) -> Option<&TuiModelEntry> {
        self.display_entries().get(self.selected).copied()
    }

    pub fn display_entries(&self) -> Vec<&TuiModelEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_downloaded)
            .chain(self.entries.iter().filter(|entry| !entry.is_downloaded))
            .collect()
    }

    pub fn move_selection_down(&mut self) {
        if self.selected + 1 < self.display_entries().len() {
            self.selected += 1;
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn show_info(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            self.mode = TuiMode::Info { entry };
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(entry) = self
            .selected_entry()
            .filter(|entry| entry.is_downloaded)
            .cloned()
        {
            self.mode = TuiMode::ConfirmDelete { entry };
        }
    }

    pub fn back_to_browse(&mut self) {
        self.mode = TuiMode::Browse;
    }

    pub fn show_custom_input(&mut self) {
        self.mode = TuiMode::CustomModelInput {
            input: Input::default(),
        };
    }

    pub fn refresh(
        &mut self,
        local_state: &LocalModelState,
        registry: &[RegistryEntry],
    ) -> anyhow::Result<()> {
        let selected_model = config::get_selected_model_entry()?;
        self.entries = build_model_list(local_state, registry, selected_model.as_ref());
        self.disk_usage_bytes = disk_usage_bytes(&self.entries);
        let display_len = self.display_entries().len();
        if self.selected >= display_len {
            self.selected = display_len.saturating_sub(1);
        }
        Ok(())
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

pub fn build_model_list(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<TuiModelEntry> {
    registry
        .iter()
        .chain(local_state.custom_models.iter())
        .map(|entry| {
            let is_downloaded = model_destination(entry).exists();
            let is_active = selected_model
                .map(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
                .unwrap_or(false);

            TuiModelEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                size_mb: entry.size_mb,
                is_downloaded,
                is_active,
                is_available_in_registry: registry
                    .iter()
                    .any(|registry_entry| registry_entry.id == entry.id),
                languages: entry.languages.clone(),
                url: entry.url.clone(),
                recommended_hardware: entry.recommended_hardware.clone(),
                category: entry.category.clone(),
            }
        })
        .collect()
}

pub fn disk_usage_bytes(entries: &[TuiModelEntry]) -> u64 {
    entries
        .iter()
        .filter(|entry| entry.is_downloaded)
        .filter_map(|entry| {
            let registry_entry = RegistryEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                languages: entry.languages.clone(),
                size_mb: entry.size_mb,
                url: entry.url.clone(),
                recommended_hardware: entry.recommended_hardware.clone(),
                sha256: None,
                category: entry.category.clone(),
            };
            model_destination(&registry_entry).metadata().ok()
        })
        .map(|metadata| metadata.len())
        .sum()
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

fn registry_entry_from_tui(entry: &TuiModelEntry) -> RegistryEntry {
    RegistryEntry {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        languages: entry.languages.clone(),
        size_mb: entry.size_mb,
        url: entry.url.clone(),
        recommended_hardware: entry.recommended_hardware.clone(),
        sha256: None,
        category: entry.category.clone(),
    }
}

fn downloaded_details(entry: &TuiModelEntry) -> (String, String) {
    let path = model_destination(&registry_entry_from_tui(entry));
    let downloaded = fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| format!("{}s since epoch", duration.as_secs()))
        .unwrap_or_else(|| "No".to_string());

    (downloaded, path.display().to_string())
}

fn activate_entry(entry: &TuiModelEntry) -> anyhow::Result<()> {
    if !entry.is_downloaded {
        anyhow::bail!("Download first with [d]");
    }
    let path = model_destination(&registry_entry_from_tui(entry));
    if !path.exists() {
        anyhow::bail!("Download first with [d]");
    }
    config::save_selected_model("local", &entry.id)
}

fn delete_entry(entry: &TuiModelEntry) -> anyhow::Result<()> {
    if delete_model(&entry.id).is_ok() {
        return Ok(());
    }

    let path = model_destination(&registry_entry_from_tui(entry));
    fs::remove_file(&path)?;
    if config::get_selected_model_entry()?
        .is_some_and(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
    {
        config::clear_selected_model()?;
    }
    Ok(())
}

fn tui_entry_from_registry(entry: &RegistryEntry, is_available_in_registry: bool) -> TuiModelEntry {
    TuiModelEntry {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        size_mb: entry.size_mb,
        is_downloaded: model_destination(entry).exists(),
        is_active: false,
        is_available_in_registry,
        languages: entry.languages.clone(),
        url: entry.url.clone(),
        recommended_hardware: entry.recommended_hardware.clone(),
        category: entry.category.clone(),
    }
}

fn initial_download_state(entry: &RegistryEntry) -> DownloadState {
    DownloadState {
        model_id: entry.id.clone(),
        downloaded_bytes: 0,
        total_bytes: u64::from(entry.size_mb) * 1024 * 1024,
        progress: 0.0,
        speed_mbps: 0.0,
        status: "Starting download".to_string(),
        is_complete: false,
    }
}

fn start_download(entry: RegistryEntry, is_custom: bool) -> RunningDownload {
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
            Some(Box::new(
                move |downloaded_bytes, total_bytes, speed_mbps| {
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
                },
            )),
            Some(task_handle),
        )
        .await?;
        validate_downloaded_model(&task_entry)?;
        if is_custom {
            register_custom_model(task_entry)?;
        } else {
            mark_downloaded_registry_model(&task_entry)?;
        }
        Ok(())
    });

    RunningDownload {
        entry,
        state,
        handle,
        task,
    }
}

fn sync_download_mode(tui: &mut ModelTui, running: &RunningDownload) {
    if let Ok(state) = running.state.lock() {
        tui.mode = TuiMode::Downloading(state.clone());
    }
}

fn render_tui(frame: &mut Frame<'_>, tui: &ModelTui) {
    match &tui.mode {
        TuiMode::Browse => render_browse(frame, tui),
        TuiMode::Info { entry } => render_info(frame, entry),
        TuiMode::ConfirmDelete { entry } => render_confirm_delete(frame, entry),
        TuiMode::CustomModelInput { input } => {
            render_custom_input(frame, input, tui.status_message.as_deref())
        }
        TuiMode::CustomModelConfirm { entry } => render_custom_confirm(frame, entry),
        TuiMode::Downloading(state) => render_download(frame, state),
    }
}

fn render_browse(frame: &mut Frame<'_>, tui: &ModelTui) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);
    let active = tui
        .entries
        .iter()
        .find(|entry| entry.is_active)
        .map(|entry| {
            format!(
                "Active model: {} ({})",
                entry.name,
                format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
            )
        })
        .unwrap_or_else(|| "Active model: none".to_string());
    frame.render_widget(
        Paragraph::new(active).block(Block::default().title("OSTT Models").borders(Borders::ALL)),
        chunks[0],
    );

    let mut items = Vec::new();
    items.push(ListItem::new(Line::from(format!(
        "Downloaded models: {} used",
        format_bytes(tui.disk_usage_bytes)
    ))));
    for entry in tui.entries.iter().filter(|entry| entry.is_downloaded) {
        items.push(model_list_item(entry));
    }
    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::from("Available to download:")));
    for entry in tui.entries.iter().filter(|entry| !entry.is_downloaded) {
        items.push(model_list_item(entry));
    }

    let selected_display_index = display_index_for_selection(tui);
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

    let status = tui.status_message.as_deref().unwrap_or(
        "[↑↓] Nav  [Enter] Activate  [d] Download  [r] Remove  [i] Info  [c] Custom  [Esc/q] Quit",
    );
    frame.render_widget(
        Paragraph::new(status).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}

fn model_list_item(entry: &TuiModelEntry) -> ListItem<'_> {
    let marker = if entry.is_active { "◉" } else { "○" };
    let active = if entry.is_active { " (active)" } else { "" };
    let languages = if entry.languages.is_empty() {
        String::new()
    } else {
        format!("     {}", entry.languages.join(", "))
    };
    ListItem::new(Line::from(format!(
        "{marker} {}{active}     {}{languages}",
        entry.name,
        format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
    )))
}

fn display_index_for_selection(tui: &ModelTui) -> Option<usize> {
    let selected_entry = tui.selected_entry()?;
    let mut index = 1;
    for entry in tui.entries.iter().filter(|entry| entry.is_downloaded) {
        if entry.id == selected_entry.id {
            return Some(index);
        }
        index += 1;
    }
    index += 2;
    for entry in tui.entries.iter().filter(|entry| !entry.is_downloaded) {
        if entry.id == selected_entry.id {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn render_info(frame: &mut Frame<'_>, entry: &TuiModelEntry) {
    let area = centered_rect(80, 70, frame.area());
    let (downloaded, path) = downloaded_details(entry);
    let lines = vec![
        Line::from(vec![
            Span::raw("Name: "),
            Span::styled(&entry.name, Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(format!("Model ID: {}", entry.id)),
        Line::from(format!(
            "Size: {}",
            format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
        )),
        Line::from(format!("Downloaded: {downloaded}")),
        Line::from(format!(
            "Active: {}",
            if entry.is_active { "Yes" } else { "No" }
        )),
        Line::from(format!("Path: {path}")),
        Line::from(format!("URL: {}", entry.url)),
        Line::from(format!(
            "Languages: {}",
            if entry.languages.is_empty() {
                "unknown".to_string()
            } else {
                entry.languages.join(", ")
            }
        )),
        Line::from(format!(
            "Recommendation: {}",
            entry.recommended_hardware.as_deref().unwrap_or("none")
        )),
        Line::from(""),
        Line::from("[Esc] Back"),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Model Info").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_confirm_delete(frame: &mut Frame<'_>, entry: &TuiModelEntry) {
    let area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(format!(
            "Delete \"{}\" ({})?\nThis cannot be undone. [y/N]",
            entry.name,
            format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
        ))
        .block(
            Block::default()
                .title("Confirm Delete")
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_custom_input(frame: &mut Frame<'_>, input: &Input, status: Option<&str>) {
    let area = centered_rect(80, 35, frame.area());
    let lines = vec![
        Line::from("Enter model page or file URL:"),
        Line::from(input.value()),
        Line::from(""),
        Line::from(status.unwrap_or("[Enter] Resolve  [Esc] Cancel")),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Download Custom Model")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_custom_confirm(frame: &mut Frame<'_>, entry: &TuiModelEntry) {
    let area = centered_rect(80, 45, frame.area());
    let lines = vec![
        Line::from(format!("Name: {}", entry.name)),
        Line::from(format!("Model ID: {}", entry.id)),
        Line::from(format!(
            "Size: {}",
            format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
        )),
        Line::from(format!("URL: {}", entry.url)),
        Line::from(""),
        Line::from("[Enter] Download  [Esc] Cancel"),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Confirm Custom Model")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
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
    frame.render_widget(
        Paragraph::new(format!("Model: {}", state.model_id)),
        chunks[0],
    );
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
        Paragraph::new(format!("{}  [Tab] Cancel", state.status)),
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

pub async fn handle_models_tui() -> anyhow::Result<()> {
    let local_state = load_state();
    let registry = fetch_registry().await.unwrap_or_default();
    let selected_model = config::get_selected_model_entry()?;
    let entries = build_model_list(&local_state, &registry, selected_model.as_ref());
    let mut tui = ModelTui::new(entries.clone(), disk_usage_bytes(&entries));
    if registry.is_empty() {
        tui.status_message = Some(
            "Could not load remote registry; custom URL entry is still available with [c]"
                .to_string(),
        );
    }
    let mut terminal = TerminalGuard::new()?;
    let mut running_download: Option<RunningDownload> = None;

    loop {
        if let Some(running) = running_download.as_ref() {
            sync_download_mode(&mut tui, running);
            if running.task.is_finished() {
                let running = running_download.take().expect("running download");
                match running.task.await? {
                    Ok(()) => {
                        tui.status_message = Some(format!("Downloaded {}", running.entry.name));
                        tui.back_to_browse();
                        tui.refresh(&load_state(), &registry)?;
                    }
                    Err(error) => {
                        tui.status_message = Some(error.to_string());
                        tui.back_to_browse();
                    }
                }
            }
        }
        terminal.terminal.draw(|frame| render_tui(frame, &tui))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            let mode = tui.mode.clone();
            match (mode, key.code) {
                (TuiMode::Browse, KeyCode::Char('q') | KeyCode::Esc) => break,
                (TuiMode::Browse, KeyCode::Down) => tui.move_selection_down(),
                (TuiMode::Browse, KeyCode::Up) => tui.move_selection_up(),
                (TuiMode::Browse, KeyCode::Enter) => {
                    if let Some(entry) = tui.selected_entry().cloned() {
                        match activate_entry(&entry) {
                            Ok(()) => {
                                tui.status_message = Some(format!("Activated {}", entry.name));
                                tui.refresh(&load_state(), &registry)?;
                            }
                            Err(error) => tui.status_message = Some(error.to_string()),
                        }
                    }
                }
                (TuiMode::Browse, KeyCode::Char('d')) => {
                    if let Some(entry) = tui.selected_entry().cloned() {
                        if entry.is_downloaded {
                            tui.status_message =
                                Some(format!("{} is already downloaded", entry.name));
                        } else if !entry.is_available_in_registry {
                            tui.status_message =
                                Some("Custom models must be downloaded through [c]".to_string());
                        } else {
                            let registry_entry = registry_entry_from_tui(&entry);
                            let running = start_download(registry_entry, false);
                            sync_download_mode(&mut tui, &running);
                            running_download = Some(running);
                        }
                    }
                }
                (TuiMode::Browse, KeyCode::Char('i')) => tui.show_info(),
                (TuiMode::Browse, KeyCode::Char('c')) => {
                    tui.status_message = None;
                    tui.show_custom_input();
                }
                (TuiMode::Browse, KeyCode::Char('r')) => tui.confirm_delete(),
                (TuiMode::Info { .. }, KeyCode::Esc) => tui.back_to_browse(),
                (TuiMode::Downloading(_), KeyCode::Tab) => {
                    if let Some(running) = running_download.as_ref() {
                        running.handle.cancel();
                        if let Ok(mut state) = running.state.lock() {
                            state.status = "Cancelling download".to_string();
                        }
                    }
                }
                (TuiMode::CustomModelInput { input }, KeyCode::Enter) => {
                    let value = input.value().to_string();
                    match resolve_custom_model(&value).await {
                        Ok(entry) => {
                            tui.status_message = None;
                            tui.mode = TuiMode::CustomModelConfirm {
                                entry: tui_entry_from_registry(&entry, false),
                            };
                        }
                        Err(error) => tui.status_message = Some(error.to_string()),
                    }
                }
                (TuiMode::CustomModelInput { input }, _) => {
                    let mut input = input.clone();
                    input.handle_event(&Event::Key(key));
                    tui.mode = TuiMode::CustomModelInput { input };
                }
                (TuiMode::CustomModelConfirm { entry }, KeyCode::Enter) => {
                    let registry_entry = registry_entry_from_tui(&entry);
                    let running = start_download(registry_entry, true);
                    sync_download_mode(&mut tui, &running);
                    running_download = Some(running);
                }
                (
                    TuiMode::ConfirmDelete { .. },
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'),
                ) => tui.back_to_browse(),
                (TuiMode::ConfirmDelete { entry }, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                    let entry = entry.clone();
                    match delete_entry(&entry) {
                        Ok(()) => {
                            tui.status_message = Some(format!("Deleted {}", entry.name));
                            tui.back_to_browse();
                            tui.refresh(&load_state(), &registry)?;
                        }
                        Err(error) => {
                            tui.status_message = Some(error.to_string());
                            tui.back_to_browse();
                        }
                    }
                }
                (_, KeyCode::Esc) => tui.back_to_browse(),
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::local_models::{model_files_dir, set_test_models_dir, TEST_ENV_LOCK};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn with_isolated_models_dir(test: impl FnOnce(PathBuf)) {
        let _guard = test_env_lock();
        let previous_home = std::env::var_os("HOME");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-models-tui-test-{unique}"));
        let models_dir = dir.join("models");
        set_test_models_dir(Some(models_dir.clone()));
        std::env::set_var("HOME", &dir);

        test(models_dir);

        set_test_models_dir(None);
        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(dir);
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: format!("{id} model"),
            description: "Test model".to_string(),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: Some("cpu".to_string()),
            sha256: None,
            category: None,
        }
    }

    #[test]
    fn build_model_list_merges_registry_and_custom_entries() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![RegistryEntry {
                    category: Some("custom".to_string()),
                    ..registry_entry("custom")
                }],
            };

            let entries = build_model_list(&state, &registry, None);

            assert_eq!(entries.len(), 2);
            assert!(entries.iter().any(|entry| {
                entry.id == "turbo" && entry.is_available_in_registry && !entry.is_downloaded
            }));
            assert!(entries.iter().any(|entry| {
                entry.id == "custom"
                    && !entry.is_available_in_registry
                    && entry.category.as_deref() == Some("custom")
            }));
        });
    }

    #[test]
    fn build_model_list_marks_downloaded_and_active_from_filesystem_and_selection() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let selected = SelectedModel {
                provider_id: "local".to_string(),
                model_id: "turbo".to_string(),
            };

            let entries = build_model_list(&LocalModelState::default(), &registry, Some(&selected));

            assert_eq!(entries.len(), 1);
            assert!(entries[0].is_downloaded);
            assert!(entries[0].is_active);
        });
    }

    #[test]
    fn disk_usage_sums_downloaded_model_files_only() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo"), registry_entry("base")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let entries = build_model_list(&LocalModelState::default(), &registry, None);

            assert_eq!(disk_usage_bytes(&entries), 3);
        });
    }

    #[test]
    fn selection_navigation_stays_in_bounds() {
        let entries = vec![
            TuiModelEntry {
                id: "a".to_string(),
                name: "A".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/a.bin".to_string(),
                recommended_hardware: None,
                category: None,
            },
            TuiModelEntry {
                id: "b".to_string(),
                name: "B".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/b.bin".to_string(),
                recommended_hardware: None,
                category: None,
            },
        ];
        let mut tui = ModelTui::new(entries, 0);

        tui.move_selection_up();
        assert_eq!(tui.selected, 0);
        tui.move_selection_down();
        tui.move_selection_down();
        assert_eq!(tui.selected, 1);
    }

    #[test]
    fn selection_navigation_uses_rendered_group_order() {
        let entries = vec![
            TuiModelEntry {
                id: "tiny".to_string(),
                name: "Tiny".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/tiny.bin".to_string(),
                recommended_hardware: None,
                category: None,
            },
            TuiModelEntry {
                id: "turbo".to_string(),
                name: "Turbo".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: true,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/turbo.bin".to_string(),
                recommended_hardware: None,
                category: None,
            },
            TuiModelEntry {
                id: "large-v3".to_string(),
                name: "Large".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/large-v3.bin".to_string(),
                recommended_hardware: None,
                category: None,
            },
        ];
        let mut tui = ModelTui::new(entries, 0);

        assert_eq!(tui.selected_entry().expect("selected").id, "turbo");
        assert_eq!(display_index_for_selection(&tui), Some(1));

        tui.move_selection_down();
        assert_eq!(tui.selected_entry().expect("selected").id, "tiny");
        assert_eq!(display_index_for_selection(&tui), Some(4));

        tui.move_selection_down();
        assert_eq!(tui.selected_entry().expect("selected").id, "large-v3");
        assert_eq!(display_index_for_selection(&tui), Some(5));
    }

    #[test]
    fn info_and_escape_return_to_browse() {
        let entries = vec![TuiModelEntry {
            id: "a".to_string(),
            name: "A".to_string(),
            description: String::new(),
            size_mb: 1,
            is_downloaded: false,
            is_active: false,
            is_available_in_registry: true,
            languages: Vec::new(),
            url: "https://example.com/a.bin".to_string(),
            recommended_hardware: None,
            category: None,
        }];
        let mut tui = ModelTui::new(entries, 0);

        tui.show_info();
        assert!(matches!(tui.mode, TuiMode::Info { .. }));
        tui.back_to_browse();
        assert!(matches!(tui.mode, TuiMode::Browse));
    }

    #[test]
    fn activation_requires_downloaded_file_and_saves_local_selection() {
        with_isolated_models_dir(|_| {
            let mut entry = TuiModelEntry {
                id: "turbo".to_string(),
                name: "Turbo".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: false,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/turbo.bin".to_string(),
                recommended_hardware: None,
                category: None,
            };

            let error = activate_entry(&entry).expect_err("missing download should fail");
            assert!(error.to_string().contains("Download first"));

            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1]).expect("write model");
            entry.is_downloaded = true;
            activate_entry(&entry).expect("activate downloaded model");

            let selected = config::get_selected_model_entry()
                .expect("load selected model")
                .expect("selected model");
            assert_eq!(selected.provider_id, "local");
            assert_eq!(selected.model_id, "turbo");
        });
    }

    #[test]
    fn confirmed_delete_removes_file_and_clears_active_selection() {
        with_isolated_models_dir(|_| {
            let entry = TuiModelEntry {
                id: "turbo".to_string(),
                name: "Turbo".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: true,
                is_active: true,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/turbo.bin".to_string(),
                recommended_hardware: None,
                category: None,
            };
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            let path = model_files_dir().join("turbo.bin");
            fs::write(&path, [1]).expect("write model");
            config::save_selected_model("local", "turbo").expect("save selected model");

            delete_entry(&entry).expect("delete model");

            assert!(!path.exists());
            assert!(config::get_selected_model_entry()
                .expect("load selected model")
                .is_none());
        });
    }
}
