use crate::engine::resampler::{ResamplingQuality, VibeResampler};
use crate::engine::time_stretch::{StretchAlgorithm, TimeStretcher};

/// Algorithms for pitch shifting.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum PitchAlgorithm {
    /// Classic stretch + resample approach.
    TimeStretchResample,
    /// Grain-based synthesis for smoother polyphonic material.
    GranularSynthesis,
    /// Direct frequency domain manipulation.
    SpectralShift,
}

/// Pitch Shifting Engine.
/// Allows changing the frequency of audio without changing its duration.
#[allow(dead_code)]
pub struct PitchShifter {
    pub algorithm: PitchAlgorithm,
    pub formant_preservation: bool,

    // Dual-stage components
    stretcher: TimeStretcher,
    resampler: VibeResampler,

    sample_rate: f64,
    channels: usize,
}

#[allow(dead_code)]
impl PitchShifter {
    pub fn new(sample_rate: f64, channels: usize) -> Self {
        Self {
            algorithm: PitchAlgorithm::TimeStretchResample,
            formant_preservation: false,
            stretcher: TimeStretcher::new(StretchAlgorithm::Linear, sample_rate, channels),
            resampler: VibeResampler::new(ResamplingQuality::High, 1024, channels),
            sample_rate,
            channels,
        }
    }

    /// Shift pitch by semitones.
    pub fn shift_semitones(&mut self, input: &[f32], semitones: f64) -> Vec<f32> {
        if semitones.abs() < 0.001 {
            return input.to_vec();
        }

        let ratio = 2.0f64.powf(semitones / 12.0);

        match self.algorithm {
            PitchAlgorithm::TimeStretchResample => {
                // 1. Time-stretch by 1/ratio (compress time)
                // Note: TimeStretcher currently works with f32
                let stretched_f32 = self.stretcher.process(input, 1.0 / ratio);

                // Convert to f64 for high-quality resampling
                let stretched_f64: Vec<f64> = stretched_f32.iter().map(|&s| s as f64).collect();

                // 2. Resample to original length (raises pitch)
                match self.resampler.resample(
                    &stretched_f64,
                    self.sample_rate,
                    self.sample_rate * ratio,
                ) {
                    Ok(resampled_f64) => {
                        // Convert back to f32 for output
                        resampled_f64.iter().map(|&s| s as f32).collect()
                    }
                    Err(_) => input.to_vec(), // Fallback
                }
            }
            _ => {
                // Other algorithms not yet implemented
                input.to_vec()
            }
        }
    }
}
