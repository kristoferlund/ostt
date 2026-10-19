use crate::transcription::local_models;
use crate::transcription::{self, TranscriptionProvider};
use std::io::Write;

pub async fn handle_model() -> anyhow::Result<()> {
    crate::model::run().await
}

pub async fn handle_model_list(
    provider: Option<String>,
    installed: bool,
    json: bool,
) -> anyhow::Result<()> {
    if let Some(provider_id) = provider.as_deref() {
        if TranscriptionProvider::from_id(provider_id).is_none() {
            anyhow::bail!(
                "Unknown provider '{}'. Supported providers: {}.",
                provider_id,
                TranscriptionProvider::supported_ids().join(", ")
            );
        }
    }

    let selected = crate::config::get_selected_model_entry()?;
    let mut rows = Vec::new();

    if !installed {
        for model in transcription::all_models() {
            if provider
                .as_deref()
                .is_some_and(|id| id != model.provider_id)
            {
                continue;
            }
            rows.push(ModelListRow {
                provider: model.provider_id.to_string(),
                model: model.model_id.to_string(),
                name: model.display_name.to_string(),
                installed: None,
                active: selected.as_ref().is_some_and(|selected| {
                    selected.provider_id == model.provider_id && selected.model_id == model.model_id
                }),
            });
        }

        if provider
            .as_deref()
            .is_none_or(|id| matches!(id, "command" | "http"))
        {
            let config = crate::config::OsttConfig::load()
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            for provider_id in ["command", "http"] {
                if provider.as_deref().is_some_and(|id| id != provider_id) {
                    continue;
                }
                if let Some(provider_config) = config.provider_configs.get(provider_id) {
                    for (profile_id, profile) in &provider_config.models {
                        rows.push(ModelListRow {
                            provider: provider_id.to_string(),
                            model: profile_id.clone(),
                            name: profile
                                .settings
                                .display_name
                                .clone()
                                .unwrap_or_else(|| profile_id.clone()),
                            installed: None,
                            active: selected.as_ref().is_some_and(|selected| {
                                selected.provider_id == provider_id
                                    && selected.model_id == *profile_id
                            }),
                        });
                    }
                }
            }
        }
    }

    if provider
        .as_deref()
        .is_none_or(crate::transcription::TranscriptionProvider::is_local_id)
    {
        let state = local_models::load_state();
        let registry = match local_models::fetch_registry().await {
            Ok(registry) => registry,
            Err(_) => local_models::load_registry_entries().unwrap_or_default(),
        };
        for entry in registry.iter().chain(state.custom_models.iter()) {
            if provider
                .as_deref()
                .is_some_and(|id| id != entry.provider_id)
            {
                continue;
            }
            let is_installed = local_models::model_destination(entry).exists();
            if installed && !is_installed {
                continue;
            }
            rows.push(ModelListRow {
                provider: entry.provider_id.clone(),
                model: entry.id.clone(),
                name: entry.name.clone(),
                installed: Some(is_installed),
                active: selected.as_ref().is_some_and(|selected| {
                    selected.provider_id == entry.provider_id && selected.model_id == entry.id
                }),
            });
        }
    }

    if json {
        let rows: Vec<_> = rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "provider": row.provider,
                    "model": row.model,
                    "name": row.name,
                    "installed": row.installed,
                    "active": row.active,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    for row in rows {
        let active = if row.active { " *" } else { "" };
        let installed = match row.installed {
            Some(true) => " [installed]",
            Some(false) => "",
            None => "",
        };
        println!(
            "{}/{}\t{}{}{}",
            row.provider, row.model, row.name, installed, active
        );
    }

    Ok(())
}

pub fn handle_model_current() -> anyhow::Result<()> {
    match crate::config::get_selected_model_entry()? {
        Some(selected) => println!("{}/{}", selected.provider_id, selected.model_id),
        None => println!("No model selected."),
    }
    Ok(())
}

pub fn handle_model_params(model: Option<String>, json: bool) -> anyhow::Result<()> {
    let selected = match model {
        Some(model) => crate::config::parse_provider_model(&model)?,
        None => crate::config::get_selected_model_entry()?.ok_or_else(|| {
            anyhow::anyhow!("No model selected. Pass PROVIDER/MODEL or run 'ostt model' first.")
        })?,
    };
    let full_model_id = format!("{}/{}", selected.provider_id, selected.model_id);
    let schema = transcription::api::option_schema(&selected.provider_id, &selected.model_id)
        .ok_or_else(|| anyhow::anyhow!("No params are supported for {full_model_id}."))?;

    if json {
        let options: Vec<_> = schema
            .options()
            .iter()
            .map(|option| {
                serde_json::json!({
                    "name": option.name,
                    "type": option.kind.name(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "provider": selected.provider_id,
                "model": selected.model_id,
                    "params": options,
            }))?
        );
        return Ok(());
    }

    for option in schema.options() {
        println!("{}: {}", option.name, option.kind.name());
    }

    Ok(())
}

pub async fn handle_model_select(model: String) -> anyhow::Result<()> {
    let selected = crate::config::parse_provider_model(&model)?;
    if crate::transcription::TranscriptionProvider::is_local_id(&selected.provider_id) {
        let activated =
            local_models::activate_model_for_provider(&selected.provider_id, &selected.model_id)?;
        reload_daemon_if_running(&selected.model_id).await?;
        println!(
            "Selected model: {}/{}",
            activated.provider_id, activated.model_id
        );
        return Ok(());
    }

    if matches!(selected.provider_id.as_str(), "command" | "http") {
        let config =
            crate::config::OsttConfig::load().map_err(|err| anyhow::anyhow!(err.to_string()))?;
        if config
            .provider_configs
            .get(&selected.provider_id)
            .and_then(|provider| provider.models.get(&selected.model_id))
            .is_none()
        {
            anyhow::bail!(
                "Unknown external profile: {}/{}",
                selected.provider_id,
                selected.model_id
            );
        }
        crate::config::save_selected_model(&selected.provider_id, &selected.model_id)?;
        println!(
            "Selected model: {}/{}",
            selected.provider_id, selected.model_id
        );
        return Ok(());
    }

    if crate::config::get_api_key(&selected.provider_id)?.is_none() {
        anyhow::bail!(
            "No credential found for provider '{}'. Run 'ostt auth login' first.",
            selected.provider_id
        );
    }

    crate::config::save_selected_model(&selected.provider_id, &selected.model_id)?;
    println!(
        "Selected model: {}/{}",
        selected.provider_id, selected.model_id
    );
    Ok(())
}

pub async fn handle_model_local_download(model_id: String) -> anyhow::Result<()> {
    let requested = parse_local_model_arg(&model_id)?;
    let entry = find_local_model_entry(&requested).await?;
    let destination = local_models::model_destination(&entry);
    let full_model_id = format!("{}/{}", entry.provider_id, entry.id);
    if destination.exists() {
        println!("Local model already downloaded: {full_model_id}");
        return Ok(());
    }

    eprintln!("Downloading local model: {full_model_id}");
    local_models::download_model(
        &entry.url,
        &destination,
        Some(Box::new(|downloaded_bytes, total_bytes, speed_mbps| {
            print_download_progress(downloaded_bytes, total_bytes, speed_mbps);
        })),
    )
    .await?;
    eprintln!();
    local_models::validate_downloaded_model(&entry)?;
    local_models::mark_downloaded_registry_model(&entry)?;
    println!("Downloaded local model: {full_model_id}");
    Ok(())
}

pub async fn handle_model_local_remove(model_id: String) -> anyhow::Result<()> {
    let requested = parse_local_model_arg(&model_id)?;
    let loaded_model_id = crate::transcription::daemon_client::probe_daemon()
        .await
        .map(|info| info.model_id);
    local_models::delete_model(&requested.model_id)?;
    if loaded_model_id.as_deref() == Some(requested.model_id.as_str()) {
        crate::transcription::daemon_client::shutdown_daemon().await?;
    }
    println!("Removed local model: {}", requested.label());
    Ok(())
}

struct ModelListRow {
    provider: String,
    model: String,
    name: String,
    installed: Option<bool>,
    active: bool,
}

struct LocalModelArg {
    /// `None` for a bare model id: any local provider with that model matches.
    provider_id: Option<String>,
    model_id: String,
}

impl LocalModelArg {
    fn matches(&self, entry: &local_models::RegistryEntry) -> bool {
        entry.id == self.model_id
            && self
                .provider_id
                .as_deref()
                .is_none_or(|provider_id| provider_id == entry.provider_id)
    }

    fn label(&self) -> String {
        match &self.provider_id {
            Some(provider_id) => format!("{provider_id}/{}", self.model_id),
            None => self.model_id.clone(),
        }
    }
}

fn parse_local_model_arg(value: &str) -> anyhow::Result<LocalModelArg> {
    if let Some(selected) = value
        .contains('/')
        .then(|| crate::config::parse_provider_model(value))
        .transpose()?
    {
        return Ok(LocalModelArg {
            provider_id: Some(selected.provider_id),
            model_id: selected.model_id,
        });
    }

    Ok(LocalModelArg {
        provider_id: None,
        model_id: value.to_string(),
    })
}

async fn find_local_model_entry(
    requested: &LocalModelArg,
) -> anyhow::Result<local_models::RegistryEntry> {
    let state = local_models::load_state();
    if let Some(entry) = state
        .custom_models
        .iter()
        .find(|entry| requested.matches(entry))
        .cloned()
    {
        return Ok(entry);
    }

    let registry = match local_models::fetch_registry().await {
        Ok(registry) => registry,
        Err(error) => local_models::load_registry_entries().map_err(|_| error)?,
    };
    registry
        .into_iter()
        .find(|entry| requested.matches(entry))
        .ok_or_else(|| anyhow::anyhow!("Unknown local model: {}", requested.label()))
}

async fn reload_daemon_if_running(model_id: &str) -> anyhow::Result<()> {
    let Some(info) = crate::transcription::daemon_client::probe_daemon().await else {
        return Ok(());
    };
    if info.model_id != model_id {
        crate::transcription::daemon_client::ensure_daemon(model_id, None).await?;
    }
    Ok(())
}

fn print_download_progress(downloaded_bytes: u64, total_bytes: u64, speed_mbps: f64) {
    if total_bytes > 0 {
        let percent = (downloaded_bytes as f64 / total_bytes as f64 * 100.0).clamp(0.0, 100.0);
        eprint!(
            "\r{percent:>5.1}%  {} / {}  {speed_mbps:.1} MB/s",
            format_bytes(downloaded_bytes),
            format_bytes(total_bytes),
        );
    } else {
        eprint!(
            "\r{} downloaded  {speed_mbps:.1} MB/s",
            format_bytes(downloaded_bytes),
        );
    }
    let _ = std::io::stderr().flush();
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_local_model_arg_accepts_short_id_for_any_local_provider() {
        // A bare id must not assume Whisper, or `ostt model download
        // parakeet-ultra` would look for a Whisper model of that name.
        let parsed = parse_local_model_arg("parakeet-ultra").unwrap();
        let mut entry = local_models::RegistryEntry {
            id: "parakeet-ultra".to_string(),
            provider_id: "parakeet".to_string(),
            name: String::new(),
            description: String::new(),
            languages: Vec::new(),
            size_mb: 0,
            url: String::new(),
            recommended_hardware: None,
            sha256: None,
            category: None,
            group_id: None,
        };

        assert_eq!(parsed.provider_id, None);
        assert!(parsed.matches(&entry));
        entry.id = "turbo".to_string();
        assert!(!parsed.matches(&entry));
    }

    #[test]
    fn parse_local_model_arg_accepts_full_id() {
        let parsed = parse_local_model_arg("whisper/turbo").unwrap();

        assert_eq!(parsed.provider_id.as_deref(), Some("whisper"));
        assert_eq!(parsed.model_id, "turbo");
    }
}
