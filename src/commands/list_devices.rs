//! List available audio input devices.

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(target_os = "linux")]
use std::fs::OpenOptions;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

/// Lists all available audio input devices on the system.
///
/// # Errors
/// - If the audio host cannot be initialized
pub fn handle_list_devices() -> Result<(), anyhow::Error> {
    // Enumerate devices while suppressing ALSA library warnings
    let (host, device_results) = suppress_stderr(|| {
        let host = cpal::default_host();
        let device_iter = host
            .input_devices()
            .map_err(|e| anyhow!("Failed to enumerate audio devices: {e}"))?;
        
        // Collect devices, skipping any that fail to query
        let devices: Vec<cpal::Device> = device_iter
            .filter_map(|d| {
                // Test if we can get the device name without crashing
                match d.name() {
                    Ok(_) => Some(d),
                    Err(_) => None,
                }
            })
            .collect();
        
        Ok((host, devices))
    })?;

    if device_results.is_empty() {
        println!("No audio input devices found on this system.");
        return Ok(());
    }

    println!();
    println!(" ┏┓┏╋╋ ");
    println!(" ┗┛┛┗┗ ");
    println!();
    println!("Available audio input devices:");
    println!();

    // Find the default device
    let default_device = host
        .default_input_device()
        .and_then(|d| d.name().ok());

    for (index, device) in device_results.iter().enumerate() {
        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let is_default = default_device.as_ref() == Some(&device_name);

        let default_indicator = if is_default { " [DEFAULT]" } else { "" };

        // Get configuration info
        let config_info = match device.default_input_config() {
            Ok(config) => {
                let sample_rate = config.sample_rate().0;
                let channels = config.channels();
                format!(" ({}Hz, {} channels)", sample_rate, channels)
            }
            Err(_) => {
                " (configuration unavailable)".to_string()
            }
        };

        println!("  ID: {}", index);
        println!("    Name: {}{}", device_name, default_indicator);
        println!("    Config:{}", config_info);
        println!();
    }

    Ok(())
}

/// Temporarily redirects stderr to /dev/null to suppress ALSA library warnings on Linux.
/// On non-Linux platforms, this is a no-op since ALSA doesn't exist.
#[cfg(target_os = "linux")]
fn suppress_stderr<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce() -> anyhow::Result<T>,
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
fn suppress_stderr<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    f()
}
