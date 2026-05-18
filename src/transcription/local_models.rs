use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{self, SelectedModel};

pub const REMOTE_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/kristoferlund/ostt-models/main/models.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub languages: Vec<String>,
    pub size_mb: u32,
    pub url: String,
    pub recommended_hardware: Option<String>,
    pub sha256: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelState {
    pub version: u32,
    #[serde(default)]
    pub custom_models: Vec<RegistryEntry>,
}

impl Default for LocalModelState {
    fn default() -> Self {
        Self {
            version: 1,
            custom_models: Vec::new(),
        }
    }
}

/// Returns the directory where local transcription models are stored.
pub fn models_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("OSTT_MODELS_DIR") {
        return PathBuf::from(path);
    }

    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("ostt").join("models")
}

fn state_path() -> PathBuf {
    models_dir().join("models.json")
}

pub fn model_files_dir() -> PathBuf {
    models_dir().join("files")
}

/// Error type for local model-related failures.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Model '{0}' was not found in the local model registry or custom models.")]
    NotFound(String),
    #[error("Model '{0}' is not downloaded. Run `ostt models download {0}` first.")]
    NotDownloaded(String),
    #[error("Model file not found at {0}")]
    FileNotFound(PathBuf),
    #[error("Failed to load model: {0}")]
    LoadFailed(String),
    #[error("Local model registry source is not configured")]
    RegistryUnavailable,
}

#[derive(Debug, Clone)]
pub struct InstalledModelView {
    pub entry: RegistryEntry,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub is_active: bool,
}

pub type DownloadProgressCallback = Box<dyn Fn(u64, u64, f64) + Send + 'static>;

pub fn model_filename(id: &str, url: &str) -> String {
    let extension = url
        .split(['?', '#'])
        .next()
        .and_then(|without_query| without_query.rsplit('/').next())
        .and_then(|segment| segment.rsplit_once('.').map(|(_, extension)| extension))
        .filter(|extension| !extension.is_empty())
        .unwrap_or("bin");

    format!("{id}.{extension}")
}

pub fn model_destination(entry: &RegistryEntry) -> PathBuf {
    model_files_dir().join(model_filename(&entry.id, &entry.url))
}

pub fn mark_downloaded_registry_model(entry: &RegistryEntry) -> anyhow::Result<()> {
    let path = model_destination(entry);
    if !path.exists() {
        anyhow::bail!(
            "download completed but model file is missing at {}",
            path.display()
        );
    }

    Ok(())
}

pub fn register_custom_model(entry: RegistryEntry) -> anyhow::Result<()> {
    let mut state = load_state();
    state.custom_models.retain(|model| model.id != entry.id);
    state.custom_models.push(entry);
    save_state(&state)
}

pub fn validate_downloaded_model(entry: &RegistryEntry) -> anyhow::Result<()> {
    let path = model_destination(entry);
    let metadata = fs::metadata(&path).map_err(|error| {
        anyhow::anyhow!(
            "download completed but model file is missing at {}: {error}",
            path.display()
        )
    })?;

    if let Some(expected_sha256) = entry.sha256.as_deref().filter(|sha| !sha.is_empty()) {
        let bytes = fs::read(&path)?;
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
            anyhow::bail!(
                "downloaded model checksum mismatch for {}: expected {}, got {}",
                entry.id,
                expected_sha256,
                actual_sha256
            );
        }
        return Ok(());
    }

    if entry.size_mb > 0 {
        let expected_bytes = u64::from(entry.size_mb) * 1024 * 1024;
        if metadata.len() != expected_bytes {
            anyhow::bail!(
                "downloaded model size mismatch for {}: expected {} bytes, got {} bytes",
                entry.id,
                expected_bytes,
                metadata.len()
            );
        }
    }

    Ok(())
}

pub fn is_safe_model_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'))
}

pub fn installed_models(
    registry: &[RegistryEntry],
    state: &LocalModelState,
    selected_model: Option<&SelectedModel>,
) -> Vec<InstalledModelView> {
    registry
        .iter()
        .chain(state.custom_models.iter())
        .filter_map(|entry| {
            let path = model_files_dir().join(model_filename(&entry.id, &entry.url));
            let metadata = fs::metadata(&path).ok()?;

            Some(InstalledModelView {
                entry: entry.clone(),
                path,
                size_bytes: metadata.len(),
                modified_at: metadata.modified().ok(),
                is_active: selected_model
                    .map(|selected| {
                        selected.provider_id == "local" && selected.model_id == entry.id
                    })
                    .unwrap_or(false),
            })
        })
        .collect()
}

pub fn load_registry_entries() -> Result<Vec<RegistryEntry>, ModelError> {
    Err(ModelError::RegistryUnavailable)
}

pub async fn fetch_registry() -> anyhow::Result<Vec<RegistryEntry>> {
    fetch_remote_registry().await
}

async fn fetch_remote_registry() -> anyhow::Result<Vec<RegistryEntry>> {
    fetch_registry_from_url(REMOTE_REGISTRY_URL).await
}

async fn fetch_registry_from_url(url: &str) -> anyhow::Result<Vec<RegistryEntry>> {
    let response = reqwest::get(url)
        .await
        .map_err(|error| anyhow::anyhow!("failed to fetch remote model registry: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("failed to fetch remote model registry: HTTP {status}");
    }

    response
        .json::<Vec<RegistryEntry>>()
        .await
        .map_err(|error| anyhow::anyhow!("failed to parse remote model registry: {error}"))
}

pub async fn download_model(
    url: &str,
    dest_path: &Path,
    progress: Option<DownloadProgressCallback>,
) -> anyhow::Result<()> {
    let response = reqwest::get(url)
        .await
        .map_err(|error| anyhow::anyhow!("failed to start model download: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("failed to download model: HTTP {status}");
    }

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let total_bytes = response.content_length().unwrap_or(0);
    let temp_path = dest_path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)?;
    let mut downloaded_bytes = 0_u64;
    let started_at = Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| anyhow::anyhow!("failed while downloading model: {error}"))?;
        file.write_all(&chunk)?;
        downloaded_bytes += chunk.len() as u64;

        if let Some(callback) = progress.as_ref() {
            let elapsed = started_at.elapsed().as_secs_f64();
            let speed_mbps = if elapsed > 0.0 {
                (downloaded_bytes as f64 / (1024.0 * 1024.0)) / elapsed
            } else {
                0.0
            };
            callback(downloaded_bytes, total_bytes, speed_mbps);
        }
    }

    file.sync_all()?;
    drop(file);
    fs::rename(temp_path, dest_path)?;
    Ok(())
}

pub fn load_state() -> LocalModelState {
    let path = state_path();
    if !path.exists() {
        return LocalModelState::default();
    }

    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_state(state: &LocalModelState) -> anyhow::Result<()> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(state)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_custom_model_entries() -> Result<Vec<RegistryEntry>, ModelError> {
    Ok(load_state().custom_models)
}

pub fn resolve_installed_model_path(model_id: &str) -> Result<PathBuf, ModelError> {
    let entry = find_model_entry(model_id)?;
    let path = model_files_dir().join(model_filename(&entry.id, &entry.url));

    if path.exists() {
        Ok(path)
    } else {
        Err(ModelError::NotDownloaded(model_id.to_string()))
    }
}

pub fn activate_model(model_id: &str) -> anyhow::Result<()> {
    let entry = find_model_entry(model_id)?;
    let path = model_files_dir().join(model_filename(&entry.id, &entry.url));

    if !path.exists() {
        return Err(ModelError::NotDownloaded(model_id.to_string()).into());
    }

    config::save_selected_model("local", model_id)
}

pub fn deactivate_model() -> anyhow::Result<()> {
    config::clear_selected_model()
}

pub fn delete_model(model_id: &str) -> anyhow::Result<()> {
    let entry = find_model_entry(model_id)?;
    let file_path = model_files_dir().join(model_filename(&entry.id, &entry.url));

    if !file_path.exists() {
        return Err(ModelError::NotDownloaded(model_id.to_string()).into());
    }

    fs::remove_file(file_path)?;

    if config::get_selected_model_entry()?
        .is_some_and(|selected| selected.provider_id == "local" && selected.model_id == model_id)
    {
        config::clear_selected_model()?;
    }

    Ok(())
}

fn find_model_entry_in(
    model_id: &str,
    registry: &[RegistryEntry],
    state: &LocalModelState,
) -> Result<RegistryEntry, ModelError> {
    state
        .custom_models
        .iter()
        .chain(registry.iter())
        .find(|entry| entry.id == model_id)
        .cloned()
        .ok_or_else(|| ModelError::NotFound(model_id.to_string()))
}

fn find_model_entry(model_id: &str) -> Result<RegistryEntry, ModelError> {
    let state = load_state();
    let registry_entries = load_registry_entries().unwrap_or_default();
    find_model_entry_in(model_id, &registry_entries, &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_isolated_data_dir(test: impl FnOnce(PathBuf)) {
        let _guard = ENV_LOCK.lock().expect("test env lock poisoned");
        let previous = env::var_os("OSTT_MODELS_DIR");
        let previous_home = env::var_os("HOME");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ostt-local-models-test-{unique}"));
        let models_dir = dir.join("models");
        env::set_var("OSTT_MODELS_DIR", &models_dir);
        env::set_var("HOME", &dir);

        test(models_dir);

        if let Some(previous) = previous {
            env::set_var("OSTT_MODELS_DIR", previous);
        } else {
            env::remove_var("OSTT_MODELS_DIR");
        }
        if let Some(previous_home) = previous_home {
            env::set_var("HOME", previous_home);
        } else {
            env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(dir);
    }

    fn registry_entry(id: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: "Test Model".to_string(),
            description: "Test model".to_string(),
            languages: vec!["en".to_string()],
            size_mb: 1,
            url: format!("https://example.com/{id}.bin"),
            recommended_hardware: None,
            sha256: None,
            category: None,
        }
    }

    fn registry_entry_with_url(id: &str, url: &str) -> RegistryEntry {
        RegistryEntry {
            url: url.to_string(),
            ..registry_entry(id)
        }
    }

    fn serve_once(status: &str, content_type: &str, body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let url = format!("http://{}", listener.local_addr().expect("server address"));
        let status = status.to_string();
        let content_type = content_type.to_string();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write headers");
            stream.write_all(&body).expect("write body");
        });

        url
    }

    #[test]
    fn load_state_returns_default_when_missing() {
        with_isolated_data_dir(|_| {
            let state = load_state();

            assert_eq!(state.version, 1);
            assert!(state.custom_models.is_empty());
        });
    }

    #[test]
    fn load_state_returns_default_when_corrupted() {
        with_isolated_data_dir(|_| {
            fs::create_dir_all(models_dir()).expect("create models dir");
            fs::write(state_path(), "not json").expect("write corrupted state");

            let state = load_state();

            assert_eq!(state.version, 1);
            assert!(state.custom_models.is_empty());
        });
    }

    #[test]
    fn save_and_load_state_round_trips() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };

            save_state(&state).expect("save state");
            let loaded = load_state();

            assert_eq!(loaded.version, 1);
            assert_eq!(loaded.custom_models.len(), 1);
            assert_eq!(loaded.custom_models[0].id, "custom");
            assert_eq!(loaded.custom_models[0].languages, vec!["en"]);
        });
    }

    #[test]
    fn save_state_creates_parent_directory() {
        with_isolated_data_dir(|dir| {
            assert!(!dir.join("ostt").join("models").exists());

            save_state(&LocalModelState::default()).expect("save state");

            assert!(state_path().exists());
        });
    }

    #[test]
    fn model_filename_uses_id_with_url_extension() {
        assert_eq!(
            model_filename(
                "kb-whisper-large",
                "https://example.com/models/kb.bin?download=1"
            ),
            "kb-whisper-large.bin"
        );
        assert_eq!(
            model_filename("turbo", "https://example.com/ggml-turbo.gguf#fragment"),
            "turbo.gguf"
        );
        assert_eq!(
            model_filename("custom", "https://example.com/download"),
            "custom.bin"
        );
    }

    #[test]
    fn safe_model_id_allows_only_portable_filename_characters() {
        assert!(is_safe_model_id("kb-whisper.large_v3"));
        assert!(!is_safe_model_id(""));
        assert!(!is_safe_model_id("Large"));
        assert!(!is_safe_model_id("model/name"));
        assert!(!is_safe_model_id("model name"));
    }

    #[test]
    fn installed_models_discovers_registry_and_custom_files() {
        with_isolated_data_dir(|_| {
            let registry = vec![registry_entry_with_url(
                "turbo",
                "https://example.com/ggml-turbo.gguf",
            )];
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.gguf"), [1, 2, 3])
                .expect("write registry model");
            fs::write(model_files_dir().join("custom.bin"), [4, 5]).expect("write custom model");

            let installed = installed_models(&registry, &state, None);

            assert_eq!(installed.len(), 2);
            assert!(installed.iter().any(|model| {
                model.entry.id == "turbo"
                    && model.path == model_files_dir().join("turbo.gguf")
                    && model.size_bytes == 3
                    && model.modified_at.is_some()
            }));
            assert!(installed.iter().any(|model| {
                model.entry.id == "custom"
                    && model.path == model_files_dir().join("custom.bin")
                    && model.size_bytes == 2
            }));
        });
    }

    #[test]
    fn installed_models_marks_selected_model_active() {
        with_isolated_data_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1]).expect("write model");
            let selected_model = SelectedModel {
                provider_id: "local".to_string(),
                model_id: "turbo".to_string(),
            };

            let installed = installed_models(
                &registry,
                &LocalModelState::default(),
                Some(&selected_model),
            );

            assert_eq!(installed.len(), 1);
            assert!(installed[0].is_active);
        });
    }

    #[test]
    fn activate_model_saves_local_provider_and_model_id() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            save_state(&state).expect("save state");
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("custom.bin"), [1]).expect("write custom model");

            activate_model("custom").expect("activate model");
            let selected = config::get_selected_model_entry()
                .expect("load selected model")
                .expect("selected model");

            assert_eq!(selected.provider_id, "local");
            assert_eq!(selected.model_id, "custom");
        });
    }

    #[test]
    fn activate_model_requires_installed_file() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            save_state(&state).expect("save state");

            let error = activate_model("custom").expect_err("activation should fail");

            assert!(error.to_string().contains("not downloaded"));
            assert!(config::get_selected_model_entry()
                .expect("load selected model")
                .is_none());
        });
    }

    #[test]
    fn deactivate_model_clears_selected_model() {
        with_isolated_data_dir(|_| {
            config::save_selected_model("local", "custom").expect("save selected model");

            deactivate_model().expect("deactivate model");

            assert!(config::get_selected_model_entry()
                .expect("load selected model")
                .is_none());
        });
    }

    #[test]
    fn delete_model_removes_file_and_clears_active_selection() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            save_state(&state).expect("save state");
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            let file_path = model_files_dir().join("custom.bin");
            fs::write(&file_path, [1]).expect("write custom model");
            config::save_selected_model("local", "custom").expect("save selected model");

            delete_model("custom").expect("delete model");

            assert!(!file_path.exists());
            assert!(config::get_selected_model_entry()
                .expect("load selected model")
                .is_none());
        });
    }

    #[test]
    fn delete_model_requires_installed_file() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            save_state(&state).expect("save state");

            let error = delete_model("custom").expect_err("delete should fail");

            assert!(error.to_string().contains("not downloaded"));
        });
    }

    #[test]
    fn delete_custom_model_keeps_metadata() {
        with_isolated_data_dir(|_| {
            let state = LocalModelState {
                version: 1,
                custom_models: vec![registry_entry("custom")],
            };
            save_state(&state).expect("save state");
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("custom.bin"), [1]).expect("write custom model");

            delete_model("custom").expect("delete model");
            let state = load_state();

            assert_eq!(state.custom_models.len(), 1);
            assert_eq!(state.custom_models[0].id, "custom");
        });
    }

    #[test]
    fn find_model_entry_reports_missing_model() {
        with_isolated_data_dir(|_| {
            let error = resolve_installed_model_path("missing").expect_err("missing model");

            assert!(matches!(error, ModelError::NotFound(model) if model == "missing"));
        });
    }

    #[test]
    fn installed_models_does_not_persist_registry_entries() {
        with_isolated_data_dir(|_| {
            let registry = vec![registry_entry("turbo")];
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_files_dir().join("turbo.bin"), [1]).expect("write model");

            let installed = installed_models(&registry, &LocalModelState::default(), None);

            assert_eq!(installed.len(), 1);
            assert!(!state_path().exists());
        });
    }

    #[tokio::test]
    async fn fetch_registry_from_url_parses_registry_entries() {
        let body = serde_json::to_vec(&vec![registry_entry("turbo")]).expect("serialize registry");
        let url = serve_once("200 OK", "application/json", body);

        let registry = fetch_registry_from_url(&url).await.expect("fetch registry");

        assert_eq!(registry.len(), 1);
        assert_eq!(registry[0].id, "turbo");
    }

    #[tokio::test]
    async fn fetch_registry_from_url_reports_http_errors() {
        let url = serve_once("404 Not Found", "text/plain", b"missing".to_vec());

        let error = fetch_registry_from_url(&url).await.expect_err("fetch should fail");

        assert!(error.to_string().contains("HTTP 404"));
    }

    #[tokio::test]
    async fn download_model_streams_to_temp_then_final_path_with_progress() {
        let body = b"model-bytes".to_vec();
        let url = serve_once("200 OK", "application/octet-stream", body.clone());
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ostt-download-test-{unique}"));
        let dest_path = dir.join("files").join("turbo.bin");
        let progress_events = std::sync::Arc::new(Mutex::new(Vec::new()));
        let callback_events = progress_events.clone();

        download_model(
            &url,
            &dest_path,
            Some(Box::new(move |downloaded, total, speed| {
                callback_events
                    .lock()
                    .expect("progress lock")
                    .push((downloaded, total, speed));
            })),
        )
        .await
        .expect("download model");

        assert_eq!(fs::read(&dest_path).expect("read final file"), body);
        assert!(!dest_path.with_extension("tmp").exists());
        let events = progress_events.lock().expect("progress lock");
        assert!(events
            .iter()
            .any(|(downloaded, total, speed)| *downloaded == 11 && *total == 11 && *speed >= 0.0));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn model_destination_uses_files_dir_and_derived_filename() {
        with_isolated_data_dir(|_| {
            let entry = registry_entry_with_url("turbo", "https://example.com/ggml-turbo.gguf");

            assert_eq!(model_destination(&entry), model_files_dir().join("turbo.gguf"));
        });
    }

    #[test]
    fn mark_downloaded_registry_model_validates_file_without_state_write() {
        with_isolated_data_dir(|_| {
            let entry = registry_entry("turbo");
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_destination(&entry), [1]).expect("write model file");

            mark_downloaded_registry_model(&entry).expect("mark downloaded");

            assert!(!state_path().exists());
        });
    }

    #[test]
    fn mark_downloaded_registry_model_reports_missing_file() {
        with_isolated_data_dir(|_| {
            let error = mark_downloaded_registry_model(&registry_entry("turbo"))
                .expect_err("missing file should fail");

            assert!(error.to_string().contains("model file is missing"));
        });
    }

    #[test]
    fn register_custom_model_replaces_duplicate_ids() {
        with_isolated_data_dir(|_| {
            register_custom_model(registry_entry_with_url(
                "custom",
                "https://example.com/old.bin",
            ))
            .expect("register old custom model");
            register_custom_model(registry_entry_with_url(
                "custom",
                "https://example.com/new.gguf",
            ))
            .expect("register replacement custom model");

            let state = load_state();
            assert_eq!(state.custom_models.len(), 1);
            assert_eq!(state.custom_models[0].id, "custom");
            assert_eq!(state.custom_models[0].url, "https://example.com/new.gguf");
        });
    }

    #[test]
    fn validate_downloaded_model_uses_sha256_when_available() {
        with_isolated_data_dir(|_| {
            let mut entry = registry_entry("custom");
            entry.sha256 = Some(format!("{:x}", Sha256::digest(b"model bytes")));
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_destination(&entry), b"model bytes").expect("write model file");

            validate_downloaded_model(&entry).expect("validate checksum");
        });
    }

    #[test]
    fn validate_downloaded_model_uses_size_when_checksum_missing() {
        with_isolated_data_dir(|_| {
            let mut entry = registry_entry("custom");
            entry.size_mb = 1;
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_destination(&entry), vec![0_u8; 1024 * 1024]).expect("write model file");

            validate_downloaded_model(&entry).expect("validate size");
        });
    }

    #[test]
    fn validate_downloaded_model_reports_size_mismatch() {
        with_isolated_data_dir(|_| {
            let mut entry = registry_entry("custom");
            entry.size_mb = 1;
            fs::create_dir_all(model_files_dir()).expect("create files dir");
            fs::write(model_destination(&entry), [1]).expect("write model file");

            let error = validate_downloaded_model(&entry).expect_err("size mismatch should fail");

            assert!(error.to_string().contains("size mismatch"));
        });
    }

    #[tokio::test]
    async fn download_model_replaces_existing_file_without_activating() {
        let _guard = ENV_LOCK.lock().expect("test env lock poisoned");
        let previous = env::var_os("OSTT_MODELS_DIR");
        let previous_home = env::var_os("HOME");

        let first_url = serve_once("200 OK", "application/octet-stream", b"first".to_vec());
        let second_url = serve_once("200 OK", "application/octet-stream", b"second".to_vec());
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ostt-redownload-test-{unique}"));
        env::set_var("OSTT_MODELS_DIR", dir.join("models"));
        env::set_var("HOME", &dir);
        config::save_selected_model("local", "active").expect("save selected model");
        let dest_path = model_files_dir().join("turbo.bin");

        download_model(&first_url, &dest_path, None)
            .await
            .expect("download first model");
        download_model(&second_url, &dest_path, None)
            .await
            .expect("replace model");

        assert_eq!(fs::read(&dest_path).expect("read replaced file"), b"second");
        assert!(!dest_path.with_extension("tmp").exists());
        let selected = config::get_selected_model_entry()
            .expect("load selected model")
            .expect("selected model");
        assert_eq!(selected.model_id, "active");
        if let Some(previous) = previous {
            env::set_var("OSTT_MODELS_DIR", previous);
        } else {
            env::remove_var("OSTT_MODELS_DIR");
        }
        if let Some(previous_home) = previous_home {
            env::set_var("HOME", previous_home);
        } else {
            env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(dir);
    }
}
