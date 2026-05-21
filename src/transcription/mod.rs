//! Transcription service for audio-to-text conversion.
//!
//! This module provides support for multiple transcription providers and models through a
//! unified interface. Each provider has its own API endpoint and authentication method.

#[cfg(any(target_os = "macos", feature = "whisper-cuda", feature = "whisper-vulkan"))]
use std::ffi::CStr;

pub mod animation;
pub mod api;
pub mod daemon;
pub mod daemon_client;
pub mod local_models;
pub mod model;
pub mod provider;

pub use animation::TranscriptionAnimation;
pub use api::{transcribe, TranscriptionConfig, TranscriptionResponse};
pub use model::TranscriptionModel;
pub use provider::TranscriptionProvider;

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
    #[cfg(not(any(target_os = "macos", feature = "whisper-cuda", feature = "whisper-vulkan")))]
    {
        "CPU inference"
    }
}

pub(crate) fn local_inference_backend_details() -> String {
    #[cfg(any(target_os = "macos", feature = "whisper-cuda", feature = "whisper-vulkan"))]
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

    #[cfg(not(any(target_os = "macos", feature = "whisper-cuda", feature = "whisper-vulkan")))]
    {
        local_inference_backend().to_string()
    }
}

#[cfg(any(target_os = "macos", feature = "whisper-cuda", feature = "whisper-vulkan"))]
fn local_gpu_devices() -> Vec<String> {
    unsafe {
        let count = whisper_rs_sys::ggml_backend_dev_count();
        let mut devices = Vec::new();

        for index in 0..count {
            let device = whisper_rs_sys::ggml_backend_dev_get(index);
            let device_type = whisper_rs_sys::ggml_backend_dev_type(device);
            if device_type != whisper_rs_sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU
                && device_type != whisper_rs_sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_IGPU
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
