pub(crate) mod handlers;
pub(crate) mod render;
pub(crate) mod types;

use crossterm::event::{self, Event};
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use std::time::Duration;
use ratatui::Terminal;

use crate::transcription::local_models::{fetch_registry, load_state};
use crate::ui::Toast;

pub(crate) use handlers::{build_local_model_entries, downloaded_model_disk_usage_bytes};

pub(crate) async fn handle_local_models_with_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    let local_state = load_state();
    let registry = fetch_registry().await.unwrap_or_default();
    let selected_model = crate::config::get_selected_model_entry()?;
    let entries = build_local_model_entries(&local_state, &registry, selected_model.as_ref());
    let mut tui = types::LocalModelsTui::new(entries.clone(), downloaded_model_disk_usage_bytes(&entries));
    if registry.is_empty() {
        tui.show_error_dialog(
            "Could not load remote registry; custom URL entry is still available with [c]"
                .to_string(),
        );
    }
    let mut running_download: Option<types::RunningDownload> = None;

    loop {
        if tui.toast.as_ref().is_some_and(Toast::is_expired) {
            tui.toast = None;
        }
        handlers::finish_completed_download(&mut tui, &registry, &mut running_download).await?;
        terminal.draw(|frame| render::render_local_models(frame, &tui))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if handlers::handle_key(&mut tui, &registry, &mut running_download, key).await? {
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::types::LocalModelEntry;
    use crate::transcription::local_models::{model_files_dir, set_test_models_dir, LocalModelState, RegistryEntry, TEST_ENV_LOCK};
    use crate::config::SelectedModel;
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
        let mut tui = types::LocalModelsTui::new(entries, 0);

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
        let tui = types::LocalModelsTui::new(entries, 0);

        assert_eq!(tui.selected_entry().expect("selected").id, "tiny");
    }

    #[test]
    fn grouped_display_index_finds_position_within_rendered_groups() {
        use super::render::grouped_display_index;

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
                group_id: Some("group-a".to_string()),
            },
            LocalModelEntry {
                id: "large".to_string(),
                name: "Large".to_string(),
                description: String::new(),
                size_mb: 1,
                is_downloaded: true,
                is_active: false,
                is_available_in_registry: true,
                languages: Vec::new(),
                url: "https://example.com/large.bin".to_string(),
                recommended_hardware: None,
                category: None,
                sha256: None,
                group_id: None,
            },
        ];
        let idx = grouped_display_index(entries.iter().collect(), "tiny", 0);
        assert_eq!(idx, Some(1));
    }
}
