use std::path::Path;

use super::TranscriptionConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
    Cpu,
    #[cfg(feature = "parakeet-cuda")]
    Cuda,
    #[cfg(feature = "parakeet-webgpu")]
    WebGpu,
}

impl Backend {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Cpu => "parakeet/cpu",
            #[cfg(feature = "parakeet-cuda")]
            Self::Cuda => "parakeet/cuda",
            #[cfg(feature = "parakeet-webgpu")]
            Self::WebGpu => "parakeet/webgpu",
        }
    }
}

pub(crate) fn backend_for_model(path: &Path) -> Backend {
    // Quantized exports are CPU models. Selecting one must keep working in the
    // same application even when GPU capabilities are compiled in.
    if path.join("encoder-model.int8.onnx").exists()
        || path.join("encoder-model.int4.onnx").exists()
    {
        return Backend::Cpu;
    }
    #[cfg(feature = "parakeet-webgpu")]
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Backend::WebGpu;
    }
    #[cfg(feature = "parakeet-cuda")]
    if cfg!(target_os = "linux") {
        return Backend::Cuda;
    }
    Backend::Cpu
}

pub(super) async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> anyhow::Result<String> {
    #[cfg(not(feature = "parakeet"))]
    {
        let _ = (config, audio_path);
        anyhow::bail!(
            "Parakeet support is not enabled in this build. Build with --features parakeet."
        );
    }
    #[cfg(feature = "parakeet")]
    {
        use crate::transcription::{daemon_client, local_models};
        let path = local_models::resolve_installed_model_path(&config.model_id)?;
        let samples = load_audio(audio_path)?;
        if !config.keywords.is_empty() {
            tracing::warn!(
                "Parakeet keyword boosting is not implemented; text replacements still apply"
            );
        }
        if daemon_client::probe_daemon()
            .await
            .is_some_and(|info| info.matches_model(&config.model_id))
        {
            // A reachable daemon's inference error is not a reason to silently retry on CPU.
            return daemon_client::request_transcription(audio_path, &Default::default()).await;
        }
        tokio::task::spawn_blocking(move || {
            let mut model = load_model(&path)?;
            infer(&mut model, &samples)
        })
        .await?
    }
}

#[cfg(feature = "parakeet")]
pub(crate) fn load_audio(path: &Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|err| anyhow::anyhow!("Parakeet requires 16 kHz mono PCM16 WAV audio: {err}"))?;
    let spec = reader.spec();
    if spec.sample_rate != 16000
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        anyhow::bail!("Parakeet requires 16 kHz mono PCM16 WAV audio. Convert with ffmpeg -i input -ar 16000 -ac 1 -c:a pcm_s16le output.wav");
    }
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|s| f32::from(s) / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    if samples.is_empty() {
        anyhow::bail!("audio is empty");
    }
    Ok(samples)
}

#[cfg(feature = "parakeet")]
pub(crate) fn load_model(path: &Path) -> anyhow::Result<parakeet_rs::ParakeetTDT> {
    use parakeet_rs::{ExecutionConfig, ParakeetTDT};
    let backend = backend_for_model(path);
    let encoder_config = match backend {
        Backend::Cpu => ExecutionConfig::default(),
        #[cfg(feature = "parakeet-cuda")]
        Backend::Cuda => ExecutionConfig::default().with_custom_configure(|builder| {
            Ok(builder
                .with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()])?)
        }),
        #[cfg(feature = "parakeet-webgpu")]
        Backend::WebGpu => ExecutionConfig::default().with_custom_configure(|builder| {
            Ok(builder.with_execution_providers([ort::ep::WebGPU::default()
                .build()
                .error_on_failure()])?)
        }),
    };
    tracing::info!(
        "Parakeet: loading encoder with {}, decoder/joint with CPU",
        backend.id()
    );
    ParakeetTDT::from_pretrained_with_joint_config(
        path,
        Some(encoder_config),
        Some(ExecutionConfig::default()),
    )
    .map_err(|err| anyhow::anyhow!("Failed to load Parakeet TDT model: {err}"))
}

#[cfg(feature = "parakeet")]
const WINDOW_SAMPLES: usize = 30 * 16000;
#[cfg(feature = "parakeet")]
const OVERLAP_SAMPLES: usize = 4 * 16000;

#[cfg(feature = "parakeet")]
fn owns_word(start: f32, first: bool, last: bool) -> bool {
    (first || start >= 2.0) && (last || start < 28.0)
}

#[cfg(feature = "parakeet")]
pub(crate) fn infer(
    model: &mut parakeet_rs::ParakeetTDT,
    samples: &[f32],
) -> anyhow::Result<String> {
    use parakeet_rs::{TimestampMode, Transcriber};
    let mut text = String::new();
    let mut start = 0;
    loop {
        let end = (start + WINDOW_SAMPLES).min(samples.len());
        let last = end == samples.len();
        let result = model
            .transcribe_samples(
                samples[start..end].to_vec(),
                16000,
                1,
                Some(TimestampMode::Words),
            )
            .map_err(|err| anyhow::anyhow!("Parakeet transcription failed: {err}"))?;
        for word in result.tokens {
            if owns_word(word.start, start == 0, last) {
                if !text.is_empty()
                    && !word
                        .text
                        .chars()
                        .all(|c| matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | ')'))
                {
                    text.push(' ');
                }
                text.push_str(&word.text);
            }
        }
        if last {
            break;
        }
        start += WINDOW_SAMPLES - OVERLAP_SAMPLES;
    }
    Ok(text.trim().to_string())
}

#[cfg(all(test, feature = "parakeet"))]
mod tests {
    use super::*;

    #[test]
    fn int8_models_stay_on_cpu_in_the_same_gpu_capable_application() {
        let path = std::env::temp_dir().join(format!(
            "ostt-backend-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("encoder-model.int8.onnx"), b"model").unwrap();
        assert_eq!(backend_for_model(&path), Backend::Cpu);
        std::fs::remove_file(path.join("encoder-model.int8.onnx")).unwrap();
        std::fs::remove_dir(path).unwrap();
    }

    #[test]
    fn fp16_models_use_only_a_backend_supported_by_the_build_and_platform() {
        let backend = backend_for_model(Path::new("/nonexistent-fp16-model"));
        if cfg!(all(
            feature = "parakeet-webgpu",
            target_os = "macos",
            target_arch = "aarch64"
        )) {
            assert_eq!(backend.id(), "parakeet/webgpu");
        } else if cfg!(all(feature = "parakeet-cuda", target_os = "linux")) {
            assert_eq!(backend.id(), "parakeet/cuda");
        } else {
            assert_eq!(backend.id(), "parakeet/cpu");
        }
    }

    #[test]
    fn overlap_ownership_has_no_gap_or_duplicate_at_the_boundary() {
        // Windows start 26 seconds apart. The midpoint at global 28s belongs
        // to the second window, retaining each whole word exactly once.
        for global in [27.99_f32, 28.0, 28.01, 29.0] {
            let owners = usize::from(owns_word(global, true, false))
                + usize::from(owns_word(global - 26.0, false, true));
            assert_eq!(owners, 1, "global time {global}");
        }
        assert!(owns_word(0.0, true, false));
        assert!(owns_word(29.9, false, true));
    }
}
