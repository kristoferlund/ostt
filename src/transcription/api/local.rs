use std::path::Path;

use super::TranscriptionConfig;
use crate::transcription::local_models::{resolve_installed_model_path, ModelError};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    let model_path = resolve_installed_model_path(&config.model_id)?;
    validate_local_audio_format(audio_path)?;
    let audio_samples = load_audio_for_whisper(audio_path)?;
    let local_config = config.local_config().cloned().unwrap_or_default();
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
        let language = if local_config.language == "auto" {
            None
        } else {
            Some(local_config.language.as_str())
        };
        params.set_language(language);
        params.set_print_timestamps(!local_config.no_timestamps);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_no_context(local_config.no_context);
        params.set_temperature(local_config.temperature);
        params.set_entropy_thold(local_config.entropy_thold);
        params.set_no_speech_thold(local_config.no_speech_thold);

        state
            .full(params, &audio_samples)
            .map_err(|err| anyhow::anyhow!("Transcription failed: {err}"))?;

        let num_segments = state.full_n_segments();
        let mut text = String::new();
        for i in 0..num_segments {
            let segment = state
                .get_segment(i)
                .ok_or_else(|| anyhow::anyhow!("Failed to get whisper segment {i}"))?;
            text.push_str(&segment.to_string());
            text.push(' ');
        }

        Ok::<String, anyhow::Error>(text.trim().to_string())
    })
    .await
    .map_err(|err| anyhow::anyhow!("Local whisper runtime task failed: {err}"))??;

    Ok(filter_obvious_hallucination(&text).unwrap_or_default())
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
