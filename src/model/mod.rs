mod cloud_model_info_view;
mod cloud_model_view;
mod local_model_view;
mod model_provider_view;
mod model_view;
mod no_cloud_providers_view;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use model_view::ModelView;

#[derive(Debug)]
pub struct UserQuit;

impl std::fmt::Display for UserQuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("quit")
    }
}

impl std::error::Error for UserQuit {}

pub(crate) fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests {
    use super::cloud_model_view::{build_cloud_provider_sections, save_cloud_selection};

    #[test]
    fn cloud_sections_include_only_authenticated_providers() {
        let sections =
            build_cloud_provider_sections(&["openai".to_string(), "local".to_string()], None);

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
