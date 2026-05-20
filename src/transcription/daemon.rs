//! Local model daemon server.
//!
//! Loads a `WhisperContext` once and serves transcription requests over a Unix
//! domain socket until the idle timeout expires. Each message is framed with a
//! 4-byte little-endian length prefix followed by UTF-8 JSON.
//!
//! Request variants (JSON `"type"` field):
//!   `"ping"`       — returns `model_id`; used by clients to verify the daemon
//!   `"transcribe"` — expects `"audio_path"`; returns `"text"`
//!   `"shutdown"`   — triggers a graceful shutdown
//!
//! The daemon writes its PID to `daemon_pid_path()` on startup and removes
//! both the socket and PID files on exit.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::timeout;

use crate::transcription::api::local::{
    filter_obvious_hallucination, load_audio_for_whisper, validate_local_audio_format,
};
use crate::transcription::local_models::{
    daemon_pid_path, daemon_socket_path, resolve_installed_model_path,
};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

// ── Protocol types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    Ping,
    Transcribe { audio_path: String },
    Shutdown,
}

#[derive(Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// ── Cleanup guard ─────────────────────────────────────────────────────────────

struct CleanupFiles {
    socket_path: PathBuf,
    pid_path: PathBuf,
}

impl Drop for CleanupFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.pid_path);
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the daemon for `model_id`.
///
/// `idle_timeout_secs` is `Some(n)` to exit after `n` seconds of inactivity,
/// or `None` to run indefinitely until a shutdown request arrives.
pub async fn run(model_id: &str, idle_timeout_secs: Option<u64>) -> anyhow::Result<()> {
    let socket_path = daemon_socket_path();
    let pid_path = daemon_pid_path();

    // Remove any stale socket from a previous unclean exit.
    let _ = std::fs::remove_file(&socket_path);

    // Ensure the parent directory exists.
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Load the model (expensive — this is the whole point of the daemon).
    tracing::info!("daemon: loading model '{model_id}'");
    let model_path = resolve_installed_model_path(model_id)?;
    let model_path_str = model_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("model path is not valid UTF-8"))?
        .to_string();

    let ctx = tokio::task::spawn_blocking(move || {
        WhisperContext::new_with_params(&model_path_str, WhisperContextParameters::default())
            .map_err(|e| anyhow::anyhow!("Failed to load model: {e}"))
    })
    .await??;

    let ctx = Arc::new(ctx);

    // Bind socket.
    let listener = UnixListener::bind(&socket_path).context("Failed to bind daemon socket")?;

    // Write PID file.
    std::fs::write(&pid_path, std::process::id().to_string())?;

    // Cleanup socket + PID on drop (covers both clean exit and panic).
    let _cleanup = CleanupFiles {
        socket_path: socket_path.clone(),
        pid_path: pid_path.clone(),
    };

    match idle_timeout_secs {
        Some(secs) => tracing::info!("daemon: model '{model_id}' loaded, ready (idle timeout {secs}s)"),
        None => tracing::info!("daemon: model '{model_id}' loaded, ready (no idle timeout)"),
    }

    loop {
        let accept_fut = listener.accept();
        let result = if let Some(secs) = idle_timeout_secs {
            timeout(Duration::from_secs(secs), accept_fut).await
        } else {
            // No idle timeout: wrap in an infallible timeout that never fires.
            Ok(accept_fut.await.map_err(|e| e))
        };

        match result {
            Err(_elapsed) => {
                tracing::info!("daemon: idle timeout reached, shutting down");
                break;
            }
            Ok(Err(e)) => {
                tracing::error!("daemon: accept error: {e}");
                break;
            }
            Ok(Ok((stream, _addr))) => {
                let model_id_owned = model_id.to_string();
                let ctx_clone = Arc::clone(&ctx);
                match handle_connection(stream, &model_id_owned, ctx_clone).await {
                    Ok(true) => {
                        tracing::info!("daemon: shutdown requested");
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::error!("daemon: connection error: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Connection handler ────────────────────────────────────────────────────────

/// Returns `Ok(true)` to signal the daemon should shut down.
async fn handle_connection(
    mut stream: UnixStream,
    model_id: &str,
    ctx: Arc<WhisperContext>,
) -> anyhow::Result<bool> {
    let raw = read_framed(&mut stream).await?;
    let request: Request = serde_json::from_slice(&raw)?;

    match request {
        Request::Ping => {
            write_framed(
                &mut stream,
                &Response {
                    ok: true,
                    model_id: Some(model_id.to_string()),
                    text: None,
                    error: None,
                },
            )
            .await?;
            Ok(false)
        }
        Request::Shutdown => {
            write_framed(
                &mut stream,
                &Response {
                    ok: true,
                    model_id: Some(model_id.to_string()),
                    text: None,
                    error: None,
                },
            )
            .await?;
            Ok(true)
        }
        Request::Transcribe { audio_path } => {
            let path = PathBuf::from(&audio_path);
            let resp = match run_inference(&path, ctx).await {
                Ok(text) => Response {
                    ok: true,
                    model_id: Some(model_id.to_string()),
                    text: Some(text),
                    error: None,
                },
                Err(e) => Response {
                    ok: false,
                    model_id: None,
                    text: None,
                    error: Some(e.to_string()),
                },
            };
            write_framed(&mut stream, &resp).await?;
            Ok(false)
        }
    }
}

// ── Inference ─────────────────────────────────────────────────────────────────

async fn run_inference(audio_path: &std::path::Path, ctx: Arc<WhisperContext>) -> anyhow::Result<String> {
    validate_local_audio_format(audio_path)?;
    let audio_samples = load_audio_for_whisper(audio_path)?;

    let raw = tokio::task::spawn_blocking(move || {
        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {e}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_timestamps(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_no_context(true);
        params.set_temperature(0.0);
        params.set_entropy_thold(2.4);
        params.set_no_speech_thold(0.6);
        state
            .full(params, &audio_samples)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {e}"))?;
        let n = state.full_n_segments();
        let mut text = String::new();
        for i in 0..n {
            let seg = state
                .get_segment(i)
                .ok_or_else(|| anyhow::anyhow!("Failed to get segment {i}"))?;
            text.push_str(&seg.to_string());
            text.push(' ');
        }
        Ok::<String, anyhow::Error>(text.trim().to_string())
    })
    .await
    .map_err(|e| anyhow::anyhow!("Blocking task panicked: {e}"))??;

    Ok(filter_obvious_hallucination(&raw).unwrap_or_default())
}

// ── Framing helpers ───────────────────────────────────────────────────────────

async fn read_framed(stream: &mut UnixStream) -> anyhow::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_framed(stream: &mut UnixStream, resp: &Response) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(resp)?;
    let len = payload.len() as u32;
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(&payload).await?;
    Ok(())
}
