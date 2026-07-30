use crate::engine::kropelka::MixAnalysis;

/// Analyzes the master output to provide feedback to Dropel (Offline Logic).
#[allow(dead_code)]
pub struct MixAnalyzer {
    pub sample_rate: f64,
    pub window_size: usize,
}

#[allow(dead_code)]
impl MixAnalyzer {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            window_size: 2048,
        }
    }

    /// Perform analysis on a buffer of master output samples.
    /// Extracts heuristics for Vibe, Energy, and technical health.
    pub fn analyze(&self, samples: &[f32]) -> MixAnalysis {
        let mut max_peak = 0.0f32;
        let mut sum_sq = 0.0f32;
        let mut clipping = false;
        let mut zero_crossings = 0;
        let mut prev_s = 0.0f32;
        let mut transient_peaks = 0;

        // Band analysis sums
        let mut sub_sum = 0.0f32; // 20-80Hz
        let mut low_sum = 0.0f32; // 80-300Hz
        let mut low_mid_sum = 0.0f32; // 300-1kHz
        let mut mid_sum = 0.0f32; // 1k-4kHz
        let mut presence_sum = 0.0f32; // 4k-8kHz
        let mut air_sum = 0.0f32; // 8k-20kHz

        // Stereo Correlation (assuming interleaved L/R)
        let mut left_sum_sq = 0.0f32;
        let mut right_sum_sq = 0.0f32;
        let mut l_r_product_sum = 0.0f32;

        for chunks in samples.chunks_exact(2) {
            let l = chunks[0];
            let r = chunks[1];

            left_sum_sq += l * l;
            right_sum_sq += r * r;
            l_r_product_sum += l * r;

            for &s in chunks {
                let abs_s = s.abs();
                if abs_s > max_peak {
                    max_peak = abs_s;
                }
                if abs_s >= 1.0 {
                    clipping = true;
                }
                sum_sq += s * s;

                // Zero crossing rate (proxy for spectral centroid/brightness)
                if (s > 0.0 && prev_s <= 0.0) || (s < 0.0 && prev_s >= 0.0) {
                    zero_crossings += 1;
                }

                // Simple transient detection (energy spike)
                if abs_s > prev_s * 2.0 && abs_s > 0.1 {
                    transient_peaks += 1;
                }

                // Band approximation using Zero Crossing Rate (ZCR) chunks
                // This is a very light-weight heuristic instead of full FFT
                let inst_zcr = if abs_s > 0.01 {
                    (s - prev_s).abs()
                } else {
                    0.0
                };
                if inst_zcr < 0.05 {
                    sub_sum += abs_s;
                } else if inst_zcr < 0.15 {
                    low_sum += abs_s;
                } else if inst_zcr < 0.4 {
                    low_mid_sum += abs_s;
                } else if inst_zcr < 0.7 {
                    mid_sum += abs_s;
                } else if inst_zcr < 0.9 {
                    presence_sum += abs_s;
                } else {
                    air_sum += abs_s;
                }

                prev_s = s;
            }
        }

        let total_samples = samples.len() as f32;
        let rms = (sum_sq / total_samples).sqrt();

        // Correlation = sum(L*R) / sqrt(sum(L^2) * sum(R^2))
        let correlation = if left_sum_sq > 0.0 && right_sum_sq > 0.0 {
            l_r_product_sum / (left_sum_sq * right_sum_sq).sqrt()
        } else {
            1.0
        };

        // Map heuristics to 0.0 - 1.0 range
        let zcr = (zero_crossings as f32 / total_samples) * 10.0;
        let t_density = (transient_peaks as f32 / (total_samples / 1000.0)).min(1.0);

        // Normalize bands
        let band_total = sub_sum + low_sum + low_mid_sum + mid_sum + presence_sum + air_sum + 1e-6;

        // Simple LUFS Approximation (K-weighting essentially emphasizes highs)
        // More high energy = higher perceived loudness
        let k_weighted_energy =
            (low_mid_sum * 0.8 + mid_sum * 1.2 + presence_sum * 1.5 + air_sum * 1.0)
                / total_samples;
        let lufs = if k_weighted_energy > 0.0 {
            0.6 * (10.0 * k_weighted_energy.log10()) // Scaled approximation
        } else {
            -70.0
        };

        MixAnalysis {
            rms_level: rms,
            peak_level: max_peak,
            clipping_detected: clipping,
            spectral_balance: zcr.min(1.0),
            transient_density: t_density,
            spectral_centroid: zcr,
            masking_detected: rms > 0.6 && zcr < 0.3,
            stereo_correlation: correlation,
            frequency_bands: [
                sub_sum / band_total,
                low_sum / band_total,
                low_mid_sum / band_total,
                mid_sum / band_total,
                presence_sum / band_total,
                air_sum / band_total,
            ],
            lufs_level: lufs,
        }
    }
}
