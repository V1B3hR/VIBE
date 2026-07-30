#![allow(dead_code)]

use super::graph::AudioBuffer;

/// Oversampling factor
#[derive(Clone, Copy)]
pub enum OversamplingFactor {
    X4 = 4,
    X8 = 8,
}

/// Polyphase FIR filter for anti-aliasing
/// Uses pre-computed coefficients for efficient upsampling/downsampling
#[derive(Clone)]
pub struct PolyphaseFir {
    factor: usize,
    coefficients: Vec<f64>,
    taps_per_phase: usize,
    state: Vec<Vec<f64>>, // [channel][samples]
}

impl PolyphaseFir {
    /// Create new polyphase FIR filter
    /// factor: oversampling factor (4 or 8)
    pub fn new(factor: usize, num_channels: usize) -> Self {
        let taps_per_phase = 32; // 32 taps per phase for good stopband
        let total_taps = taps_per_phase * factor;

        // Design lowpass FIR using windowed sinc
        let cutoff = 0.5 / factor as f64; // Nyquist / oversampling factor
        let mut coefficients = vec![0.0; total_taps];

        for i in 0..total_taps {
            let n = i as f64 - (total_taps as f64 - 1.0) / 2.0;

            // Sinc function
            let sinc = if n == 0.0 {
                2.0 * std::f64::consts::PI * cutoff
            } else {
                (2.0 * std::f64::consts::PI * cutoff * n).sin() / n
            };

            // Blackman window for better stopband
            let window = 0.42
                - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (total_taps as f64 - 1.0)).cos()
                + 0.08 * (4.0 * std::f64::consts::PI * i as f64 / (total_taps as f64 - 1.0)).cos();

            coefficients[i] = sinc * window;
        }

        // Normalize
        let sum: f64 = coefficients.iter().sum();
        for coef in &mut coefficients {
            *coef /= sum;
        }

        Self {
            factor,
            coefficients,
            taps_per_phase,
            state: vec![vec![0.0; taps_per_phase]; num_channels],
        }
    }

    /// Upsample by inserting zeros and filtering
    pub fn upsample(&mut self, input: &[f64], output: &mut [f64], channel: usize) {
        let input_len = input.len();
        let output_len = input_len * self.factor;

        for i in 0..input_len {
            // Insert sample and zeros
            for phase in 0..self.factor {
                let out_idx = i * self.factor + phase;
                if out_idx >= output_len {
                    break;
                }

                if phase == 0 {
                    // Actual sample
                    self.state[channel].rotate_right(1);
                    self.state[channel][0] = input[i];
                }

                // Apply polyphase filter
                let mut sum = 0.0;
                for tap in 0..self.taps_per_phase {
                    let coef_idx = tap * self.factor + phase;
                    if coef_idx < self.coefficients.len() {
                        sum += self.state[channel][tap] * self.coefficients[coef_idx];
                    }
                }
                output[out_idx] = sum * self.factor as f64;
            }
        }
    }

    /// Downsample by filtering and decimating
    pub fn downsample(&mut self, input: &[f64], output: &mut [f64], channel: usize) {
        let output_len = input.len() / self.factor;

        for i in 0..output_len {
            // Take every Nth sample after filtering
            let in_idx = i * self.factor;

            self.state[channel].rotate_right(1);
            self.state[channel][0] = input[in_idx];

            // Apply filter
            let mut sum = 0.0;
            for tap in 0..self.taps_per_phase {
                let coef_idx = tap * self.factor;
                if coef_idx < self.coefficients.len() {
                    sum += self.state[channel][tap] * self.coefficients[coef_idx];
                }
            }
            output[i] = sum;
        }
    }
}

/// Oversampling wrapper for audio processors
/// Wraps any AudioProcessor to run at higher sample rate
pub struct OversamplingWrapper {
    factor: OversamplingFactor,
    upsampler: PolyphaseFir,
    downsampler: PolyphaseFir,
    internal_buffer: Vec<Vec<f64>>, // [channel][samples * factor]
}

impl OversamplingWrapper {
    pub fn new(factor: OversamplingFactor, max_frames: usize, num_channels: usize) -> Self {
        let f = factor as usize;
        Self {
            factor,
            upsampler: PolyphaseFir::new(f, num_channels),
            downsampler: PolyphaseFir::new(f, num_channels),
            internal_buffer: vec![vec![0.0; max_frames * f]; num_channels],
        }
    }

    /// Process audio through oversampled path
    /// Upsample -> Process at high rate -> Downsample
    pub fn process<F>(&mut self, buffer: &mut AudioBuffer, mut process_fn: F)
    where
        F: FnMut(&mut [f64], usize), // (samples, channel)
    {
        let frames = buffer.frames;
        let factor = self.factor as usize;
        let oversampled_frames = frames * factor;

        for ch in 0..buffer.num_channels {
            // Upsample
            self.upsampler.upsample(
                &buffer.channels_data[ch][..frames],
                &mut self.internal_buffer[ch][..oversampled_frames],
                ch,
            );

            // Process at high sample rate
            process_fn(&mut self.internal_buffer[ch][..oversampled_frames], ch);

            // Downsample
            self.downsampler.downsample(
                &self.internal_buffer[ch][..oversampled_frames],
                &mut buffer.channels_data[ch][..frames],
                ch,
            );
        }
    }
}
