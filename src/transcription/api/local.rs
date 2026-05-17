use std::path::Path;

use super::TranscriptionConfig;
use crate::transcription::local_models::{resolve_installed_model_path, ModelError};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub async fn transcribe(config: &TranscriptionConfig, audio_path: &Path) -> anyhow::Result<String> {
    let model_path = resolve_installed_model_path(&config.model_id)?;
    validate_local_audio_format(audio_path)?;
    let audio_samples = load_audio_for_whisper(audio_path)?;

    let text = tokio::task::spawn_blocking(move || {
        let model_path = model_path.to_string_lossy().into_owned();
        let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .map_err(|err| ModelError::LoadFailed(err.to_string()))?;
        let mut state = ctx
            .create_state()
            .map_err(|err| anyhow::anyhow!("Failed to create whisper state: {err}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_timestamps(false);
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

fn filter_obvious_hallucination(text: &str) -> Option<String> {
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

fn load_audio_for_whisper(audio_path: &Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(audio_path)?;
    let mut samples = Vec::new();

    for sample in reader.samples::<i16>() {
        samples.push(sample? as f32 / i16::MAX as f32);
    }

    Ok(samples)
}

fn validate_local_audio_format(audio_path: &Path) -> anyhow::Result<()> {
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
