use std::path::Path;

use super::TranscriptionConfig;
use crate::transcription::local_models::{resolve_installed_model_path, ModelError};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    // ── Daemon path ───────────────────────────────────────────────────────────
    // When daemon mode is enabled (default: true), ensure the model is kept
    // loaded in a background process and route the request through it.
    // Any failure falls through to direct (in-process) transcription.
    if let Some(lc) = config.local_config() {
        let effective = lc.effective_for_model(&config.model_id);
        if effective.daemon {
            match try_daemon_transcription(&config.model_id, audio_path, effective.daemon_idle_timeout_secs).await {
                Some(result) => return result,
                None => tracing::debug!("daemon unavailable, falling back to direct transcription"),
            }
        }
    }

    // ── Direct (in-process) path ──────────────────────────────────────────────
    let model_path = resolve_installed_model_path(&config.model_id)?;
    validate_local_audio_format(audio_path)?;
    let audio_samples = load_audio_for_whisper(audio_path)?;
    whisper_rs::install_logging_hooks();

    #[cfg(all(target_os = "macos", not(feature = "whisper-cuda"), not(feature = "whisper-vulkan")))]
    tracing::debug!("local transcription: Metal GPU acceleration enabled");
    #[cfg(feature = "whisper-cuda")]
    tracing::debug!("local transcription: CUDA GPU acceleration enabled");
    #[cfg(feature = "whisper-vulkan")]
    tracing::debug!("local transcription: Vulkan GPU acceleration enabled");
    #[cfg(not(any(target_os = "macos", feature = "whisper-cuda", feature = "whisper-vulkan")))]
    tracing::debug!("local transcription: CPU inference");

    let text = tokio::task::spawn_blocking(move || {
        let model_path = model_path.to_string_lossy().into_owned();
        // WhisperContextParameters::default() sets use_gpu: cfg!(feature = "_gpu").
        // On macOS the binary is built with the metal feature, so GPU is used automatically.
        let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .map_err(|err| ModelError::LoadFailed(err.to_string()))?;
        let mut state = ctx
            .create_state()
            .map_err(|err| anyhow::anyhow!("Failed to create whisper state: {err}"))?;
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
            .map_err(|err| anyhow::anyhow!("Transcription failed: {err}"))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|err| anyhow::anyhow!("Failed to get whisper segments: {err}"))?;
        let mut text = String::new();
        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .map_err(|err| anyhow::anyhow!("Failed to get whisper segment {i}: {err}"))?;
            text.push_str(&segment);
            text.push(' ');
        }

        Ok::<String, anyhow::Error>(text.trim().to_string())
    })
    .await
    .map_err(|err| anyhow::anyhow!("Local whisper runtime task failed: {err}"))??;

    Ok(filter_obvious_hallucination(&text).unwrap_or_default())
}

/// Try to route transcription through the daemon. Returns `Some(result)` when
/// the daemon handled the request (successfully or with an error), `None` when
/// the daemon is unavailable and the caller should fall back to direct transcription.
async fn try_daemon_transcription(
    model_id: &str,
    audio_path: &Path,
    idle_timeout_secs: u64,
) -> Option<anyhow::Result<String>> {
    use crate::transcription::daemon_client;

    if let Err(e) = daemon_client::ensure_daemon(model_id, Some(idle_timeout_secs)).await {
        tracing::warn!("could not ensure daemon for model '{model_id}': {e}");
        return None;
    }

    match daemon_client::request_transcription(audio_path).await {
        Ok(raw) => Some(Ok(filter_obvious_hallucination(&raw).unwrap_or_default())),
        Err(e) => {
            tracing::warn!("daemon transcription failed: {e}");
            None
        }
    }
}

pub(crate) fn filter_obvious_hallucination(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();
    let blocked = ["[blank_audio]", "[silence]", "♪", "♫"];
    if blocked.iter().any(|token| lower.contains(token)) {
        return None;
    }

    let alphanumeric = trimmed.chars().filter(|c| c.is_alphanumeric()).count();
    let total = trimmed
        .chars()
        .filter(|c| !c.is_whitespace())
        .count()
        .max(1);
    if (alphanumeric as f32 / total as f32) < 0.30 {
        return None;
    }

    Some(trimmed.to_string())
}

pub(crate) fn load_audio_for_whisper(audio_path: &Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(audio_path)?;
    let mut samples = Vec::new();

    for sample in reader.samples::<i16>() {
        samples.push(sample? as f32 / i16::MAX as f32);
    }

    Ok(samples)
}

pub(crate) fn validate_local_audio_format(audio_path: &Path) -> anyhow::Result<()> {
    let reader = hound::WavReader::open(audio_path).map_err(|err| {
        anyhow::anyhow!(
            "Local transcription requires WAV audio: {err}. Configure [audio] with output_format = \"pcm_s16le -ar 16000\" and sample_rate = 16000."
        )
    })?;
    let spec = reader.spec();

    if spec.sample_format != hound::SampleFormat::Int
        || spec.bits_per_sample != 16
        || spec.sample_rate != 16_000
        || spec.channels != 1
    {
        anyhow::bail!(
            "Local transcription requires WAV signed 16-bit PCM, 16 kHz, mono audio. Configure [audio] with output_format = \"pcm_s16le -ar 16000\" and sample_rate = 16000. Current file has format {:?}, {} bits, {} Hz, {} channel(s).",
            spec.sample_format,
            spec.bits_per_sample,
            spec.sample_rate,
            spec.channels
        );
    }

    Ok(())
}
