//! Client for the local model daemon.
//!
//! Provides functions to probe a running daemon, ensure one is running for a
//! given model (starting it if needed), and send transcription requests.
//!
//! All functions are non-fatal: if the daemon is unreachable the caller should
//! fall back to direct (in-process) transcription.

use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::{sleep, timeout};

use crate::config::LocalTranscriptionConfig;

use super::local_models::daemon_socket_path;

// ── Protocol types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request<'a> {
    Ping,
    Transcribe {
        audio_path: &'a str,
        config: &'a LocalTranscriptionConfig,
    },
    Shutdown,
}

#[derive(Deserialize)]
struct Response {
    ok: bool,
    #[serde(default)]
    model_id: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

// ── Public types ──────────────────────────────────────────────────────────────

/// Information returned by a successful daemon ping.
pub struct DaemonInfo {
    pub model_id: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Probe the daemon. Returns `Some(DaemonInfo)` when it is reachable.
pub async fn probe_daemon() -> Option<DaemonInfo> {
    let mut stream = UnixStream::connect(daemon_socket_path()).await.ok()?;
    let resp = timeout(Duration::from_secs(2), send(&mut stream, &Request::Ping))
        .await
        .ok()??;
    resp.ok.then_some(DaemonInfo {
        model_id: resp.model_id.unwrap_or_default(),
    })
}

/// Ensure the daemon is running and loaded with `model_id`.
///
/// If a daemon is already running for the same model, returns immediately.
/// If a daemon is running for a *different* model, it is shut down first.
/// Waits up to 20 seconds for the daemon to become ready.
///
/// `idle_timeout_secs` is `Some(n)` to apply an inactivity timeout, or `None`
/// to run the daemon until explicitly stopped (used by `daemon start` and services).
pub async fn ensure_daemon(model_id: &str, idle_timeout_secs: Option<u64>) -> anyhow::Result<()> {
    if let Some(info) = probe_daemon().await {
        if info.model_id == model_id {
            tracing::debug!("daemon already loaded for model '{model_id}'");
            return Ok(());
        }
        tracing::debug!(
            "daemon loaded for '{}', shutting down to load '{model_id}'",
            info.model_id
        );
        let _ = shutdown_daemon().await;
        sleep(Duration::from_millis(500)).await;
    }

    spawn_daemon_process(model_id, idle_timeout_secs)?;
    wait_for_daemon(model_id, Duration::from_secs(20)).await
}

/// Send a transcription request to a running daemon.
///
/// The daemon must already be loaded with the correct model. Returns the raw
/// transcription text (before hallucination filtering).
pub async fn request_transcription(
    audio_path: &Path,
    config: &LocalTranscriptionConfig,
) -> anyhow::Result<String> {
    let audio_path_str = audio_path
        .to_str()
        .ok_or_else(|| anyhow!("audio path is not valid UTF-8"))?;
    let mut stream = UnixStream::connect(daemon_socket_path())
        .await
        .context("could not connect to daemon socket")?;
    let resp = timeout(
        Duration::from_secs(120),
        send(
            &mut stream,
            &Request::Transcribe {
                audio_path: audio_path_str,
                config,
            },
        ),
    )
    .await
    .context("daemon transcription timed out")?
    .ok_or_else(|| anyhow!("daemon returned null response"))?;
    if resp.ok {
        Ok(resp.text.unwrap_or_default())
    } else {
        Err(anyhow!(resp.error.unwrap_or_else(|| "daemon error".into())))
    }
}

/// Ask the daemon to shut down gracefully. Ignores errors (daemon may not be running).
pub async fn shutdown_daemon() -> anyhow::Result<()> {
    let Ok(mut stream) = UnixStream::connect(daemon_socket_path()).await else {
        return Ok(());
    };
    let _ = timeout(
        Duration::from_secs(3),
        send(&mut stream, &Request::Shutdown),
    )
    .await;
    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

pub(crate) fn spawn_daemon_process(
    model_id: &str,
    idle_timeout_secs: Option<u64>,
) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("could not determine ostt executable path")?;
    tracing::info!("daemon: spawning for model '{model_id}'");
    let mut args = vec!["daemon", "run", "--model-id", model_id];
    let timeout_str;
    if let Some(secs) = idle_timeout_secs {
        timeout_str = secs.to_string();
        args.push("--idle-timeout-secs");
        args.push(&timeout_str);
    }
    std::process::Command::new(&exe)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn daemon process")?;
    Ok(())
}

async fn wait_for_daemon(model_id: &str, deadline: Duration) -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();
    let mut delay = Duration::from_millis(100);
    loop {
        if let Some(info) = probe_daemon().await {
            if info.model_id == model_id {
                tracing::debug!("daemon ready for model '{model_id}'");
                return Ok(());
            }
        }
        if start.elapsed() >= deadline {
            anyhow::bail!("timed out waiting for daemon (model '{model_id}')");
        }
        sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(2));
    }
}

async fn send<'a>(stream: &mut UnixStream, req: &Request<'a>) -> Option<Response> {
    let payload = serde_json::to_vec(req).ok()?;
    let len = payload.len() as u32;
    stream.write_all(&len.to_le_bytes()).await.ok()?;
    stream.write_all(&payload).await.ok()?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await.ok()?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await.ok()?;
    serde_json::from_slice(&buf).ok()
}
