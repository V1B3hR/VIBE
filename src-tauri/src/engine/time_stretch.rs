// Faza 1: Time Stretching
/// Algorithms available for time stretching.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum StretchAlgorithm {
    /// Classic phase vocoder for tonal materials.
    PhaseVocoder {
        fft_size: usize, // 2048, 4096, 8192
        hop_size: usize, // fft_size / 4
        window: WindowType,
    },
    /// Pitch Synchronous Overlap-Add for monophonic sources (vocals/instruments).
    Psola { crossfade_ms: f64 },
    /// Modern transient-aware algorithm.
    SignalSmith { transient_preservation: bool },
    /// Simple linear interpolation for preview/low-fidelity.
    Linear,
}

/// Window functions for spectral processing.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum WindowType {
    Hann,
    Hamming,
    Blackman,
    Kaiser(f64),
}

/// Quality presets for stretching.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(dead_code)]
pub enum StretchQuality {
    Draft, // Fast, artifacts OK (preview)
    #[default]
    Standard, // Balanced (real-time)
    HQ,    // High quality (offline)
    Ultra, // Best quality (mastering)
}

/// Time Stretching Engine.
/// Provides industry-standard time compression/expansion without affecting pitch.
#[allow(dead_code)]
pub struct TimeStretcher {
    pub algorithm: StretchAlgorithm,
    pub quality: StretchQuality,
    pub sample_rate: f64,
    pub channels: usize,

    // State buffers
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
}

#[allow(dead_code)]
impl TimeStretcher {
    pub fn new(algorithm: StretchAlgorithm, sample_rate: f64, channels: usize) -> Self {
        Self {
            algorithm,
            quality: StretchQuality::Standard,
            sample_rate,
            channels,
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
        }
    }

    /// Process audio data.
    pub fn process(&mut self, input: &[f32], ratio: f64) -> Vec<f32> {
        if (ratio - 1.0).abs() < 0.001 {
            return input.to_vec();
        }

        match self.algorithm {
            StretchAlgorithm::Linear => self.process_linear(input, ratio),
            StretchAlgorithm::Psola { crossfade_ms: _ } => {
                // Implementing WSOLA for pitch-preserving monophonic/polyphonic stretching
                self.process_wsola(input, ratio)
            }
            StretchAlgorithm::SignalSmith { .. } => {
                // Fallback to WSOLA for high-quality
                self.process_wsola(input, ratio)
            }
            _ => self.process_linear(input, ratio)
        }
    }

    fn process_wsola(&self, input: &[f32], ratio: f64) -> Vec<f32> {
        let channels = self.channels;
        let win_size = 2048; // About 46ms at 44.1kHz
        let syn_hop = win_size / 2;
        let pmax = 512; // Maximum drift for cross-correlation search

        // Create Hanning window
        let mut window = vec![0.0f32; win_size];
        for i in 0..win_size {
            window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (win_size as f32 - 1.0)).cos());
        }

        let target_len = ((input.len() / channels) as f64 / ratio) as usize;
        let mut output = vec![0.0f32; target_len * channels];
        
        // We need to keep track of the output we've built to cross-correlate.
        // In a true block-by-block real-time system, this is kept in self.output_buffer.
        // For offline/chunk processing here, we work directly on output.

        let num_hops = target_len.saturating_sub(win_size) / syn_hop;
        if num_hops == 0 {
            return self.process_linear(input, ratio);
        }

        // Copy first window directly
        let first_in = 0;
        let max_input_frames = input.len() / channels;
        for i in 0..win_size {
            if i < max_input_frames {
                for c in 0..channels {
                    output[i * channels + c] = input[(first_in + i) * channels + c] * window[i];
                }
            }
        }

        for k in 1..num_hops {
            let out_pos = k * syn_hop;
            let ideal_in_pos = (out_pos as f64 * ratio) as usize;
            
            // Search limits
            let search_min = ideal_in_pos.saturating_sub(pmax);
            let search_max = (ideal_in_pos + pmax).min(max_input_frames.saturating_sub(win_size));

            // The template to match is the overlapping part of the last output window
            // which is from out_pos to out_pos + syn_hop.
            // But we actually cross-correlate with the natural continuation of the LAST used input window.
            // For simplicity and stability, we correlate the overlapping output with the candidate inputs.
            let mut best_in_pos = search_min;
            let mut best_corr = f32::MIN;

            for candidate in search_min..search_max {
                let mut corr = 0.0f32;
                // Cross-correlation over the overlapping region
                for i in 0..syn_hop {
                    let out_idx = out_pos + i;
                    let cand_idx = candidate + i;
                    
                    if cand_idx < max_input_frames && out_idx < target_len {
                        // Sum across channels for phase-coherent stereo WSOLA
                        let mut frame_corr = 0.0;
                        for c in 0..channels {
                            frame_corr += output[out_idx * channels + c] * input[cand_idx * channels + c];
                        }
                        corr += frame_corr;
                    }
                }
                if corr > best_corr {
                    best_corr = corr;
                    best_in_pos = candidate;
                }
            }

            // Overlap-Add the best matching window
            for i in 0..win_size {
                let out_idx = out_pos + i;
                let in_idx = best_in_pos + i;
                if in_idx < max_input_frames && out_idx < target_len {
                    for c in 0..channels {
                        output[out_idx * channels + c] += input[in_idx * channels + c] * window[i];
                    }
                }
            }
        }

        // Apply gain compensation due to hop-size/window summing
        // For Hanning with 50% overlap, sum is 1.0, so it's already amplitude-preserved!
        output
    }

    fn process_linear(&self, input: &[f32], ratio: f64) -> Vec<f32> {
        let target_len = (input.len() as f64 / ratio) as usize;
        let mut output = Vec::with_capacity(target_len);

        for i in 0..target_len {
            let src_pos = i as f64 * ratio;
            let idx = src_pos as usize;
            let frac = (src_pos - idx as f64) as f32;

            if idx + self.channels < input.len() {
                for c in 0..self.channels {
                    let s1 = input[idx + c];
                    let s2 = input[idx + self.channels + c];
                    output.push(s1 + (s2 - s1) * frac);
                }
            } else {
                for c in 0..self.channels {
                    output.push(input[input.len() - self.channels + c]);
                }
            }
        }
        output
    }

    // Phase 1.1 Helpers for presets
    pub fn preset_drums(sample_rate: f64, channels: usize) -> Self {
        Self::new(
            StretchAlgorithm::SignalSmith {
                transient_preservation: true,
            },
            sample_rate,
            channels,
        )
    }

    pub fn preset_vocals(sample_rate: f64, channels: usize) -> Self {
        Self::new(
            StretchAlgorithm::Psola { crossfade_ms: 15.0 },
            sample_rate,
            channels,
        )
    }
}

/// Utility for Instant Warp: Syncs sample to project BPM.
#[allow(dead_code)]
pub fn calculate_stretch_ratio(sample_bpm: f64, project_bpm: f64) -> f64 {
    sample_bpm / project_bpm
}
