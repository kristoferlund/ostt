//! Time-domain waveform visualization.
//!
//! Displays audio amplitude over time as a scrolling waveform.

/// Updates waveform history with new volume sample.
///
/// Manages a scrolling buffer of volume values for time-domain display.
///
/// # Arguments
/// * `history` - Mutable reference to volume history buffer
/// * `current_volume` - Current volume (0-100)
/// * `max_width` - Maximum width of display (terminal width)
pub fn update_waveform(history: &mut Vec<u64>, current_volume: u8, max_width: usize) {
    history.push(current_volume as u64);
    
    if history.len() > max_width {
        history.remove(0);
    }
}

/// Resizes waveform history to match terminal width.
///
/// # Arguments
/// * `history` - Mutable reference to volume history buffer
/// * `target_width` - New terminal width
pub fn resize_waveform(history: &mut Vec<u64>, target_width: usize) {
    if history.len() > target_width {
        while history.len() > target_width {
            history.remove(0);
        }
    } else if history.len() < target_width {
        while history.len() < target_width {
            history.insert(0, 0);
        }
    }
}
