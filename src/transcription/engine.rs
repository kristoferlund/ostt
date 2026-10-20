//! Local inference through transcribe.cpp, shared by in-process and daemon
//! transcription.

use std::path::Path;
use std::sync::Once;

use transcribe_cpp::{
    Backend, Error, Model, ModelOptions, RunExtension, RunOptions, WhisperRunOptions,
};

use crate::config::LocalTranscriptionConfig;
use crate::transcription::local_models::ModelError;

/// Loads a local model on the build's preferred backend (GPU when compiled in
/// and available, CPU otherwise).
pub(crate) fn load_model(path: &Path) -> Result<Model, ModelError> {
    init();
    Model::load(path).map_err(|err| ModelError::LoadFailed(err.to_string()))
}

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Route native and ggml diagnostics into the tracing log instead of stderr.
        transcribe_cpp::init_logging();
        // Logged before loading so a failed GPU load still shows what was available.
        let devices = transcribe_cpp::devices()
            .iter()
            .map(|device| format!("{} ({})", device.kind, device.description))
            .collect::<Vec<_>>()
            .join(", ");
        tracing::info!("local inference devices: {devices}");
    });
}

/// Identifies the local provider for a downloaded model file from its
/// architecture. Loading the model also verifies the file is usable.
pub(crate) fn provider_for_model_file(path: &Path) -> anyhow::Result<&'static str> {
    init();
    // CPU only: detection must not compete with a running daemon for GPU memory.
    let options = ModelOptions {
        backend: Backend::Cpu,
        ..Default::default()
    };
    let model =
        Model::load_with(path, &options).map_err(|err| ModelError::LoadFailed(err.to_string()))?;
    provider_for_arch(&model.arch())
}

fn provider_for_arch(arch: &str) -> anyhow::Result<&'static str> {
    match arch {
        "whisper" => Ok("whisper"),
        "parakeet" => Ok("parakeet"),
        other => anyhow::bail!(
            "Unsupported local model type '{other}'. Supported types: Whisper and Parakeet."
        ),
    }
}

/// Transcribes 16 kHz mono samples with a loaded model.
pub(crate) fn transcribe(
    model: &Model,
    samples: &[f32],
    config: &LocalTranscriptionConfig,
) -> anyhow::Result<String> {
    let mut session = model
        .session()
        .map_err(|err| anyhow::anyhow!("Failed to create transcription session: {err}"))?;
    let arch = model.arch();
    let transcript = match session.run(samples, &run_options(&arch, config)) {
        Ok(transcript) => transcript,
        // The engine stopped early but kept what it decoded; losing a whole
        // recording would be worse than returning an incomplete transcript.
        Err(
            ref err @ (Error::OutputTruncated {
                partial: Some(ref partial),
                ..
            }
            | Error::OutputRepetition {
                partial: Some(ref partial),
                ..
            }),
        ) => {
            tracing::warn!("returning partial transcript: {err}");
            (**partial).clone()
        }
        Err(err) => anyhow::bail!("Transcription failed: {err}"),
    };
    let text = transcript.text.trim();
    if arch == "whisper" {
        // Whisper invents text such as "[BLANK_AUDIO]" for silence.
        return Ok(
            crate::transcription::api::local::filter_obvious_hallucination(text)
                .unwrap_or_default(),
        );
    }
    Ok(text.to_string())
}

/// Describes the backend and device a loaded model runs on, for logs and status.
pub(crate) fn backend_details(model: &Model) -> String {
    let device = match model.device() {
        Ok(device) if device.memory_total > 0 => format!(
            "{}, {} MB",
            device.description,
            device.memory_total / (1024 * 1024)
        ),
        Ok(device) => device.description,
        Err(_) => "unknown device".to_string(),
    };
    format!("{} model on {} ({device})", model.arch(), model.backend())
}

fn run_options(arch: &str, config: &LocalTranscriptionConfig) -> RunOptions {
    RunOptions {
        language: (config.language != "auto").then(|| config.language.clone()),
        // Other model families reject or ignore Whisper decode settings.
        family: (arch == "whisper").then(|| {
            RunExtension::Whisper(WhisperRunOptions {
                condition_on_prev_tokens: Some(!config.no_context),
                temperature: Some(config.temperature),
                no_speech_thold: Some(config.no_speech_thold),
                ..Default::default()
            })
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whisper_options(options: &RunOptions) -> &WhisperRunOptions {
        match options.family.as_ref() {
            Some(RunExtension::Whisper(whisper)) => whisper,
            other => panic!("expected whisper options, got {other:?}"),
        }
    }

    #[test]
    fn auto_language_lets_the_model_detect_the_language() {
        let options = run_options("whisper", &LocalTranscriptionConfig::default());
        assert_eq!(options.language, None);
    }

    #[test]
    fn explicit_language_is_passed_as_a_hint() {
        let config = LocalTranscriptionConfig {
            language: "sv".to_string(),
            ..Default::default()
        };
        assert_eq!(
            run_options("whisper", &config).language.as_deref(),
            Some("sv")
        );
    }

    #[test]
    fn user_decoding_params_keep_their_whisper_meaning() {
        let config = LocalTranscriptionConfig {
            no_context: true,
            temperature: 0.3,
            no_speech_thold: 0.5,
            ..Default::default()
        };
        let options = run_options("whisper", &config);
        let whisper = whisper_options(&options);
        // no_context=true must stop conditioning on earlier segments, or a
        // hallucinated segment can propagate through the rest of a recording.
        assert_eq!(whisper.condition_on_prev_tokens, Some(false));
        assert_eq!(whisper.temperature, Some(0.3));
        assert_eq!(whisper.no_speech_thold, Some(0.5));
    }

    #[test]
    fn custom_models_are_filed_under_the_provider_matching_their_architecture() {
        assert_eq!(provider_for_arch("whisper").unwrap(), "whisper");
        assert_eq!(provider_for_arch("parakeet").unwrap(), "parakeet");
        // Other families load in the engine but have no provider, params or
        // UI support yet, so they must be refused rather than filed as Whisper.
        assert!(provider_for_arch("moonshine").is_err());
    }

    #[test]
    fn whisper_settings_are_not_sent_to_other_model_families() {
        // Other families reject a Whisper extension, which would fail every run.
        let options = run_options("parakeet", &LocalTranscriptionConfig::default());
        assert!(options.family.is_none());
    }
}
