//! Provider credential authentication.

use crate::config;
use crate::transcription;
use cliclack::outro;
use cliclack::{confirm, intro, note, password, select};
use console::style;
use std::collections::HashSet;

/// Handles cloud provider API key management.
pub async fn handle_auth() -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Authentication ===");

    ctrlc::set_handler(move || {}).expect("setting Ctrl-C handler");

    println!("\n ┏┓┏╋╋ \n ┗┛┛┗┗ \n");

    intro(style(" auth ").on_white().black())?;

    let providers = cloud_providers();
    if providers.is_empty() {
        return Err(anyhow::anyhow!("No cloud providers available"));
    }

    let selected_provider = select_provider("Select provider:", &providers)?;
    let current_api_key = config::get_api_key(selected_provider.id()).ok().flatten();

    let api_key = if current_api_key.is_some() {
        let api_key_prompt = format!(
            "Enter API key for {} (press Enter to keep current):",
            selected_provider.name()
        );
        password(&api_key_prompt)
            .allow_empty()
            .interact()
            .map_err(|e| anyhow::anyhow!("API key input cancelled: {e}"))?
    } else {
        let api_key_prompt = format!("Enter API key for {}:", selected_provider.name());
        password(&api_key_prompt)
            .interact()
            .map_err(|e| anyhow::anyhow!("API key input cancelled: {e}"))?
    };

    let api_key_to_save = if api_key.is_empty() {
        if let Some(key) = current_api_key {
            key
        } else {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }
    } else {
        api_key
    };

    config::save_api_key(selected_provider.id(), &api_key_to_save)?;

    outro("Credential saved. Run `ostt model` to choose a transcription model.")?;

    tracing::info!(
        "Authentication completed: provider={}",
        selected_provider.id()
    );

    Ok(())
}

pub async fn handle_logout() -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Logout ===");

    ctrlc::set_handler(move || {}).expect("setting Ctrl-C handler");

    println!("\n ┏┓┏╋╋ \n ┗┛┛┗┗ \n");

    intro(style(" auth logout ").on_white().black())?;

    let authorized = config::get_authorized_providers()?;
    let providers = authorized_cloud_providers(&authorized);
    if providers.is_empty() {
        note(
            "No cloud credentials found",
            "Run `ostt auth login` to add credentials.",
        )?;
        return Ok(());
    }

    let selected_provider = select_provider("Select provider to log out:", &providers)?;
    let confirmed = confirm(format!(
        "Remove stored credential for {}?",
        selected_provider.name()
    ))
    .interact()
    .map_err(|e| anyhow::anyhow!("Confirmation cancelled: {e}"))?;

    if !confirmed {
        outro("Logout cancelled.")?;
        return Ok(());
    }

    config::clear_api_key(selected_provider.id())?;
    let cleared_selection = clear_selected_model_if_provider_matches(selected_provider.id())?;

    if cleared_selection {
        outro("Credential removed. Active model cleared; run `ostt model` to choose a model.")?;
    } else {
        outro("Credential removed.")?;
    }

    tracing::info!("Logged out provider={}", selected_provider.id());

    Ok(())
}

fn select_provider(
    prompt: &str,
    providers: &[transcription::TranscriptionProvider],
) -> Result<transcription::TranscriptionProvider, anyhow::Error> {
    let mut select_prompt = select(prompt);
    for (i, provider) in providers.iter().enumerate() {
        select_prompt = select_prompt.item(i, provider.name(), "");
    }
    let selected_idx: usize = select_prompt
        .interact()
        .map_err(|e| anyhow::anyhow!("Selection cancelled: {e}"))?;

    Ok(providers[selected_idx].clone())
}

fn cloud_providers() -> Vec<transcription::TranscriptionProvider> {
    transcription::TranscriptionProvider::all()
        .iter()
        .filter(|provider| **provider != transcription::TranscriptionProvider::Local)
        .cloned()
        .collect()
}

fn authorized_cloud_providers(
    authorized_provider_ids: &[String],
) -> Vec<transcription::TranscriptionProvider> {
    let authorized: HashSet<&str> = authorized_provider_ids.iter().map(String::as_str).collect();
    cloud_providers()
        .into_iter()
        .filter(|provider| authorized.contains(provider.id()))
        .collect()
}

fn clear_selected_model_if_provider_matches(provider_id: &str) -> anyhow::Result<bool> {
    if config::get_selected_model_entry()?
        .is_some_and(|selected| selected.provider_id == provider_id)
    {
        config::clear_selected_model()?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestHome {
        previous_home: Option<std::ffi::OsString>,
        dir: std::path::PathBuf,
    }

    impl TestHome {
        fn new() -> Self {
            let previous_home = std::env::var_os("HOME");
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("ostt-auth-test-{unique}"));
            fs::create_dir_all(&dir).expect("create temp home");
            std::env::set_var("HOME", &dir);

            Self { previous_home, dir }
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            if let Some(previous_home) = self.previous_home.take() {
                std::env::set_var("HOME", previous_home);
            } else {
                std::env::remove_var("HOME");
            }
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn login_provider_options_exclude_local() {
        let providers = cloud_providers();

        assert!(!providers.contains(&transcription::TranscriptionProvider::Local));
        assert!(providers.contains(&transcription::TranscriptionProvider::OpenAI));
    }

    #[test]
    fn logout_provider_options_include_only_authorized_cloud_providers() {
        let authorized = vec![
            "local".to_string(),
            "openai".to_string(),
            "unknown".to_string(),
        ];

        let providers = authorized_cloud_providers(&authorized);

        assert_eq!(
            providers,
            vec![transcription::TranscriptionProvider::OpenAI]
        );
    }

    #[test]
    fn saving_api_key_preserves_unrelated_credentials() {
        let _guard = env_lock().lock().expect("lock env");
        let _home = TestHome::new();

        config::save_api_key("openai", "old-openai").expect("save openai");
        config::save_api_key("groq", "groq-key").expect("save groq");
        config::save_api_key("openai", "new-openai").expect("update openai");

        assert_eq!(
            config::get_api_key("openai").expect("get openai"),
            Some("new-openai".to_string())
        );
        assert_eq!(
            config::get_api_key("groq").expect("get groq"),
            Some("groq-key".to_string())
        );
    }

    #[test]
    fn logout_clears_selected_model_only_for_matching_provider() {
        let _guard = env_lock().lock().expect("lock env");
        let _home = TestHome::new();

        config::save_selected_model("openai", "whisper").expect("save selected model");

        assert!(!clear_selected_model_if_provider_matches("groq").expect("clear groq"));
        assert!(config::get_selected_model_entry()
            .expect("load selected model")
            .is_some());

        assert!(clear_selected_model_if_provider_matches("openai").expect("clear openai"));
        assert!(config::get_selected_model_entry()
            .expect("load selected model")
            .is_none());
    }
}
