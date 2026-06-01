//! Display recent log entries from the application.

use anyhow::anyhow;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

const DEFAULT_LINES: usize = 50;

/// Shows recent log entries from the application logs.
///
/// Displays the most recent log entries from the current day's log file.
/// If the log file doesn't exist, shows an informative message.
///
/// # Errors
/// - If the log directory cannot be determined
/// - If log files cannot be read
pub fn handle_logs() -> Result<(), anyhow::Error> {
    let log_dir = crate::app_dirs::log_dir()?;

    if !log_dir.exists() {
        println!("Log directory does not exist yet: {}", log_dir.display());
        println!("Logs will be created when the application runs.");
        return Ok(());
    }

    // Find the most recent log file
    let log_file = find_latest_log(&log_dir)?;

    if !log_file.exists() {
        println!("No log files found in: {}", log_dir.display());
        println!("Run 'ostt' or other commands to generate logs.");
        return Ok(());
    }

    // Read and display the log file
    let content =
        fs::read_to_string(&log_file).map_err(|e| anyhow!("Failed to read log file: {e}"))?;

    if content.is_empty() {
        println!("Log file is empty: {}", log_file.display());
        return Ok(());
    }

    // Split into lines and show the last DEFAULT_LINES
    let lines: Vec<&str> = content.lines().collect();
    let start_index = if lines.len() > DEFAULT_LINES {
        lines.len() - DEFAULT_LINES
    } else {
        0
    };

    if start_index > 0 {
        println!("Showing last {} of {} lines:", DEFAULT_LINES, lines.len());
    } else {
        println!("Showing all {} lines:", lines.len());
    }
    println!("Full log file at: {}", log_file.display());
    println!();

    for line in lines[start_index..].iter() {
        println!("{line}");
    }

    Ok(())
}

pub fn handle_logs_path() -> Result<(), anyhow::Error> {
    let log_dir = crate::app_dirs::log_dir()?;
    let log_file = find_latest_log(&log_dir).unwrap_or_else(|_| log_dir.join("ostt.log"));
    println!("{}", log_file.display());
    Ok(())
}

pub fn handle_logs_follow() -> Result<(), anyhow::Error> {
    let log_dir = crate::app_dirs::log_dir()?;
    let log_file = find_latest_log(&log_dir)?;
    let mut file = fs::File::open(&log_file)
        .map_err(|e| anyhow!("Failed to open log file {}: {e}", log_file.display()))?;
    let mut position = file.seek(SeekFrom::End(0))?;

    loop {
        thread::sleep(Duration::from_millis(500));
        let len = file.metadata()?.len();
        if len < position {
            position = file.seek(SeekFrom::Start(0))?;
        }
        if len == position {
            continue;
        }

        file.seek(SeekFrom::Start(position))?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        print!("{buffer}");
        position = file.stream_position()?;
    }
}

/// Finds the latest (most recently modified) log file in the directory.
fn find_latest_log(log_dir: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let entries =
        fs::read_dir(log_dir).map_err(|e| anyhow!("Failed to read log directory: {e}"))?;

    let mut latest_file: Option<(PathBuf, std::time::SystemTime)> = None;

    for entry in entries {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        // Only consider files with ostt.log in their name
        if !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("ostt.log"))
        {
            continue;
        }

        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if latest_file.is_none() || modified > latest_file.as_ref().unwrap().1 {
                    latest_file = Some((path, modified));
                }
            }
        }
    }

    latest_file
        .map(|(path, _)| path)
        .ok_or_else(|| anyhow!("No log files found in {}", log_dir.display()))
}
