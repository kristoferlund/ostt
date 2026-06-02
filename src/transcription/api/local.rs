use std::path::Path;

use super::TranscriptionConfig;
use crate::config::{LocalTranscriptionConfig, ModelOptionValue};
use crate::transcription::{
    daemon_client::{probe_daemon, request_transcription},
    local_models::{resolve_installed_model_path, ModelError},
    model::{ModelOptionKind, ModelOptionSchema, ModelOptionSpec},
};
use indexmap::IndexMap;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const OPTIONS: &[ModelOptionSpec] = &[
    ModelOptionSpec {
        name: "entropy_thold",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "language",
        kind: ModelOptionKind::String,
    },
    ModelOptionSpec {
        name: "no_context",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "no_speech_thold",
        kind: ModelOptionKind::Number,
    },
    ModelOptionSpec {
        name: "no_timestamps",
        kind: ModelOptionKind::Bool,
    },
    ModelOptionSpec {
        name: "temperature",
        kind: ModelOptionKind::Number,
    },
];

pub(super) fn option_schema(model_id: &str) -> Option<ModelOptionSchema> {
    (!model_id.is_empty()).then(|| ModelOptionSchema::new(OPTIONS))
}

pub(super) fn validate_options(
    full_model_id: &str,
    options: &IndexMap<String, ModelOptionValue>,
) -> anyhow::Result<()> {
    super::validate_number_min(full_model_id, options, "entropy_thold", 0.0)?;
    super::validate_number_range(full_model_id, options, "no_speech_thold", 0.0..=1.0)?;
    super::validate_number_range(full_model_id, options, "temperature", 0.0..=1.0)?;

    Ok(())
}

pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    validate_local_audio_format(audio_path)?;
    let mut local_config = config.local_config().cloned().unwrap_or_default();
    apply_model_options(config, &mut local_config);

    if let Some(text) = try_daemon_transcription(&config.model_id, audio_path, &local_config).await
    {
        return Ok(text);
    }

    tracing::info!(
        "local transcription mode: in-process (daemon unavailable or not usable for model '{}')",
        config.model_id
    );
    let model_path = resolve_installed_model_path(&config.model_id)?;
    let audio_samples = load_audio_for_whisper(audio_path)?;
    whisper_rs::install_logging_hooks();

    let text = tokio::task::spawn_blocking(move || {
        let model_path = model_path.to_string_lossy().into_owned();
        // WhisperContextParameters::default() sets use_gpu: cfg!(feature = "_gpu").
        // On macOS the binary is built with the metal feature, so GPU is used automatically.
        let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .map_err(|err| ModelError::LoadFailed(err.to_string()))?;
        tracing::info!(
            "local transcription backend: {}",
            crate::transcription::local_inference_backend_details()
        );
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

fn apply_model_options(config: &TranscriptionConfig, local_config: &mut LocalTranscriptionConfig) {
    if let Some(language) = config.option_string("language") {
        local_config.language = language.to_string();
    }
    if let Some(no_timestamps) = config.option_bool("no_timestamps") {
        local_config.no_timestamps = no_timestamps;
    }
    if let Some(no_context) = config.option_bool("no_context") {
        local_config.no_context = no_context;
    }
    if let Some(temperature) = config.option_number("temperature") {
        local_config.temperature = temperature as f32;
    }
    if let Some(entropy_thold) = config.option_number("entropy_thold") {
        local_config.entropy_thold = entropy_thold as f32;
    }
    if let Some(no_speech_thold) = config.option_number("no_speech_thold") {
        local_config.no_speech_thold = no_speech_thold as f32;
    }
}

async fn try_daemon_transcription(
    model_id: &str,
    audio_path: &Path,
    local_config: &LocalTranscriptionConfig,
) -> Option<String> {
    let Some(info) = probe_daemon().await else {
        tracing::info!(
            "local transcription mode: in-process (daemon not running, selected model '{}')",
            model_id
        );
        return None;
    };
    if info.model_id != model_id {
        tracing::info!(
            "local transcription mode: in-process (daemon loaded model '{}', selected model '{}')",
            info.model_id,
            model_id
        );
        return None;
    }

    tracing::info!("local transcription mode: daemon (model '{model_id}')");
    match request_transcription(audio_path, local_config).await {
        Ok(text) => {
            tracing::info!("local daemon transcription completed for model '{model_id}'");
            Some(text)
        }
        Err(err) => {
            tracing::warn!(
                "local daemon transcription failed for model '{model_id}', falling back in-process: {err}"
            );
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
            "Local transcription requires WAV audio: {err}. Configure [audio] with output_format = \"pcm_s16le -ar 16000\"."
        )
    })?;
    let spec = reader.spec();

    if spec.sample_format != hound::SampleFormat::Int
        || spec.bits_per_sample != 16
        || spec.sample_rate != 16_000
        || spec.channels != 1
    {
        anyhow::bail!(
            "Local transcription requires WAV signed 16-bit PCM, 16 kHz, mono audio. Configure [audio] with output_format = \"pcm_s16le -ar 16000\". Current file has format {:?}, {} bits, {} Hz, {} channel(s).",
            spec.sample_format,
            spec.bits_per_sample,
            spec.sample_rate,
            spec.channels
        );
    }

    Ok(())
}
