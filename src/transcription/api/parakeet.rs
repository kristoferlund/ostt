//! Local Parakeet model inference for offline transcription using sherpa-onnx.
//!
//! Handles transcription using locally-stored ONNX models from NVIDIA's Parakeet TDT family.
//! Supports both English-only (v2) and multilingual (v3) models with no API required.
//! Uses sherpa-rs bindings for sherpa-onnx format compatibility.

use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

use super::TranscriptionConfig;
use crate::transcription::TranscriptionModel;

/// Returns the model directory path for a given Parakeet model.
///
/// Models are stored in ~/.config/ostt/models/<model-name>/
fn get_model_path(model: &TranscriptionModel) -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;

    let model_name = match model {
        TranscriptionModel::ParakeetTdtV2 => "parakeet-tdt-v2",
        TranscriptionModel::ParakeetTdtV3 => "parakeet-tdt-v3",
        _ => return Err(anyhow!("Invalid Parakeet model")),
    };

    Ok(home_dir
        .join(".config")
        .join("ostt")
        .join("models")
        .join(model_name))
}

/// Checks if the required model files exist in the model directory.
///
/// Required files for sherpa-onnx TDT models:
/// - encoder.int8.onnx (or encoder.onnx for FP32)
/// - decoder.int8.onnx (or decoder.onnx for FP32)
/// - joiner.int8.onnx (or joiner.onnx for FP32)
/// - tokens.txt (or vocab.txt)
fn verify_model_files(model_dir: &Path) -> Result<(String, String, String, String)> {
    // Check for encoder (prefer quantized, fall back to FP32)
    let encoder = if model_dir.join("encoder.int8.onnx").exists() {
        model_dir.join("encoder.int8.onnx")
    } else if model_dir.join("encoder.onnx").exists() {
        model_dir.join("encoder.onnx")
    } else if model_dir.join("encoder-model.int8.onnx").exists() {
        model_dir.join("encoder-model.int8.onnx")
    } else if model_dir.join("encoder-model.onnx").exists() {
        model_dir.join("encoder-model.onnx")
    } else {
        return Err(anyhow!(
            "Encoder model not found in {}. Expected one of:\n  - encoder.int8.onnx\n  - encoder.onnx\n  - encoder-model.int8.onnx\n  - encoder-model.onnx",
            model_dir.display()
        ));
    };

    // Check for decoder (prefer quantized, fall back to FP32)
    let decoder = if model_dir.join("decoder.int8.onnx").exists() {
        model_dir.join("decoder.int8.onnx")
    } else if model_dir.join("decoder.onnx").exists() {
        model_dir.join("decoder.onnx")
    } else if model_dir.join("decoder-model.int8.onnx").exists() {
        model_dir.join("decoder-model.int8.onnx")
    } else if model_dir.join("decoder-model.onnx").exists() {
        model_dir.join("decoder-model.onnx")
    } else {
        return Err(anyhow!(
            "Decoder model not found in {}. Expected one of:\n  - decoder.int8.onnx\n  - decoder.onnx\n  - decoder-model.int8.onnx\n  - decoder-model.onnx",
            model_dir.display()
        ));
    };

    // Check for joiner (prefer quantized, fall back to FP32)
    let joiner = if model_dir.join("joiner.int8.onnx").exists() {
        model_dir.join("joiner.int8.onnx")
    } else if model_dir.join("joiner.onnx").exists() {
        model_dir.join("joiner.onnx")
    } else if model_dir.join("joiner-model.int8.onnx").exists() {
        model_dir.join("joiner-model.int8.onnx")
    } else if model_dir.join("joiner-model.onnx").exists() {
        model_dir.join("joiner-model.onnx")
    } else {
        return Err(anyhow!(
            "Joiner model not found in {}. Expected one of:\n  - joiner.int8.onnx\n  - joiner.onnx\n  - joiner-model.int8.onnx\n  - joiner-model.onnx",
            model_dir.display()
        ));
    };

    // Check for tokens/vocab file
    let tokens = if model_dir.join("tokens.txt").exists() {
        model_dir.join("tokens.txt")
    } else if model_dir.join("vocab.txt").exists() {
        model_dir.join("vocab.txt")
    } else {
        return Err(anyhow!(
            "Tokens file not found in {}. Expected one of:\n  - tokens.txt\n  - vocab.txt",
            model_dir.display()
        ));
    };

    Ok((
        encoder.to_string_lossy().to_string(),
        decoder.to_string_lossy().to_string(),
        joiner.to_string_lossy().to_string(),
        tokens.to_string_lossy().to_string(),
    ))
}

/// Transcribes an audio file using a local Parakeet model via sherpa-onnx.
///
/// This function performs offline inference using ONNX Runtime with the specified
/// Parakeet TDT model. No internet connection or API key required.
///
/// # Errors
/// - If the model directory doesn't exist
/// - If required model files are missing
/// - If the audio file cannot be read
/// - If inference fails
pub async fn transcribe(
    config: &TranscriptionConfig,
    audio_path: &Path,
) -> Result<String> {
    let model_dir = get_model_path(&config.model)?;

    // Debug to file since TUI captures stderr
    let debug_log = format!(
        "DEBUG LOG:\nModel path: {}\nExists: {}\nModel variant: {:?}\n",
        model_dir.display(),
        model_dir.exists(),
        config.model
    );
    let _ = std::fs::write("/tmp/ostt_debug.log", &debug_log);

    // Check if model directory exists
    if !model_dir.exists() {
        // Write more debug info
        let parent_info = if let Some(parent) = model_dir.parent() {
            if parent.exists() {
                if let Ok(entries) = std::fs::read_dir(parent) {
                    let dirs: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect();
                    format!("Parent exists. Contents: {:?}", dirs)
                } else {
                    "Parent exists but can't read".to_string()
                }
            } else {
                "Parent doesn't exist".to_string()
            }
        } else {
            "No parent".to_string()
        };

        let _ = std::fs::write("/tmp/ostt_debug.log", format!("{}\n{}", &debug_log, parent_info));

        return Err(anyhow!(
            "Model directory not found: {}\n\nPlease download the model first. See README for instructions.",
            model_dir.display()
        ));
    }

    // Verify all required model files exist and get paths
    let (encoder_path, decoder_path, joiner_path, tokens_path) = verify_model_files(&model_dir)?;

    tracing::info!(
        "Loading Parakeet model from: {}",
        model_dir.display()
    );
    tracing::debug!(
        "Model files - encoder: {}, decoder: {}, joiner: {}, tokens: {}",
        encoder_path,
        decoder_path,
        joiner_path,
        tokens_path
    );

    // Use sherpa-rs for transcription
    use sherpa_rs::{read_audio_file, transducer::{TransducerConfig as SherpaConfig, TransducerRecognizer}};

    // Read audio file (sherpa-rs handles resampling to 16kHz if needed)
    tracing::info!(
        "Loading audio file: {}",
        audio_path.display()
    );

    let (samples, sample_rate) = read_audio_file(audio_path.to_str()
        .ok_or_else(|| anyhow!("Invalid audio path"))?)
        .map_err(|e| anyhow!("Failed to read audio file: {:?}", e))?;

    tracing::info!(
        "Audio loaded: {} samples at {}Hz",
        samples.len(),
        sample_rate
    );

    // Verify sample rate (sherpa-onnx expects 16kHz)
    if sample_rate != 16000 {
        tracing::warn!(
            "Audio sample rate is {}Hz, but model expects 16kHz. Transcription may fail or produce incorrect results.",
            sample_rate
        );
    }

    // Create transducer recognizer configuration
    // Use more threads for faster CPU inference
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
        .max(4); // At least 4 threads

    tracing::info!("Using {} threads for inference", num_threads);

    let recognizer_config = SherpaConfig {
        encoder: encoder_path,
        decoder: decoder_path,
        joiner: joiner_path,
        tokens: tokens_path,
        num_threads,
        sample_rate: 16000,
        feature_dim: 80,
        model_type: "nemo_transducer".to_string(),
        debug: false,
        ..Default::default()
    };

    tracing::info!("Creating transducer recognizer...");
    let start_load = std::time::Instant::now();
    let mut recognizer = TransducerRecognizer::new(recognizer_config)
        .map_err(|e| anyhow!("Failed to create recognizer: {:?}", e))?;
    tracing::info!("Model loaded in {:?}", start_load.elapsed());

    tracing::info!("Transcribing audio...");
    let start_transcribe = std::time::Instant::now();
    let result = recognizer.transcribe(sample_rate, &samples);
    tracing::info!("Transcription completed in {:?}", start_transcribe.elapsed());

    // Trim whitespace from result
    Ok(result.trim().to_string())
}
