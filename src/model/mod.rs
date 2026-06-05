mod local_model_view;
mod model_view;

pub use model_view::ModelView;

#[derive(Debug)]
pub struct UserQuit;

impl std::fmt::Display for UserQuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("quit")
    }
}

impl std::error::Error for UserQuit {}

#[cfg(test)]
mod tests {
    use super::local_model_view::build_model_entries;
    use crate::transcription::local_models::LocalModelState;

    #[test]
    fn unified_model_entries_include_only_authenticated_cloud_providers() {
        let config = crate::config::OsttConfig::default();
        let entries = build_model_entries(
            &config,
            &["openai".to_string(), "local".to_string()],
            &LocalModelState::default(),
            &[],
            None,
            None,
        );

        assert!(entries.iter().any(|entry| entry.provider_id == "openai"));
        assert!(entries.iter().all(|entry| entry.provider_id != "local"));
    }

    #[test]
    fn unified_model_entries_mark_only_selected_provider_model_active() {
        let selected = crate::config::SelectedModel {
            provider_id: "openai".to_string(),
            model_id: "whisper-1".to_string(),
        };
        let config = crate::config::OsttConfig::default();

        let entries = build_model_entries(
            &config,
            &["openai".to_string(), "groq".to_string()],
            &LocalModelState::default(),
            &[],
            Some(&selected),
            None,
        );

        let active_entries: Vec<_> = entries.iter().filter(|entry| entry.is_active).collect();
        assert_eq!(active_entries.len(), 1);
        assert_eq!(active_entries[0].provider_id, "openai");
        assert_eq!(active_entries[0].id, selected.model_id);
    }

    #[test]
    fn unified_model_entries_include_configured_profiles_first() {
        let mut config = crate::config::OsttConfig::default();
        let mut http_profile = crate::config::ProviderModelConfig::default();
        http_profile.settings.display_name = Some("Speaches".to_string());
        http_profile.settings.endpoint =
            Some("http://localhost:8000/v1/audio/transcriptions".to_string());
        config
            .provider_configs
            .entry("http".to_string())
            .or_default()
            .models
            .insert("speaches".to_string(), http_profile);
        let selected = crate::config::SelectedModel {
            provider_id: "http".to_string(),
            model_id: "speaches".to_string(),
        };

        let entries = build_model_entries(
            &config,
            &[],
            &LocalModelState::default(),
            &[],
            Some(&selected),
            None,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].group_id.as_deref(), Some("Custom models"));
        assert_eq!(entries[0].provider_id, "http");
        assert_eq!(entries[0].id, "speaches");
        assert!(entries[0].is_active);
    }

    #[test]
    fn selection_save_helpers_persist_provider_aware_state() {
        use crate::config;
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let _guard = crate::transcription::local_models::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_config_home = std::env::var_os("XDG_CONFIG_HOME");

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ostt-model-test-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        std::env::set_var("HOME", &dir);
        std::env::set_var("XDG_CONFIG_HOME", dir.join(".config"));
        crate::transcription::local_models::set_test_models_dir(Some(dir.join("models")));

        config::save_selected_model("openai", "whisper-1").expect("save cloud selection");
        let config = config::OsttConfig::load().expect("load config");
        assert_eq!(config.transcription.provider.as_deref(), Some("openai"));
        assert_eq!(config.transcription.model.as_deref(), Some("whisper-1"));
        let selected = config::get_selected_model_entry()
            .expect("load selection")
            .expect("selected cloud model");
        assert_eq!(selected.provider_id, "openai");
        assert_eq!(selected.model_id, "whisper-1");

        crate::transcription::local_models::set_test_models_dir(None);
        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(previous_xdg_config_home) = previous_xdg_config_home {
            std::env::set_var("XDG_CONFIG_HOME", previous_xdg_config_home);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
