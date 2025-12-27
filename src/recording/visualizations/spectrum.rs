//! Frequency spectrum visualization using FFT.
//!
//! Displays audio energy distribution across frequency bands in the human voice range.

use rustfft::{FftPlanner, num_complex::Complex};

/// Stateful spectrum analyzer with internal FFT planner.
pub struct SpectrumAnalyzer {
    fft_planner: FftPlanner<f32>,
    display_data: Vec<u64>,
    num_bins: usize,
}

impl SpectrumAnalyzer {
    /// Creates a new spectrum analyzer.
    pub fn new(num_bins: usize) -> Self {
        Self {
            fft_planner: FftPlanner::new(),
            display_data: vec![0u64; num_bins],
            num_bins,
        }
    }

    /// Updates spectrum with new samples, applying smoothing.
    pub fn update(&mut self, samples: &[i16], sample_rate: u32, reference_level_db: i8) {
        let new_bins = calculate_spectrum(
            samples,
            sample_rate,
            self.num_bins,
            reference_level_db,
            &mut self.fft_planner,
        );

        // Apply moving average smoothing to reduce visual jitter
        for (old_val, new_val) in self.display_data.iter_mut().zip(new_bins.iter()) {
            *old_val = (*old_val + *new_val) / 2;
        }
    }

    /// Resizes the analyzer for a new terminal width.
    pub fn resize(&mut self, new_width: usize, samples: &[i16], sample_rate: u32, reference_level_db: i8) {
        self.num_bins = new_width;
        if !samples.is_empty() {
            self.display_data = calculate_spectrum(
                samples,
                sample_rate,
                self.num_bins,
                reference_level_db,
                &mut self.fft_planner,
            );
        } else {
            self.display_data = vec![0u64; self.num_bins];
        }
    }

    /// Returns the current display data.
    pub fn data(&self) -> &[u64] {
        &self.display_data
    }
}

/// Calculates frequency spectrum from audio samples using FFT.
///
/// Returns magnitudes normalized to 0-100, matching volume meter scaling.
/// Focuses on 100-1500 Hz (human voice fundamentals and harmonics).
///
/// # Arguments
/// * `samples` - Audio samples (i16 PCM)
/// * `sample_rate` - Audio sample rate in Hz
/// * `num_bins` - Number of frequency bins to return (typically terminal width)
/// * `reference_level_db` - Reference level for 100% display
/// * `fft_planner` - Reusable FFT planner for performance
pub fn calculate_spectrum(
    samples: &[i16],
    sample_rate: u32,
    num_bins: usize,
    reference_level_db: i8,
    fft_planner: &mut FftPlanner<f32>,
) -> Vec<u64> {
    if samples.is_empty() {
        return vec![0u64; num_bins];
    }

    let fft_size = 2048;
    let sample_count = samples.len().min(fft_size);
    let start_idx = samples.len().saturating_sub(sample_count);
    let recent_samples = &samples[start_idx..];

    // Apply Hanning window to reduce spectral leakage
    let mut buffer: Vec<Complex<f32>> = recent_samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window = 0.5
                * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / sample_count as f32).cos());
            Complex::new(s as f32 * window / 32768.0, 0.0)
        })
        .collect();

    buffer.resize(fft_size, Complex::new(0.0, 0.0));

    let fft = fft_planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    let freq_resolution = sample_rate as f32 / fft_size as f32;

    // Focus on core human voice range: 100-1500 Hz
    let min_freq = 100.0;
    let max_freq = 1500.0;

    let min_bin = (min_freq / freq_resolution) as usize;
    let max_bin = (max_freq / freq_resolution).min((fft_size / 2) as f32) as usize;

    let noise_gate_db = reference_level_db as f32 - 35.0;

    // Distribute FFT bins evenly across display width
    let useful_bins = max_bin - min_bin;
    let mut result = vec![0u64; num_bins];

    for (display_idx, result_bin) in result.iter_mut().enumerate() {
        let start_bin =
            min_bin + ((display_idx * useful_bins) as f32 / num_bins as f32) as usize;
        let end_bin = (min_bin
            + (((display_idx + 1) * useful_bins) as f32 / num_bins as f32) as usize)
            .min(max_bin)
            .max(start_bin + 1);

        if start_bin >= max_bin {
            break;
        }

        let mut sum = 0.0;
        let mut count = 0;
        for bin_idx in start_bin..end_bin {
            if bin_idx < buffer.len() / 2 {
                sum += buffer[bin_idx].norm();
                count += 1;
            }
        }

        if count > 0 {
            let avg_magnitude = sum / count as f32;

            let db = if avg_magnitude > 1e-10 {
                20.0 * avg_magnitude.log10()
            } else {
                -100.0
            };

            // Reduce by 20 dB to align FFT energy concentration with RMS volume
            let adjusted_db = db - 20.0;

            if adjusted_db < noise_gate_db {
                *result_bin = 0;
            } else {
                let db_range = reference_level_db as f32 - noise_gate_db;
                let normalized =
                    ((adjusted_db - noise_gate_db) / db_range * 100.0).clamp(0.0, 100.0);
                *result_bin = normalized as u64;
            }
        }
    }

    result
}
