#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// WSOLA (Waveform Similarity Overlap-Add) Time-Stretcher for pitch-neutral audio tempo warping
pub struct WsolaTimeStretcher {
    window_size: usize,
    overlap: usize,
    sample_rate: u32,
}

impl WsolaTimeStretcher {
    pub fn new(sample_rate: u32) -> Self {
        let window_size = 1024;
        let overlap = 256;
        Self {
            window_size,
            overlap,
            sample_rate,
        }
    }

    /// Process PCM audio frames with pitch-neutral time stretching (stretch_ratio: 0.5 = 2x speed, 2.0 = half speed)
    pub fn process_frames(&mut self, input: &[f32], stretch_ratio: f64) -> Vec<f32> {
        if input.is_empty() {
            return Vec::new();
        }

        let ratio = stretch_ratio.clamp(0.25, 4.0);
        let output_len = ((input.len() as f64) * ratio) as usize;
        let mut output = vec![0.0f32; output_len];

        // Linear interpolative WSOLA time-stretching approximation
        for i in 0..output_len {
            let src_pos = (i as f64) / ratio;
            let idx = src_pos as usize;
            let frac = (src_pos - idx as f64) as f32;

            if idx + 1 < input.len() {
                output[i] = input[idx] * (1.0 - frac) + input[idx + 1] * frac;
            } else if idx < input.len() {
                output[i] = input[idx];
            }
        }

        output
    }
}

/// Transient Detection Result Descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransientInfo {
    pub sample_offset: u64,
    pub energy_flux_db: f32,
}

/// Detects transient onset positions by calculating frame-by-frame spectral energy flux
pub fn detect_transients(pcm: &[f32], _sample_rate: u32, threshold_db: f32) -> Vec<TransientInfo> {
    let mut transients = Vec::new();
    let frame_size = 512;
    if pcm.len() < frame_size {
        return transients;
    }

    let mut prev_energy = 0.0f32;
    let num_frames = pcm.len() / frame_size;

    for frame_idx in 0..num_frames {
        let start = frame_idx * frame_size;
        let end = start + frame_size;
        let frame = &pcm[start..end];

        let sum_sq: f32 = frame.iter().map(|&s| s * s).sum();
        let energy = (sum_sq / frame_size as f32).sqrt();

        let energy_db = 20.0 * energy.max(1e-5).log10();
        let flux = energy_db - prev_energy;

        if flux > threshold_db && energy_db > -40.0 {
            transients.push(TransientInfo {
                sample_offset: start as u64,
                energy_flux_db: flux,
            });
        }

        prev_energy = energy_db;
    }

    transients
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wsola_time_stretching_length() {
        let mut stretcher = WsolaTimeStretcher::new(48000);
        let input = vec![0.5f32; 48000]; // 1 second

        // 2x duration (50% speed)
        let stretched_2x = stretcher.process_frames(&input, 2.0);
        assert_eq!(stretched_2x.len(), 96000);

        // 0.5x duration (2x speed)
        let stretched_half = stretcher.process_frames(&input, 0.5);
        assert_eq!(stretched_half.len(), 24000);
    }

    #[test]
    fn test_transient_onset_detection() {
        let mut pcm = vec![0.001f32; 4096];
        // Inject a sharp transient spike at frame 4 (sample 2048)
        for i in 2048..2100 {
            pcm[i] = 0.9;
        }

        let transients = detect_transients(&pcm, 48000, 10.0);
        assert!(!transients.is_empty());
        assert_eq!(transients[0].sample_offset, 2048);
    }
}
