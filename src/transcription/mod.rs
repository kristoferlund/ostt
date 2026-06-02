//! Transcription service for audio-to-text conversion.
//!
//! This module provides support for multiple transcription providers and models through a
//! unified interface. Each provider has its own API endpoint and authentication method.

#[cfg(any(
    target_os = "macos",
    feature = "whisper-cuda",
    feature = "whisper-vulkan"
))]
use std::ffi::CStr;

pub mod animation;
pub mod api;
pub(crate) mod context;
pub mod daemon;
pub mod daemon_client;
pub mod local_models;
pub mod model;
pub mod provider;

pub use animation::TranscriptionAnimation;
pub use api::{transcribe, TranscriptionConfig, TranscriptionResponse};
pub(crate) use context::build_context;
pub use model::{all_models, find_model, models_for_provider, ModelOptionKind, ModelSpec};
pub use provider::TranscriptionProvider;

pub fn config_for_selected_model(
    selected_model: &crate::config::SelectedModel,
    api_key: Option<String>,
    keywords: Vec<String>,
    providers: crate::config::file::ProvidersConfig,
    model_options: indexmap::IndexMap<String, crate::config::ModelOptionValue>,
) -> anyhow::Result<TranscriptionConfig> {
    let provider =
        TranscriptionProvider::from_id(&selected_model.provider_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown provider '{}'. Supported providers: {}.",
                selected_model.provider_id,
                TranscriptionProvider::supported_ids().join(", ")
            )
        })?;

    if provider == TranscriptionProvider::Local {
        return Ok(TranscriptionConfig::new_local(
            selected_model.model_id.clone(),
            keywords,
            providers,
            model_options,
        ));
    }

    let model = find_model(&selected_model.provider_id, &selected_model.model_id).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown model '{}' for provider '{}'. Please run 'ostt model' to select a supported model.",
            selected_model.model_id,
            selected_model.provider_id
        )
    })?;

    let api_key = api_key.ok_or_else(|| {
        anyhow::anyhow!("No API key for {}. Please run 'ostt auth'", provider.name())
    })?;

    Ok(TranscriptionConfig::new_cloud(
        provider,
        selected_model.model_id.clone(),
        model.endpoint,
        api_key,
        keywords,
        providers,
        model_options,
    ))
}

pub(crate) fn local_inference_backend() -> &'static str {
    #[cfg(feature = "whisper-cuda")]
    {
        "CUDA GPU acceleration"
    }
    #[cfg(all(not(feature = "whisper-cuda"), feature = "whisper-vulkan"))]
    {
        "Vulkan GPU acceleration"
    }
    #[cfg(all(
        not(feature = "whisper-cuda"),
        not(feature = "whisper-vulkan"),
        target_os = "macos"
    ))]
    {
        "Metal GPU acceleration"
    }
    #[cfg(not(any(
        target_os = "macos",
        feature = "whisper-cuda",
        feature = "whisper-vulkan"
    )))]
    {
        "CPU inference"
    }
}

pub(crate) fn local_inference_backend_details() -> String {
    #[cfg(any(
        target_os = "macos",
        feature = "whisper-cuda",
        feature = "whisper-vulkan"
    ))]
    {
        let devices = local_gpu_devices();
        if devices.is_empty() {
            return format!(
                "{} requested, but no GPU devices were reported by ggml",
                local_inference_backend()
            );
        }
        let devices = devices
            .iter()
            .enumerate()
            .map(|(index, device)| format!("{index}: {device}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} using device(s): {devices}", local_inference_backend())
    }

    #[cfg(not(any(
        target_os = "macos",
        feature = "whisper-cuda",
        feature = "whisper-vulkan"
    )))]
    {
        local_inference_backend().to_string()
    }
}

#[cfg(any(
    target_os = "macos",
    feature = "whisper-cuda",
    feature = "whisper-vulkan"
))]
fn local_gpu_devices() -> Vec<String> {
    unsafe {
        let count = whisper_rs_sys::ggml_backend_dev_count();
        let mut devices = Vec::new();

        for index in 0..count {
            let device = whisper_rs_sys::ggml_backend_dev_get(index);
            let device_type = whisper_rs_sys::ggml_backend_dev_type(device);
            if device_type != whisper_rs_sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU
                && device_type
                    != whisper_rs_sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_IGPU
            {
                continue;
            }

            let description = whisper_rs_sys::ggml_backend_dev_description(device);
            if description.is_null() {
                devices.push("unknown GPU".to_string());
            } else {
                devices.push(CStr::from_ptr(description).to_string_lossy().into_owned());
            }
        }

        devices
    }
}
