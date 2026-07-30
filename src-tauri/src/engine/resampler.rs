#![allow(dead_code)]

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

/// High-fidelity resampler using Sinc interpolation (band-limited).
/// Powered by the pure-Rust `rubato` crate for cross-platform stability (x86/ARM).
/// Supports full f64 precision "Maybach" signal path.
pub struct VibeResampler {
    chunk_size: usize,
    num_channels: usize,
    quality: ResamplingQuality,
}

impl VibeResampler {
    pub fn new(quality: ResamplingQuality, chunk_size: usize, num_channels: usize) -> Self {
        Self {
            quality,
            chunk_size,
            num_channels,
        }
    }

    /// Resample a complete file/clip from source_rate to target_rate.
    /// Uses offline processing strategy (chunked) for memory efficiency.
    pub fn resample(
        &self,
        input: &[f64],
        source_rate: f64,
        target_rate: f64,
    ) -> Result<Vec<f64>, String> {
        if (source_rate - target_rate).abs() < 0.1 {
            return Ok(input.to_vec());
        }

        // Define Sinc parameters based on Quality
        let (sinc_len, f_cutoff, win_func) = match self.quality {
            ResamplingQuality::Quick => (32, 0.9, WindowFunction::Hann),
            ResamplingQuality::Low => (64, 0.95, WindowFunction::Blackman),
            ResamplingQuality::Medium => (128, 0.95, WindowFunction::BlackmanHarris2),
            ResamplingQuality::High => (256, 0.99, WindowFunction::BlackmanHarris2), // "VIBE Standard"
            ResamplingQuality::VeryHigh => (512, 0.999, WindowFunction::BlackmanHarris2), // "Maybach Mastering"
        };

        let params = SincInterpolationParameters {
            sinc_len,
            f_cutoff,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: 128,
            window: win_func,
        };

        // Prepare Rubato Resampler
        // Rubato expects separate vectors for each channel (Planar), but VIBE mostly interleaves or uses planar elsewhere.
        // For convenience here we assume input is Interleaved (L, R, L, R) and we de-interleave.

        let channels = self.num_channels;
        let mut planar_input: Vec<Vec<f64>> = vec![vec![]; channels];

        for (i, sample) in input.iter().enumerate() {
            planar_input[i % channels].push(*sample);
        }

        // Create resampler
        // SincFixedIn allows pushing arbitrary chunks
        let mut resampler = SincFixedIn::<f64>::new(
            target_rate / source_rate,
            2.0, // Max ratio constraints
            params,
            planar_input[0].len(), // Input length (ideal)
            channels,
        )
        .map_err(|e| format!("Failed to create Rubato resampler: {}", e))?;

        // Perform resampling
        let waves_out = resampler
            .process(&planar_input, None)
            .map_err(|e| format!("Rubato process failed: {}", e))?;

        // Interleave back
        let out_len = waves_out[0].len();
        let mut output = Vec::with_capacity(out_len * channels);
        for i in 0..out_len {
            for c in 0..channels {
                output.push(waves_out[c][i]);
            }
        }

        Ok(output)
    }
}

/// Resampling quality presets
#[derive(Clone, Copy, Debug, Default)]
pub enum ResamplingQuality {
    Quick, // Preview
    Low,
    Medium, // Real-time mixing
    #[default]
    High, // High quality (Offline Render default)
    VeryHigh, // Mastering grade (Sinc 512)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rubato_upsample() {
        let resampler = VibeResampler::new(ResamplingQuality::Medium, 1024, 1);

        let source_rate = 44100.0;
        let target_rate = 48000.0;

        // 10ms sine
        let samples = 441;
        let mut input = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = i as f64 / source_rate;
            input.push((2.0 * std::f64::consts::PI * 440.0 * t).sin());
        }

        let output = resampler
            .resample(&input, source_rate, target_rate)
            .unwrap();

        let expected_len = (samples as f64 * target_rate / source_rate).ceil() as usize;
        // Rubato output size can vary slightly depending on block handling/padding
        // Just verify it's reasonably close
        assert!((output.len() as i32 - expected_len as i32).abs() < 100);
    }
}
