use crate::commands::model::UserQuit;
use crate::config::{self, SelectedModel};
use crate::transcription::local_models::{
    delete_model, download_model_with_handle, fetch_registry, is_safe_model_id, load_state,
    mark_downloaded_registry_model, model_destination, register_downloaded_custom_model,
    resolve_custom_model, validate_custom_model_registration, validate_downloaded_model,
    DownloadHandle, LocalModelState, RegistryEntry,
};
use crate::ui::{
    centered_fixed_rect, render_dialog, render_error_dialog, render_toast, DialogAction, Toast,
    ToastStyle,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
use ratatui::{Frame, Terminal};
use std::fs;
use std::io::{self, Stdout};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(255, 255, 255);
const HIGHLIGHT_BG: Color = Color::Rgb(20, 20, 20);
const HELP_FG: Color = Color::Rgb(100, 100, 100);
const SECTION_FG: Color = Color::Rgb(120, 120, 120);
const LOGO: &str = " ┏┓┏╋╋ \n ┗┛┛┗┗ \n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalModelEntry {
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
    pub sha256: Option<String>,
    pub group_id: Option<String>,
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
pub enum LocalModelsMode {
    Browse,
    CustomModelInput {
        input: Input,
        selected_action: DialogAction,
    },
    CustomModelDetails {
        resolved_entry: RegistryEntry,
        id_input: Input,
        name_input: Input,
        focus: CustomModelDetailsFocus,
        selected_action: DialogAction,
    },
    ConfirmDownload {
        entry: LocalModelEntry,
        selected_action: DialogAction,
    },
    Downloading(DownloadState),
    Info {
        entry: LocalModelEntry,
    },
    ConfirmDelete {
        entry: LocalModelEntry,
        selected_action: DialogAction,
    },
    ErrorDialog {
        message: String,
        return_mode: Box<LocalModelsMode>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomModelDetailsFocus {
    Id,
    Name,
}

#[derive(Clone, Debug)]
pub struct LocalModelsTui {
    pub entries: Vec<LocalModelEntry>,
    pub selected: usize,
    pub mode: LocalModelsMode,
    pub downloaded_model_disk_usage_bytes: u64,
    pub toast: Option<Toast>,
}

struct RunningDownload {
    state: Arc<Mutex<DownloadState>>,
    handle: DownloadHandle,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl LocalModelsTui {
    pub fn new(entries: Vec<LocalModelEntry>, downloaded_model_disk_usage_bytes: u64) -> Self {
        Self {
            entries,
            selected: 0,
            mode: LocalModelsMode::Browse,
            downloaded_model_disk_usage_bytes,
            toast: None,
        }
    }

    pub fn selected_entry(&self) -> Option<&LocalModelEntry> {
        self.display_entries().get(self.selected).copied()
    }

    pub fn display_entries(&self) -> Vec<&LocalModelEntry> {
        self.entries.iter().collect()
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
            self.mode = LocalModelsMode::Info { entry };
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(entry) = self
            .selected_entry()
            .filter(|entry| entry.is_downloaded)
            .cloned()
        {
            self.mode = LocalModelsMode::ConfirmDelete {
                entry,
                selected_action: DialogAction::Cancel,
            };
        }
    }

    pub fn confirm_download(&mut self) {
        if let Some(entry) = self
            .selected_entry()
            .filter(|entry| !entry.is_downloaded)
            .cloned()
        {
            self.mode = LocalModelsMode::ConfirmDownload {
                entry,
                selected_action: DialogAction::Ok,
            };
        }
    }

    pub fn select_dialog_action(&mut self, action: DialogAction) {
        self.mode = match self.mode.clone() {
            LocalModelsMode::ConfirmDelete { entry, .. } => LocalModelsMode::ConfirmDelete {
                entry,
                selected_action: action,
            },
            LocalModelsMode::ConfirmDownload { entry, .. } => LocalModelsMode::ConfirmDownload {
                entry,
                selected_action: action,
            },
            LocalModelsMode::CustomModelInput { input, .. } => LocalModelsMode::CustomModelInput {
                input,
                selected_action: action,
            },
            LocalModelsMode::CustomModelDetails {
                resolved_entry,
                id_input,
                name_input,
                focus,
                ..
            } => LocalModelsMode::CustomModelDetails {
                resolved_entry,
                id_input,
                name_input,
                focus,
                selected_action: action,
            },
            mode => mode,
        };
    }

    pub fn toggle_dialog_action(&mut self) {
        let selected_action = match self.dialog_action() {
            Some(DialogAction::Ok) => DialogAction::Cancel,
            Some(DialogAction::Cancel) => DialogAction::Ok,
            None => return,
        };
        self.select_dialog_action(selected_action);
    }

    pub fn dialog_action(&self) -> Option<DialogAction> {
        match &self.mode {
            LocalModelsMode::ConfirmDelete {
                selected_action, ..
            }
            | LocalModelsMode::ConfirmDownload {
                selected_action, ..
            }
            | LocalModelsMode::CustomModelInput {
                selected_action, ..
            }
            | LocalModelsMode::CustomModelDetails {
                selected_action, ..
            } => Some(*selected_action),
            _ => None,
        }
    }

    pub fn show_error_dialog(&mut self, message: String) {
        self.mode = LocalModelsMode::ErrorDialog {
            message,
            return_mode: Box::new(self.mode.clone()),
        };
    }

    pub fn close_error_dialog(&mut self) {
        if let LocalModelsMode::ErrorDialog { return_mode, .. } = self.mode.clone() {
            self.mode = *return_mode;
        }
    }

    pub fn back_to_browse(&mut self) {
        self.mode = LocalModelsMode::Browse;
    }

    pub fn show_custom_input(&mut self) {
        self.mode = LocalModelsMode::CustomModelInput {
            input: Input::default(),
            selected_action: DialogAction::Ok,
        };
    }

    pub fn refresh(
        &mut self,
        local_state: &LocalModelState,
        registry: &[RegistryEntry],
    ) -> anyhow::Result<()> {
        let selected_model = config::get_selected_model_entry()?;
        self.entries = build_local_model_entries(local_state, registry, selected_model.as_ref());
        self.downloaded_model_disk_usage_bytes = downloaded_model_disk_usage_bytes(&self.entries);
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

pub fn build_local_model_entries(
    local_state: &LocalModelState,
    registry: &[RegistryEntry],
    selected_model: Option<&SelectedModel>,
) -> Vec<LocalModelEntry> {
    registry
        .iter()
        .chain(local_state.custom_models.iter())
        .map(|entry| {
            let is_downloaded = model_destination(entry).exists();
            let is_active = selected_model
                .map(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
                .unwrap_or(false);

            LocalModelEntry {
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
                sha256: entry.sha256.clone(),
                group_id: entry.group_id.clone(),
            }
        })
        .collect()
}

pub fn downloaded_model_disk_usage_bytes(entries: &[LocalModelEntry]) -> u64 {
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
                group_id: entry.group_id.clone(),
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

fn registry_entry_from_model(entry: &LocalModelEntry) -> RegistryEntry {
    RegistryEntry {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        languages: entry.languages.clone(),
        size_mb: entry.size_mb,
        url: entry.url.clone(),
        recommended_hardware: entry.recommended_hardware.clone(),
        sha256: entry.sha256.clone(),
        category: entry.category.clone(),
        group_id: entry.group_id.clone(),
    }
}

fn downloaded_details(entry: &LocalModelEntry) -> (String, String) {
    let path = model_destination(&registry_entry_from_model(entry));
    let downloaded = fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| format!("{}s since epoch", duration.as_secs()))
        .unwrap_or_else(|| "No".to_string());

    (downloaded, path.display().to_string())
}

fn activate_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if !entry.is_downloaded {
        anyhow::bail!("Download first with [d]");
    }
    let path = model_destination(&registry_entry_from_model(entry));
    if !path.exists() {
        anyhow::bail!("Download first with [d]");
    }
    config::save_selected_model("local", &entry.id)
}

fn delete_entry(entry: &LocalModelEntry) -> anyhow::Result<()> {
    if delete_model(&entry.id).is_ok() {
        return Ok(());
    }

    let path = model_destination(&registry_entry_from_model(entry));
    fs::remove_file(&path)?;
    if config::get_selected_model_entry()?
        .is_some_and(|selected| selected.provider_id == "local" && selected.model_id == entry.id)
    {
        config::clear_selected_model()?;
    }
    Ok(())
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
            register_downloaded_custom_model(task_entry)?;
        } else {
            mark_downloaded_registry_model(&task_entry)?;
        }
        Ok(())
    });

    RunningDownload {
        state,
        handle,
        task,
    }
}

fn sync_download_progress(tui: &mut LocalModelsTui, running: &RunningDownload) {
    if let Ok(state) = running.state.lock() {
        tui.mode = LocalModelsMode::Downloading(state.clone());
    }
}

fn render_local_models(frame: &mut Frame<'_>, tui: &LocalModelsTui) {
    let area = frame.area();

    let padding_block = Block::default()
        .padding(Padding::uniform(1))
        .style(Style::default().bg(BG));
    frame.render_widget(&padding_block, area);
    let padded_area = padding_block.inner(area);

    let main_block = Block::default().style(Style::default().fg(FG).bg(BG));
    frame.render_widget(&main_block, padded_area);
    let inner_area = main_block.inner(padded_area);

    match &tui.mode {
        LocalModelsMode::Browse => render_browse(frame, inner_area, tui),
        LocalModelsMode::Info { entry } => {
            render_info(frame, inner_area, entry);
        }
        LocalModelsMode::ConfirmDelete {
            entry,
            selected_action,
        } => {
            render_browse(frame, inner_area, tui);
            render_confirm_delete(frame, entry, *selected_action);
        }
        LocalModelsMode::ConfirmDownload {
            entry,
            selected_action,
        } => {
            render_browse(frame, inner_area, tui);
            render_confirm_download(frame, entry, *selected_action);
        }
        LocalModelsMode::CustomModelInput {
            input,
            selected_action,
        } => {
            render_logo(frame, inner_area);
            render_custom_input(frame, input, *selected_action);
        }
        LocalModelsMode::CustomModelDetails {
            id_input,
            name_input,
            focus,
            selected_action,
            ..
        } => {
            render_logo(frame, inner_area);
            render_custom_details(frame, id_input, name_input, *focus, *selected_action);
        }
        LocalModelsMode::Downloading(state) => {
            render_browse(frame, inner_area, tui);
            render_download(frame, state);
        }
        LocalModelsMode::ErrorDialog {
            message,
            return_mode,
        } => {
            match return_mode.as_ref() {
                LocalModelsMode::CustomModelInput {
                    input,
                    selected_action,
                } => {
                    render_logo(frame, inner_area);
                    render_custom_input(frame, input, *selected_action);
                }
                _ => render_browse(frame, inner_area, tui),
            }
            render_error_dialog(frame, "Error", message.clone());
        }
    }

    if let Some(toast) = &tui.toast {
        render_toast(frame, toast, ToastStyle::default());
    }
}

fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn section_header(label: impl Into<String>) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        label.into(),
        Style::default().fg(SECTION_FG),
    )))
}

fn render_logo(frame: &mut Frame<'_>, area: Rect) {
    let [header_area, _] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
    let header = Paragraph::new(LOGO)
        .style(Style::default().fg(FG))
        .alignment(Alignment::Left);
    frame.render_widget(header, header_area);
}

fn render_browse(frame: &mut Frame<'_>, inner_area: Rect, tui: &LocalModelsTui) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .areas(inner_area);

    let header = Paragraph::new(LOGO)
        .style(Style::default().fg(FG))
        .alignment(Alignment::Left);
    frame.render_widget(header, header_area);

    let mut items = Vec::new();
    push_grouped_model_items(&mut items, tui.entries.iter().collect());

    let selected_display_index = display_index_for_selected_model(tui);
    let mut state = ListState::default().with_selected(selected_display_index);
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Local Models ")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().bg(HIGHLIGHT_BG).fg(FG))
            .highlight_symbol("> "),
        list_area,
        &mut state,
    );

    let footer_text = "↑↓ nav, ↵ activate/download, d delete, i info, c custom, esc/q back";
    frame.render_widget(
        Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(HELP_FG)),
        footer_area,
    );
}

fn push_grouped_model_items(
    items: &mut Vec<ListItem<'static>>,
    entries: Vec<&LocalModelEntry>,
) {
    let mut current_group: Option<&str> = None;
    for entry in entries {
        let group = entry.group_id.as_deref().unwrap_or("Custom");
        if current_group != Some(group) {
            if current_group.is_some() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(section_header(group.to_string()));
            current_group = Some(group);
        }
        items.push(local_model_list_item(entry));
    }
}

fn local_model_list_item(entry: &LocalModelEntry) -> ListItem<'static> {
    let active_marker = if entry.is_active { "◉" } else { "○" };
    let downloaded_marker = if entry.is_downloaded { "✓" } else { " " };
    let size = format_bytes(u64::from(entry.size_mb) * 1024 * 1024);
    let description = entry.description.trim();

    ListItem::new(Line::from(format!(
        "{} {} {}, {}, {}",
        active_marker, downloaded_marker, entry.name, size, description,
    )))
}

fn display_index_for_selected_model(tui: &LocalModelsTui) -> Option<usize> {
    let selected_entry_id = tui.selected_entry().map(|entry| entry.id.as_str())?;
    grouped_display_index(tui.entries.iter().collect(), selected_entry_id, 0)
}

fn grouped_display_index(
    entries: Vec<&LocalModelEntry>,
    selected_entry_id: &str,
    start_index: usize,
) -> Option<usize> {
    let mut index = start_index;
    let mut current_group: Option<&str> = None;
    for entry in entries {
        let group = entry.group_id.as_deref().unwrap_or("Custom");
        if current_group != Some(group) {
            if current_group.is_some() {
                index += 1;
            }
            index += 1;
            current_group = Some(group);
        }
        if entry.id == selected_entry_id {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn render_info(frame: &mut Frame<'_>, inner_area: Rect, entry: &LocalModelEntry) {
    let [header_area, content_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .areas(inner_area);
    frame.render_widget(
        Paragraph::new(LOGO)
            .style(Style::default().fg(FG))
            .alignment(Alignment::Left),
        header_area,
    );

    let (_, path) = downloaded_details(entry);
    let mut lines = vec![
        Line::from(format!("ID: {}", entry.id)),
        Line::from(""),
        Line::from(entry.description.clone()),
        Line::from(""),
        Line::from(format!(
            "Recommended hardware: {}",
            entry.recommended_hardware.as_deref().unwrap_or("none")
        )),
        Line::from(""),
        Line::from(format!("Size (MB): {}", entry.size_mb)),
        Line::from(format!(
            "Languages: {}",
            if entry.languages.is_empty() {
                "unknown".to_string()
            } else {
                entry.languages.join(", ")
            }
        )),
        Line::from(format!("Url: {}", entry.url)),
        Line::from(format!(
            "Downloaded: {}",
            if entry.is_downloaded { "Yes" } else { "No" }
        )),
        Line::from(format!(
            "Active: {}",
            if entry.is_active { "Yes" } else { "No" }
        )),
    ];
    if entry.is_downloaded {
        lines.push(Line::from(format!("Local path: {path}")));
    }
    if let Some(sha256) = &entry.sha256 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("SHA256: {sha256}")));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(entry.name.as_str())
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        content_area,
    );
    frame.render_widget(
        Paragraph::new("esc/q back")
            .alignment(Alignment::Center)
            .style(Style::default().fg(HELP_FG)),
        footer_area,
    );
}

fn render_confirm_delete(
    frame: &mut Frame<'_>,
    entry: &LocalModelEntry,
    selected_action: DialogAction,
) {
    render_dialog(
        frame,
        "Confirm Delete",
        vec![
            Line::from(format!(
                "Delete \"{}\" ({})?",
                entry.name,
                format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
            )),
            Line::from(""),
            Line::from("This cannot be undone."),
        ],
        selected_action,
    );
}

fn render_confirm_download(
    frame: &mut Frame<'_>,
    entry: &LocalModelEntry,
    selected_action: DialogAction,
) {
    render_dialog(
        frame,
        "Start Download",
        vec![
            Line::from(format!("Download \"{}\"?", entry.name)),
            Line::from(""),
            Line::from(format!(
                "Size: {}",
                format_bytes(u64::from(entry.size_mb) * 1024 * 1024)
            )),
        ],
        selected_action,
    );
}

fn render_custom_input(frame: &mut Frame<'_>, input: &Input, selected_action: DialogAction) {
    let area = centered_fixed_rect(70, 12, frame.area());
    let lines = vec![
        padded_line("Paste a Hugging Face model page or a direct model file URL."),
        padded_line("Supported files: .gguf and ggml-*.bin."),
        Line::from(""),
        padded_line("Enter model page or file URL:"),
        input_line(input.value(), true),
        Line::from(""),
        wizard_buttons(selected_action),
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

fn render_custom_details(
    frame: &mut Frame<'_>,
    id_input: &Input,
    name_input: &Input,
    focus: CustomModelDetailsFocus,
    selected_action: DialogAction,
) {
    let area = centered_fixed_rect(70, 14, frame.area());
    let lines = vec![
        padded_line("Choose how this custom model should appear in OSTT."),
        Line::from(""),
        padded_line("ID:"),
        input_line(id_input.value(), focus == CustomModelDetailsFocus::Id),
        Line::from(""),
        padded_line("Name:"),
        input_line(name_input.value(), focus == CustomModelDetailsFocus::Name),
        Line::from(""),
        wizard_buttons(selected_action),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Custom Model Details")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn padded_line(text: impl Into<String>) -> Line<'static> {
    Line::from(format!(" {}", text.into()))
}

fn input_line(value: &str, focused: bool) -> Line<'static> {
    let cursor = if focused { "█" } else { "" };
    let text = format!("{value}{cursor}");
    let padded = format!(" {:<66}", text);
    Line::from(Span::styled(padded, Style::default().bg(HIGHLIGHT_BG)))
}

fn wizard_buttons(selected_action: DialogAction) -> Line<'static> {
    let button_style = |action| {
        if selected_action == action {
            Style::default().fg(BG).bg(FG).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        }
    };
    Line::from(vec![
        Span::raw("                                                "),
        Span::styled("<Cancel>", button_style(DialogAction::Cancel)),
        Span::raw("  "),
        Span::styled("<Next>", button_style(DialogAction::Ok)),
    ])
}

fn render_download(frame: &mut Frame<'_>, state: &DownloadState) {
    let area = centered_fixed_rect(70, 10, frame.area());
    let eta = if state.speed_mbps > 0.0 && state.total_bytes > state.downloaded_bytes {
        let remaining_mb = (state.total_bytes - state.downloaded_bytes) as f64 / (1024.0 * 1024.0);
        format!("ETA: {:.0}s", remaining_mb / state.speed_mbps)
    } else {
        "ETA: unknown".to_string()
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Model: {}", state.model_id)),
            Line::from(""),
            Line::from(progress_bar(state.progress, 58)),
            Line::from(""),
            Line::from(format!(
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
            Line::from(""),
            Line::from(Span::styled(
                "<Cancel>",
                Style::default().fg(BG).bg(FG).add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .title(state.status.as_str())
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false }),
        area,
    );
}

fn progress_bar(progress: f64, width: usize) -> String {
    let progress = progress.clamp(0.0, 1.0);
    let completed = (progress * width as f64).round() as usize;
    let remaining = width.saturating_sub(completed);
    format!("{}{}", "█".repeat(completed), "░".repeat(remaining))
}

async fn finish_completed_download(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    running_download: &mut Option<RunningDownload>,
) -> anyhow::Result<()> {
    let Some(running) = running_download.as_ref() else {
        return Ok(());
    };

    sync_download_progress(tui, running);
    if !running.task.is_finished() {
        return Ok(());
    }

    let running = running_download.take().expect("running download");
    match running.task.await? {
        Ok(()) => {
            tui.back_to_browse();
            tui.refresh(&load_state(), registry)?;
            tui.toast = Some(Toast::new("Download complete"));
        }
        Err(error) => {
            if error.to_string() != "model download cancelled" {
                tui.back_to_browse();
                tui.show_error_dialog(error.to_string());
            } else {
                tui.back_to_browse();
            }
        }
    }
    Ok(())
}

async fn handle_key(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    running_download: &mut Option<RunningDownload>,
    key: KeyEvent,
) -> anyhow::Result<bool> {
    if is_ctrl_c(&key) {
        return Err(UserQuit.into());
    }

    let mode = tui.mode.clone();
    match (mode, key.code) {
        (LocalModelsMode::Browse, KeyCode::Char('q') | KeyCode::Esc) => return Ok(true),
        (LocalModelsMode::Browse, KeyCode::Down) => tui.move_selection_down(),
        (LocalModelsMode::Browse, KeyCode::Up) => tui.move_selection_up(),
        (LocalModelsMode::Browse, KeyCode::Enter) => handle_selected_entry(tui, registry)?,
        (LocalModelsMode::Browse, KeyCode::Char('i')) => tui.show_info(),
        (LocalModelsMode::Browse, KeyCode::Char('c')) => {
            tui.toast = None;
            tui.show_custom_input();
        }
        (LocalModelsMode::ErrorDialog { .. }, KeyCode::Enter | KeyCode::Esc) => {
            tui.close_error_dialog()
        }
        (LocalModelsMode::Browse, KeyCode::Char('d')) => tui.confirm_delete(),
        (LocalModelsMode::Info { .. }, KeyCode::Esc | KeyCode::Char('q')) => tui.back_to_browse(),
        (LocalModelsMode::Downloading(_), KeyCode::Enter | KeyCode::Tab | KeyCode::Esc) => {
            cancel_download(running_download)
        }
        (LocalModelsMode::CustomModelInput { .. }, KeyCode::Esc | KeyCode::Char('q')) => {
            tui.back_to_browse()
        }
        (
            LocalModelsMode::CustomModelInput {
                input,
                selected_action,
            },
            KeyCode::Enter,
        ) => {
            if selected_action == DialogAction::Ok {
                resolve_custom_input(tui, input.value()).await;
            } else {
                tui.back_to_browse();
            }
        }
        (LocalModelsMode::CustomModelInput { .. }, KeyCode::Left | KeyCode::Right) => {
            tui.toggle_dialog_action()
        }
        (
            LocalModelsMode::CustomModelInput {
                input,
                selected_action,
            },
            _,
        ) => {
            let mut input = input.clone();
            input.handle_event(&Event::Key(key));
            tui.mode = LocalModelsMode::CustomModelInput {
                input,
                selected_action,
            };
        }
        (
            LocalModelsMode::ConfirmDownload { .. } | LocalModelsMode::ConfirmDelete { .. },
            KeyCode::Left | KeyCode::Right,
        ) => tui.toggle_dialog_action(),
        (LocalModelsMode::CustomModelDetails { .. }, KeyCode::Esc | KeyCode::Char('q')) => {
            tui.back_to_browse()
        }
        (LocalModelsMode::CustomModelDetails { .. }, KeyCode::Left | KeyCode::Right) => {
            tui.toggle_dialog_action()
        }
        (LocalModelsMode::CustomModelDetails { .. }, KeyCode::Tab) => {
            toggle_custom_details_focus(tui)
        }
        (
            LocalModelsMode::CustomModelDetails {
                selected_action, ..
            },
            KeyCode::Enter,
        ) => {
            if selected_action == DialogAction::Ok {
                start_custom_details_download(tui, running_download)
            } else {
                tui.back_to_browse();
            }
        }
        (
            LocalModelsMode::CustomModelDetails {
                id_input,
                name_input,
                focus,
                resolved_entry,
                selected_action,
            },
            _,
        ) => {
            let mut id_input = id_input.clone();
            let mut name_input = name_input.clone();
            match focus {
                CustomModelDetailsFocus::Id => id_input.handle_event(&Event::Key(key)),
                CustomModelDetailsFocus::Name => name_input.handle_event(&Event::Key(key)),
            };
            tui.mode = LocalModelsMode::CustomModelDetails {
                resolved_entry,
                id_input,
                name_input,
                focus,
                selected_action,
            };
        }
        (
            LocalModelsMode::ConfirmDownload { .. },
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'),
        ) => tui.back_to_browse(),
        (
            LocalModelsMode::ConfirmDownload {
                entry,
                selected_action,
            },
            KeyCode::Enter,
        ) => {
            if selected_action == DialogAction::Ok {
                start_confirmed_download(tui, running_download, &entry);
            } else {
                tui.back_to_browse();
            }
        }
        (
            LocalModelsMode::ConfirmDownload { entry, .. },
            KeyCode::Char('y') | KeyCode::Char('Y'),
        ) => {
            start_confirmed_download(tui, running_download, &entry);
        }
        (
            LocalModelsMode::ConfirmDelete { .. },
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'),
        ) => tui.back_to_browse(),
        (
            LocalModelsMode::ConfirmDelete {
                entry,
                selected_action,
            },
            KeyCode::Enter,
        ) => {
            if selected_action == DialogAction::Ok {
                delete_confirmed_entry(tui, registry, &entry)?;
            } else {
                tui.back_to_browse();
            }
        }
        (LocalModelsMode::ConfirmDelete { entry, .. }, KeyCode::Char('y') | KeyCode::Char('Y')) => {
            delete_confirmed_entry(tui, registry, &entry)?
        }
        _ => {}
    }

    Ok(false)
}

fn handle_selected_entry(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
) -> anyhow::Result<()> {
    let Some(entry) = tui.selected_entry().cloned() else {
        return Ok(());
    };

    if !entry.is_downloaded {
        if entry.is_available_in_registry {
            tui.mode = LocalModelsMode::ConfirmDownload {
                entry,
                selected_action: DialogAction::Ok,
            };
        } else {
            tui.show_error_dialog("Custom models must be added through [c]".to_string());
        }
        return Ok(());
    }

    match activate_entry(&entry) {
        Ok(()) => {
            tui.toast = Some(Toast::new(format!("Activated {}", entry.name)));
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => tui.toast = Some(Toast::new(error.to_string())),
    }
    Ok(())
}

fn start_confirmed_download(
    tui: &mut LocalModelsTui,
    running_download: &mut Option<RunningDownload>,
    entry: &LocalModelEntry,
) {
    let running = start_download(registry_entry_from_model(entry), false);
    sync_download_progress(tui, &running);
    *running_download = Some(running);
}

fn cancel_download(running_download: &mut Option<RunningDownload>) {
    if let Some(running) = running_download.as_ref() {
        running.handle.cancel();
        if let Ok(mut state) = running.state.lock() {
            state.status = "Cancelling download".to_string();
        }
    }
}

async fn resolve_custom_input(tui: &mut LocalModelsTui, value: &str) {
    match resolve_custom_model(value).await {
        Ok(entry) => {
            tui.toast = None;
            tui.mode = LocalModelsMode::CustomModelDetails {
                id_input: Input::new(entry.id.clone()),
                name_input: Input::new(entry.name.clone()),
                resolved_entry: entry,
                focus: CustomModelDetailsFocus::Id,
                selected_action: DialogAction::Ok,
            };
        }
        Err(error) => tui.toast = Some(Toast::new(error.to_string())),
    }
}

fn toggle_custom_details_focus(tui: &mut LocalModelsTui) {
    if let LocalModelsMode::CustomModelDetails {
        resolved_entry,
        id_input,
        name_input,
        focus,
        selected_action,
    } = tui.mode.clone()
    {
        tui.mode = LocalModelsMode::CustomModelDetails {
            resolved_entry,
            id_input,
            name_input,
            focus: match focus {
                CustomModelDetailsFocus::Id => CustomModelDetailsFocus::Name,
                CustomModelDetailsFocus::Name => CustomModelDetailsFocus::Id,
            },
            selected_action,
        };
    }
}

fn start_custom_details_download(
    tui: &mut LocalModelsTui,
    running_download: &mut Option<RunningDownload>,
) {
    let LocalModelsMode::CustomModelDetails {
        mut resolved_entry,
        id_input,
        name_input,
        ..
    } = tui.mode.clone()
    else {
        return;
    };
    let id = id_input.value().trim();
    let name = name_input.value().trim();
    if !is_safe_model_id(id) {
        tui.toast = Some(Toast::new(
            "Model ID must use lowercase letters, numbers, '.', '_' or '-'",
        ));
        return;
    }
    if tui.entries.iter().any(|entry| entry.id == id) {
        tui.toast = Some(Toast::new(format!("Model ID '{id}' already exists")));
        return;
    }
    if name.is_empty() {
        tui.toast = Some(Toast::new("Model name is required"));
        return;
    }
    resolved_entry.id = id.to_string();
    resolved_entry.name = name.to_string();
    if let Err(error) = validate_custom_model_registration(&resolved_entry) {
        tui.toast = Some(Toast::new(error.to_string()));
        return;
    }
    let running = start_download(resolved_entry, true);
    sync_download_progress(tui, &running);
    *running_download = Some(running);
}

fn delete_confirmed_entry(
    tui: &mut LocalModelsTui,
    registry: &[RegistryEntry],
    entry: &LocalModelEntry,
) -> anyhow::Result<()> {
    match delete_entry(entry) {
        Ok(()) => {
            tui.toast = Some(Toast::new(format!("Deleted {}", entry.name)));
            tui.back_to_browse();
            tui.refresh(&load_state(), registry)?;
        }
        Err(error) => {
            tui.back_to_browse();
            tui.show_error_dialog(error.to_string());
        }
    }
    Ok(())
}

pub async fn handle_local_models_with_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    let local_state = load_state();
    let registry = fetch_registry().await.unwrap_or_default();
    let selected_model = config::get_selected_model_entry()?;
    let entries = build_local_model_entries(&local_state, &registry, selected_model.as_ref());
    let mut tui = LocalModelsTui::new(entries.clone(), downloaded_model_disk_usage_bytes(&entries));
    if registry.is_empty() {
        tui.show_error_dialog(
            "Could not load remote registry; custom URL entry is still available with [c]"
                .to_string(),
        );
    }
    let mut running_download: Option<RunningDownload> = None;

    loop {
        if tui.toast.as_ref().is_some_and(Toast::is_expired) {
            tui.toast = None;
        }
        finish_completed_download(&mut tui, &registry, &mut running_download).await?;
        terminal.draw(|frame| render_local_models(frame, &tui))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if handle_key(&mut tui, &registry, &mut running_download, key).await? {
                break;
            }
        }
    }
    Ok(())
}

pub async fn handle_local_models() -> anyhow::Result<()> {
    let mut terminal = TerminalGuard::new()?;
    handle_local_models_with_terminal(&mut terminal.terminal).await
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
            group_id: None,
        }
    }

    #[test]
    fn build_local_model_entries_merges_registry_and_custom_entries() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![RegistryEntry {
                    category: Some("custom".to_string()),
                    group_id: Some("Custom".to_string()),
                    ..registry_entry("custom")
                }],
            };

            let entries = build_local_model_entries(&state, &registry, None);

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
    fn build_local_model_entries_marks_downloaded_and_active_from_filesystem_and_selection() {
        with_isolated_models_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1, 2, 3]).expect("write model");
            let selected = SelectedModel {
                provider_id: "local".to_string(),
                model_id: "turbo".to_string(),
            };

            let entries =
                build_local_model_entries(&LocalModelState::default(), &registry, Some(&selected));

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
            let entries = build_local_model_entries(&LocalModelState::default(), &registry, None);

            assert_eq!(downloaded_model_disk_usage_bytes(&entries), 3);
        });
    }

    #[test]
    fn selection_navigation_stays_in_bounds() {
        let entries = vec![
            LocalModelEntry {
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
                sha256: None,
                group_id: None,
            },
            LocalModelEntry {
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
                sha256: None,
                group_id: None,
            },
        ];
        let mut tui = LocalModelsTui::new(entries, 0);

        tui.move_selection_up();
        assert_eq!(tui.selected, 0);
        tui.move_selection_down();
        tui.move_selection_down();
        assert_eq!(tui.selected, 1);
    }

    #[test]
    fn selection_navigation_uses_rendered_group_order() {
        let entries = vec![
            LocalModelEntry {
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
                sha256: None,
                group_id: None,
            },
            LocalModelEntry {
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
                sha256: None,
                group_id: None,
            },
            LocalModelEntry {
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
                sha256: None,
                group_id: None,
            },
        ];
        let mut tui = LocalModelsTui::new(entries, 0);

        assert_eq!(tui.selected_entry().expect("selected").id, "tiny");
        assert_eq!(display_index_for_selected_model(&tui), Some(1));

        tui.move_selection_down();
        assert_eq!(tui.selected_entry().expect("selected").id, "turbo");
        assert_eq!(display_index_for_selected_model(&tui), Some(2));

        tui.move_selection_down();
        assert_eq!(tui.selected_entry().expect("selected").id, "large-v3");
        assert_eq!(display_index_for_selected_model(&tui), Some(3));
    }

    #[test]
    fn selection_index_skips_empty_downloaded_placeholder() {
        let entries = vec![LocalModelEntry {
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
            sha256: None,
            group_id: None,
        }];
        let tui = LocalModelsTui::new(entries, 0);

        assert_eq!(display_index_for_selected_model(&tui), Some(1));
    }

    #[test]
    fn progress_bar_uses_light_shade_for_remaining_width() {
        assert_eq!(progress_bar(0.5, 4), "██░░");
    }

    #[test]
    fn info_and_escape_return_to_browse() {
        let entries = vec![LocalModelEntry {
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
            sha256: None,
            group_id: None,
        }];
        let mut tui = LocalModelsTui::new(entries, 0);

        tui.show_info();
        assert!(matches!(tui.mode, LocalModelsMode::Info { .. }));
        tui.back_to_browse();
        assert!(matches!(tui.mode, LocalModelsMode::Browse));
    }

    #[test]
    fn activation_requires_downloaded_file_and_saves_local_selection() {
        with_isolated_models_dir(|_| {
            let mut entry = LocalModelEntry {
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
                sha256: None,
                group_id: None,
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
            let entry = LocalModelEntry {
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
                sha256: None,
                group_id: None,
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
