#![allow(dead_code)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::engine::graph::MAX_CHANNELS;

pub mod lufs_meter;
pub use lufs_meter::{LufsMeter, LufsResults};

/// GPU-style asynchronous metering system
/// Computes RMS/Peak/LUFS on separate paths, passes atomic values to UI
pub struct GpuMeter {
    peaks: Vec<Arc<AtomicU64>>,
    rms: Vec<Arc<AtomicU64>>,
    true_peaks: Vec<Arc<AtomicU64>>,
    lufs_integrated: Arc<AtomicU64>,
    lufs_momentary: Arc<AtomicU64>,
    lufs_short_term: Arc<AtomicU64>,

    // Professional R128 Meter (standardized for 2-5 channels usually, used for master)
    lufs_engine: Arc<LufsMeter>,
}

impl GpuMeter {
    pub fn new(sample_rate: u32) -> Self {
        let mut peaks = Vec::with_capacity(MAX_CHANNELS);
        let mut rms = Vec::with_capacity(MAX_CHANNELS);
        let mut true_peaks = Vec::with_capacity(MAX_CHANNELS);

        for _ in 0..MAX_CHANNELS {
            peaks.push(Arc::new(AtomicU64::new(0)));
            rms.push(Arc::new(AtomicU64::new(0)));
            true_peaks.push(Arc::new(AtomicU64::new((-70.0f64).to_bits())));
        }

        Self {
            peaks,
            rms,
            true_peaks,
            lufs_integrated: Arc::new(AtomicU64::new((-70.0f64).to_bits())),
            lufs_momentary: Arc::new(AtomicU64::new((-70.0f64).to_bits())),
            lufs_short_term: Arc::new(AtomicU64::new((-70.0f64).to_bits())),
            lufs_engine: Arc::new(LufsMeter::new(sample_rate)),
        }
    }

    /// Update meters from multichannel audio buffers
    pub fn update_multichannel(&self, channels: &[&[f64]]) {
        let num_chans = channels.len();
        if num_chans == 0 {
            return;
        }
        let frames = channels[0].len();
        if frames == 0 {
            return;
        }

        for c in 0..num_chans.min(MAX_CHANNELS) {
            let data = channels[c];

            // 1. Raw Peak
            let peak = data.iter().map(|x| x.abs()).fold(0.0f64, f64::max);
            self.peaks[c].store(peak.to_bits(), Ordering::Relaxed);

            // 2. RMS
            let sum_sq: f64 = data.iter().map(|x| x * x).sum();
            let rms = (sum_sq / frames as f64).sqrt();
            self.rms[c].store(rms.to_bits(), Ordering::Relaxed);
        }

        // 3. EBU R128 LUFS & True Peak (Legacy/Master stereo for now)
        if num_chans >= 2 {
            self.lufs_engine.process(channels[0], channels[1]);
            let results = self.lufs_engine.get_results();

            self.lufs_integrated
                .store(results.integrated.to_bits(), Ordering::Relaxed);
            self.lufs_momentary
                .store(results.momentary.to_bits(), Ordering::Relaxed);
            self.lufs_short_term
                .store(results.short_term.to_bits(), Ordering::Relaxed);
            self.true_peaks[0].store(results.true_peak_l.to_bits(), Ordering::Relaxed);
            self.true_peaks[1].store(results.true_peak_r.to_bits(), Ordering::Relaxed);
        }
    }

    /// Backward compatibility for stereo update
    pub fn update(&self, left: &[f64], right: &[f64]) {
        self.update_multichannel(&[left, right]);
    }

    pub fn get_peak(&self, channel: usize) -> f64 {
        if channel < self.peaks.len() {
            f64::from_bits(self.peaks[channel].load(Ordering::Relaxed))
        } else {
            0.0
        }
    }

    pub fn get_rms(&self, channel: usize) -> f64 {
        if channel < self.rms.len() {
            f64::from_bits(self.rms[channel].load(Ordering::Relaxed))
        } else {
            0.0
        }
    }

    pub fn get_peak_db(&self, channel: usize) -> f64 {
        to_db(self.get_peak(channel))
    }

    pub fn get_rms_db(&self, channel: usize) -> f64 {
        to_db(self.get_rms(channel))
    }

    /// Get LUFS values
    pub fn get_lufs_full(&self) -> LufsResults {
        LufsResults {
            integrated: f64::from_bits(self.lufs_integrated.load(Ordering::Relaxed)),
            momentary: f64::from_bits(self.lufs_momentary.load(Ordering::Relaxed)),
            short_term: f64::from_bits(self.lufs_short_term.load(Ordering::Relaxed)),
            true_peak_l: f64::from_bits(self.true_peaks[0].load(Ordering::Relaxed)),
            true_peak_r: f64::from_bits(if self.true_peaks.len() > 1 {
                self.true_peaks[1].load(Ordering::Relaxed)
            } else {
                0
            }),
            range: 0.0,
        }
    }

    pub fn reset_lufs(&self) {
        self.lufs_engine.reset();
    }
}

/// BS.1770 K-Weighting Filter Chain
/// Stage 1: High Shelving (Gain +4dB, fc ~1500Hz)
/// Stage 2: High Pass (fc ~100Hz)
struct KWeightingFilter {
    // Stage 1 State
    x1_1: f64,
    x2_1: f64,
    y1_1: f64,
    y2_1: f64,
    // Stage 2 State
    x1_2: f64,
    x2_2: f64,
    y1_2: f64,
    y2_2: f64,
}

impl KWeightingFilter {
    fn new() -> Self {
        Self {
            x1_1: 0.0,
            x2_1: 0.0,
            y1_1: 0.0,
            y2_1: 0.0,
            x1_2: 0.0,
            x2_2: 0.0,
            y1_2: 0.0,
            y2_2: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f64) -> f64 {
        // Coefficients for 48kHz (Standard)
        // Stage 1 (Head Shadow Filter - Shelf)
        let a0_1 = 1.53512485958697;
        let a1_1 = -2.69169618940638;
        let a2_1 = 1.19839281085285;
        let b0_1 = 1.0;
        let b1_1 = -1.69065929318241;
        let b2_1 = 0.73248077421585;

        // Direct Form II Transposed implementation (or DF1)
        // y[n] = (a0*x[n] + a1*x[n-1] + a2*x[n-2] - b1*y[n-1] - b2*y[n-2]) / b0
        let out1 = (a0_1 * input + a1_1 * self.x1_1 + a2_1 * self.x2_1
            - b1_1 * self.y1_1
            - b2_1 * self.y2_1)
            / b0_1;

        // Update state 1
        self.x2_1 = self.x1_1;
        self.x1_1 = input;
        self.y2_1 = self.y1_1;
        self.y1_1 = out1;

        // Stage 2 (High Pass)
        let a0_2 = 1.0;
        let a1_2 = -2.0;
        let a2_2 = 1.0;
        let b0_2 = 1.0;
        let b1_2 = -1.99004745483398;
        let b2_2 = 0.99007225036621;

        let out2 = (a0_2 * out1 + a1_2 * self.x1_2 + a2_2 * self.x2_2
            - b1_2 * self.y1_2
            - b2_2 * self.y2_2)
            / b0_2;

        // Update state 2
        self.x2_2 = self.x1_2;
        self.x1_2 = out1;
        self.y2_2 = self.y1_2;
        self.y1_2 = out2;

        out2
    }
}

/// Convert linear amplitude to dB
fn to_db(linear: f64) -> f64 {
    if linear <= 0.0 {
        -96.0 // Silence threshold
    } else {
        20.0 * linear.log10()
    }
}

/// Convert power to dB
fn to_db_power(power: f64) -> f64 {
    if power <= 0.0 {
        -96.0
    } else {
        10.0 * power.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_update() {
        let meter = GpuMeter::new(44100);

        let left = vec![0.5; 1024];
        let right = vec![0.75; 1024];

        meter.update(&left, &right);

        let peak_l = meter.get_peak(0);
        let peak_r = meter.get_peak(1);
        assert!((peak_l - 0.5).abs() < 0.001);
        assert!((peak_r - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_db_conversion() {
        assert_eq!(to_db(1.0), 0.0); // 0 dB
        assert!((to_db(0.5) - (-6.02)).abs() < 0.1); // ~-6 dB
        assert_eq!(to_db(0.0), -96.0); // Silence
    }
}
