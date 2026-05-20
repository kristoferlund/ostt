mod views;

use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};


#[derive(Debug)]
pub struct UserQuit;

impl std::fmt::Display for UserQuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("quit")
    }
}

impl std::error::Error for UserQuit {}

pub struct ModelSelector {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl ModelSelector {
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        use views::provider::ModelProviderChoice;

        loop {
            let choice = views::provider::choose_model_provider(&mut self.terminal).await?;
            let result = match choice {
                ModelProviderChoice::Quit => break,
                ModelProviderChoice::Local => {
                    views::local::handle_local_models_with_terminal(&mut self.terminal).await
                }
                ModelProviderChoice::Cloud => {
                    views::cloud::run_cloud_model_selector(&mut self.terminal).await
                }
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
}

impl Drop for ModelSelector {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::views::cloud::{build_cloud_provider_sections, save_cloud_selection};

    #[test]
    fn cloud_sections_include_only_authenticated_providers() {
        let sections = build_cloud_provider_sections(
            &["openai".to_string(), "local".to_string()],
            None,
        );

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title, "OpenAI");
        assert!(sections[0]
            .models
            .iter()
            .all(|entry| entry.provider_id == "openai"));
    }

    #[test]
    fn selection_save_helpers_persist_provider_aware_state() {
        use crate::config;
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let _guard = crate::transcription::local_models::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-model-test-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        std::env::set_var("HOME", &dir);
        crate::transcription::local_models::set_test_models_dir(Some(dir.join("models")));

        let whisper = crate::transcription::TranscriptionModel::Whisper;
        save_cloud_selection("openai", &whisper).expect("save cloud selection");
        let config = config::OsttConfig::load().expect("load config");
        assert_eq!(config.transcription.provider.as_deref(), Some("openai"));
        assert_eq!(config.transcription.model.as_deref(), Some(whisper.id()));
        let selected = config::get_selected_model_entry()
            .expect("load selection")
            .expect("selected cloud model");
        assert_eq!(selected.provider_id, "openai");
        assert_eq!(selected.model_id, whisper.id());

        crate::transcription::local_models::set_test_models_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }
}
