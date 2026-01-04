//! Audio recording and format conversion module.
//!
//! This module handles audio input device management, PCM sample capture, and
//! format conversion using ffmpeg. Audio is captured from the system's default
//! input device, converted to mono, and saved in the requested format.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use super::ffmpeg::find_ffmpeg;

#[cfg(target_os = "linux")]
use std::fs::OpenOptions;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

/// Records audio from a specified or default input device.
///
/// Features:
/// - Captures from a specified input device or system default at its native sample rate
/// - Converts multi-channel audio to mono by averaging channels
/// - Saves audio via ffmpeg for format flexibility
/// - Automatic cleanup of temporary files
/// - Pause and resume support
pub struct AudioRecorder {
    /// Actual recording sample rate from device
    sample_rate: u32,
    /// Recorded audio samples (i16 PCM mono)
    samples: Arc<Mutex<Vec<i16>>>,
    /// Active audio input stream (kept alive during recording)
    stream: Option<cpal::Stream>,
    /// Number of channels in device's native format
    device_channels: usize,
    /// Whether recording is currently paused
    is_paused: Arc<Mutex<bool>>,
    /// Device name or "default" to use the system default device
    device_name: String,
}

impl AudioRecorder {
    /// Creates a new audio recorder with requested sample rate and device.
    ///
    /// # Arguments
    /// * `requested_sample_rate` - The desired sample rate in Hz (actual may differ based on device)
    /// * `device_name` - Device name/ID to use. Use "default" for system default device
    ///
    /// Note: The actual recording sample rate may differ based on device capabilities.
    /// Call `get_sample_rate()` after `start_recording()` to get the actual rate.
    pub fn new(requested_sample_rate: u32, device_name: String) -> Self {
        Self {
            sample_rate: requested_sample_rate,
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            device_channels: 1,
            is_paused: Arc::new(Mutex::new(false)),
            device_name,
        }
    }

    /// Starts recording from the configured input device.
    ///
    /// # Errors
    /// - If the specified device is not available
    /// - If device configuration fails
    /// - If audio stream creation fails
    pub fn start_recording(&mut self) -> Result<()> {
        // Get device while suppressing ALSA library warnings
        let device = suppress_alsa_warnings(|| {
            let host = cpal::default_host();

            if self.device_name == "default" {
                host.default_input_device()
                    .ok_or_else(|| anyhow!("No audio input device available"))
            } else {
                // Try to find device by name or index
                find_device_by_name(&host, &self.device_name)
            }
        })?;

        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown device".to_string());
        tracing::info!("Recording device: {}", device_name);

        let device_config = device.default_input_config()?;
        let device_sample_rate = device_config.sample_rate().0;
        let num_channels = device_config.channels() as usize;

        // Warn if requested sample rate doesn't match device
        if device_sample_rate != self.sample_rate {
            tracing::warn!(
                "Requested sample rate {}Hz but device uses {}Hz. Recording at device rate.",
                self.sample_rate,
                device_sample_rate
            );
        }

        tracing::debug!(
            "Device configuration: {}Hz, {} channels",
            device_sample_rate,
            num_channels
        );

        // Update to actual device parameters
        self.sample_rate = device_sample_rate;
        self.device_channels = num_channels;

        // Set up audio callback with cloned Arc references
        let samples_arc = Arc::clone(&self.samples);
        let pause_arc = Arc::clone(&self.is_paused);
        let callback_channels = num_channels;

        let stream = device.build_input_stream(
            &device_config.into(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let is_paused = *pause_arc.lock().unwrap();
                if !is_paused {
                    Self::handle_audio_callback(data, &samples_arc, callback_channels);
                }
            },
            |err| {
                tracing::error!("Audio stream error: {}", err);
            },
            None,
        )?;

        // Start playback and store stream
        stream.play()?;
        self.stream = Some(stream);

        tracing::debug!("Audio stream started");
        Ok(())
    }

    /// Stops recording and saves audio to the specified output file.
    ///
    /// The audio is first saved as a temporary WAV file, then converted to the
    /// requested format using ffmpeg. The temporary file is cleaned up after conversion.
    ///
    /// # Arguments
    /// * `output_path` - Path where the final encoded audio will be saved
    /// * `format` - ffmpeg codec and options, e.g., "mp3 -ab 16k -ar 12000"
    ///
    /// # Errors
    /// - If no samples were recorded
    /// - If temporary WAV creation fails
    /// - If ffmpeg conversion fails
    pub fn stop_recording(&mut self, output_path: Option<PathBuf>, format: &str) -> Result<()> {
        // Stop the audio stream
        self.stream = None;

        let samples = self.samples.lock().unwrap().clone();
        let sample_count = samples.len();

        if sample_count == 0 {
            tracing::warn!("Recording stopped with no samples captured");
            return Ok(());
        }

        // Calculate and log recording duration
        let duration_secs = sample_count as f32 / self.sample_rate as f32;
        tracing::info!(
            "Recording stopped: {:.2}s ({} samples at {}Hz)",
            duration_secs,
            sample_count,
            self.sample_rate
        );

        // Save and convert to desired format
        if let Some(output_file) = output_path {
            let temp_wav = self.create_temp_wav_path();

            self.save_wav(&samples, &temp_wav)?;
            self.convert_with_ffmpeg(&temp_wav, &output_file, format)?;

            // Clean up temporary file
            if let Err(e) = std::fs::remove_file(&temp_wav) {
                tracing::debug!("Failed to remove temp file: {}", e);
            }

            // Log final file info
            let file_size = std::fs::metadata(&output_file)?.len();
            tracing::info!(
                "Audio saved: {} ({} bytes, format: {})",
                output_file.display(),
                file_size,
                format
            );
        }

        Ok(())
    }

    /// Handles incoming audio data from the audio callback.
    ///
    /// Converts multi-channel audio to mono by averaging all channels.
    fn handle_audio_callback(
        data: &[i16],
        samples_arc: &Arc<Mutex<Vec<i16>>>,
        num_channels: usize,
    ) {
        let mut samples = samples_arc.lock().unwrap();

        match num_channels {
            1 => {
                // Mono: use samples directly
                samples.extend_from_slice(data);
            }
            2 => {
                // Stereo: average pairs of samples
                for chunk in data.chunks_exact(2) {
                    let left = chunk[0] as i32;
                    let right = chunk[1] as i32;
                    let mono = ((left + right) / 2) as i16;
                    samples.push(mono);
                }
            }
            _ => {
                // Multi-channel: average all channels per sample
                for chunk in data.chunks_exact(num_channels) {
                    let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                    let mono = (sum / num_channels as i32) as i16;
                    samples.push(mono);
                }
            }
        }
    }

    /// Saves audio samples as a temporary WAV file.
    ///
    /// This creates an uncompressed PCM WAV intermediate file that will be
    /// converted to the desired format by ffmpeg.
    fn save_wav(&self, samples: &[i16], path: &Path) -> Result<()> {
        let wav_spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, wav_spec)?;

        for &sample in samples {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        tracing::debug!("Temporary WAV created: {}", path.display());
        Ok(())
    }

    /// Converts audio using ffmpeg based on format string.
    ///
    /// # Arguments
    /// * `input_wav` - Path to temporary WAV file
    /// * `output_path` - Final output file path
    /// * `format` - Format string: "codec [options]", e.g., "mp3 -ab 16k -ar 12000"
    ///
    /// The format string is parsed to extract the codec and any additional ffmpeg
    /// arguments. Mono conversion is always enforced.
    fn convert_with_ffmpeg(
        &self,
        input_wav: &Path,
        output_path: &Path,
        format: &str,
    ) -> Result<()> {
        // Parse codec and additional options from format string
        let format_parts: Vec<&str> = format.split_whitespace().collect();

        if format_parts.is_empty() {
            return Err(anyhow!("Invalid format string: empty"));
        }

        let codec = format_parts[0];

        // Find ffmpeg binary with cross-platform support
        let ffmpeg_path = find_ffmpeg()?;

        // Build ffmpeg command
        let mut cmd = Command::new(&ffmpeg_path);
        cmd.arg("-loglevel")
            .arg("error")
            .arg("-i")
            .arg(input_wav)
            .arg("-acodec")
            .arg(codec)
            .arg("-ac")
            .arg("1") // Force mono
            .arg("-y"); // Overwrite output

        // Add any additional ffmpeg options from format string
        for option in &format_parts[1..] {
            cmd.arg(option);
        }

        cmd.arg(output_path);

        // Execute ffmpeg
        let output = cmd.output()?;

        if output.status.success() {
            tracing::debug!("Audio converted to {} format", codec);
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            tracing::error!("ffmpeg conversion failed: {}", error_msg);
            Err(anyhow!("Audio encoding failed: {error_msg}"))
        }
    }

    /// Creates a path for the temporary WAV file.
    fn create_temp_wav_path(&self) -> PathBuf {
        std::env::temp_dir().join(format!("ostt_{}.wav", std::process::id()))
    }

    // Getters for recorded data

    /// Returns a clone of all recorded samples.
    pub fn samples(&self) -> Vec<i16> {
        self.samples.lock().unwrap().clone()
    }

    /// Returns the number of recorded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    /// Returns the actual sample rate of the recording.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Pauses recording without stopping the audio stream or losing samples.
    pub fn pause(&self) {
        *self.is_paused.lock().unwrap() = true;
        tracing::debug!("Recording paused");
    }

    /// Resumes recording from a paused state.
    pub fn resume(&self) {
        *self.is_paused.lock().unwrap() = false;
        tracing::debug!("Recording resumed");
    }

    /// Returns whether recording is currently paused.
    pub fn is_paused(&self) -> bool {
        *self.is_paused.lock().unwrap()
    }

    /// Toggles between paused and recording states.
    pub fn toggle_pause(&self) {
        let mut paused = self.is_paused.lock().unwrap();
        *paused = !*paused;
        if *paused {
            tracing::debug!("Recording paused");
        } else {
            tracing::debug!("Recording resumed");
        }
    }
}

// Maintain backward compatibility with existing API
impl AudioRecorder {
    /// Deprecated: Use `samples()` instead.
    pub fn get_samples(&self) -> Vec<i16> {
        self.samples()
    }

    /// Deprecated: Use `sample_rate()` instead.
    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate()
    }
}

/// Finds an audio input device by name or numeric index.
///
/// # Arguments
/// * `host` - The cpal audio host
/// * `device_spec` - Either "default" for system default, a device name, or a numeric index (0, 1, 2, etc.)
///
/// # Errors
/// - If no device with the specified name/index is found
fn find_device_by_name(
    host: &cpal::Host,
    device_spec: &str,
) -> Result<cpal::Device> {
    // Try to parse as a numeric index first
    if let Ok(index) = device_spec.parse::<usize>() {
        let devices: Vec<_> = host
            .input_devices()
            .map_err(|e| anyhow!("Failed to enumerate devices: {e}"))?
            .collect();

        if index < devices.len() {
            return Ok(devices.into_iter().nth(index).unwrap());
        } else {
            return Err(anyhow!(
                "Device index {} is out of range (0-{})",
                index,
                devices.len().saturating_sub(1)
            ));
        }
    }

    // Try to find by name
    let devices = host
        .input_devices()
        .map_err(|e| anyhow!("Failed to enumerate devices: {e}"))?;

    for device in devices {
        if let Ok(name) = device.name() {
            if name == device_spec {
                return Ok(device);
            }
        }
    }

    Err(anyhow!(
        "Audio input device '{device_spec}' not found. Use 'ostt list-devices' to see available devices."
    ))
}

/// Temporarily redirects stderr to /dev/null to suppress ALSA library warnings on Linux.
/// On non-Linux platforms, this is a no-op since ALSA doesn't exist.
#[cfg(target_os = "linux")]
fn suppress_alsa_warnings<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    // Open /dev/null for writing
    let dev_null = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|e| anyhow!("Failed to open /dev/null: {e}"))?;

    let dev_null_fd = dev_null.as_raw_fd();

    // Save the current stderr file descriptor
    let old_stderr = unsafe { libc::dup(libc::STDERR_FILENO) };
    if old_stderr == -1 {
        return Err(anyhow!("Failed to duplicate stderr"));
    }

    // Redirect stderr to /dev/null
    let redirect_result = unsafe { libc::dup2(dev_null_fd, libc::STDERR_FILENO) };
    if redirect_result == -1 {
        unsafe { libc::close(old_stderr) };
        return Err(anyhow!("Failed to redirect stderr"));
    }

    // Execute the closure
    let result = f();

    // Restore the original stderr
    unsafe {
        libc::dup2(old_stderr, libc::STDERR_FILENO);
        libc::close(old_stderr);
    }

    result
}

/// On non-Linux platforms, no stderr suppression is needed since ALSA doesn't exist.
#[cfg(not(target_os = "linux"))]
fn suppress_alsa_warnings<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    f()
}
