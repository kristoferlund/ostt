//! Provider and model authentication.
//!
//! Unified authentication flow: select a provider/model combination and optionally enter an API key.
//! Users can keep existing API keys by pressing Enter without entering anything.
//!
//! For Berget AI, uses OAuth2 PKCE flow to authenticate and create an API key.

use crate::auth;
use crate::config;
use crate::transcription;
use cliclack::note;
use cliclack::outro;
use cliclack::{intro, password, select};
use console::style;

/// Handles provider + model selection and API key management.
///
/// Shows all available provider/model combinations for the user to choose from.
/// If a provider already has an API key saved, the user can press Enter to keep it.
/// Supports switching between models of the same provider without re-entering the API key.
pub async fn handle_auth() -> Result<(), anyhow::Error> {
    tracing::info!("=== ostt Authentication ===");

    ctrlc::set_handler(move || {}).expect("setting Ctrl-C handler");

    println!("\n ┏┓┏╋╋ \n ┗┛┛┗┗ \n");

    intro(style(" auth ").on_white().black())?;

    // Ensure config file exists - create default config if it doesn't
    if config::OsttConfig::load().is_err() {
        tracing::info!("Config file not found, creating default config");
        crate::setup::run_setup()?;
        tracing::info!("Default config created");
    }

    // Get all available provider/model combinations
    let providers = transcription::TranscriptionProvider::all();
    let mut provider_model_options: Vec<(
        transcription::TranscriptionProvider,
        transcription::TranscriptionModel,
    )> = Vec::new();
    let mut display_options: Vec<String> = Vec::new();

    // Get the currently selected model from secrets (not from config file)
    let maybe_current_model_id = config::get_selected_model().ok().flatten();

    if let Some(current_model_id) = maybe_current_model_id {
        note("current model", current_model_id)?;
    }

    // Build list of all provider/model combinations
    for provider in providers.iter() {
        let models = transcription::TranscriptionModel::models_for_provider(provider);
        for model in models {
            display_options.push(format!("{} / {}", provider.name(), model.description()));
            provider_model_options.push((provider.clone(), model));
        }
    }

    if provider_model_options.is_empty() {
        return Err(anyhow::anyhow!("No provider/model combinations available"));
    }

    let mut select_prompt = select("Select provider and model:");
    for (i, option) in display_options.iter().enumerate() {
        select_prompt = select_prompt.item(i, option, "");
    }
    let selected_idx: usize = select_prompt
        .interact()
        .map_err(|e| anyhow::anyhow!("Selection cancelled: {e}"))?;

    let (selected_provider, selected_model) = &provider_model_options[selected_idx];

    // Check if we already have an API key for this provider
    let current_api_key = config::get_api_key(selected_provider.id()).ok().flatten();

    // For Berget AI, use OAuth2 PKCE flow
    let api_key_to_save = if *selected_provider == transcription::TranscriptionProvider::Berget {
        // Ask user if they want to use OAuth2 PKCE flow or enter API key manually
        let use_oauth = select(format!(
            "How would you like to authenticate with {}?",
            selected_provider.name()
        ))
        .item(0, "OAuth2 (recommended)", "Log in with Berget AI account")
        .item(1, "API Key", "Enter API key manually")
        .interact()
        .map_err(|e| anyhow::anyhow!("Authentication method selection cancelled: {e}"))?;

        if use_oauth == 0 {
            // Use OAuth2 PKCE flow
            println!("\n🔐 Initiating OAuth2 authentication...");
            println!("   This will open your browser for authentication.\n");

            let oauth_result = auth::authenticate_with_pkce().await
                .map_err(|e| anyhow::anyhow!("OAuth2 authentication failed: {e}"))?;

            println!("\n✅ Authentication successful!");
            println!("   Creating API key...\n");

            // Create API key using OAuth token
            let api_key = auth::create_api_key(&oauth_result.access_token).await
                .map_err(|e| anyhow::anyhow!("Failed to create API key: {e}"))?;

            println!("✅ API key created successfully!\n");

            api_key
        } else {
            // Manual API key entry
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

            // If empty input and we have a current key, keep the current one
            if api_key.is_empty() {
                if let Some(key) = current_api_key {
                    key
                } else {
                    return Err(anyhow::anyhow!("API key cannot be empty"));
                }
            } else {
                api_key
            }
        }
    } else {
        // For other providers, use manual API key entry
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

        // If empty input and we have a current key, keep the current one
        if api_key.is_empty() {
            if let Some(key) = current_api_key {
                key
            } else {
                return Err(anyhow::anyhow!("API key cannot be empty"));
            }
        } else {
            api_key
        }
    };

    // Save the API key for this provider
    config::save_api_key(selected_provider.id(), &api_key_to_save)?;

    // Save the selected model to secrets (not to config file)
    config::save_selected_model(selected_provider.id(), selected_model.id())?;
    // Note: save_selected_model ignores provider_id and stores only the model_id
    // since only one model selection is active globally

    outro("✅ Configuration saved.")?;

    tracing::info!(
        "Authentication completed: provider={}, model={}",
        selected_provider.id(),
        selected_model.id()
    );

    Ok(())
}
