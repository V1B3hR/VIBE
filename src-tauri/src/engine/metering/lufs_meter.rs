use ebur128::{EbuR128, Mode};
use std::sync::Mutex;

/// Professional LUFS Metering compliant with EBU R128 / BS.1770-4
pub struct LufsMeter {
    meter: Mutex<EbuR128>,
    sample_rate: u32,
}

impl LufsMeter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            meter: Mutex::new(
                EbuR128::new(
                    2,
                    sample_rate,
                    Mode::I | Mode::M | Mode::S | Mode::TRUE_PEAK,
                )
                .expect("Failed to initialize EBU R128 meter"),
            ),
            sample_rate,
        }
    }

    pub fn process(&self, left: &[f64], right: &[f64]) {
        let frames = left.len().min(right.len());
        if frames == 0 {
            return;
        }

        // Interleave for ebur128
        let mut interleaved = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            interleaved.push(left[i]);
            interleaved.push(right[i]);
        }

        if let Ok(mut meter) = self.meter.lock() {
            let _ = meter.add_frames_f64(&interleaved);
        }
    }

    pub fn get_results(&self) -> LufsResults {
        if let Ok(meter) = self.meter.lock() {
            LufsResults {
                momentary: meter.loudness_momentary().unwrap_or(-70.0),
                short_term: meter.loudness_shortterm().unwrap_or(-70.0),
                integrated: meter.loudness_global().unwrap_or(-70.0),
                true_peak_l: meter.true_peak(0).unwrap_or(-70.0),
                true_peak_r: meter.true_peak(1).unwrap_or(-70.0),
                range: meter.loudness_range().unwrap_or(0.0),
            }
        } else {
            LufsResults::default()
        }
    }

    pub fn reset(&self) {
        if let Ok(mut meter) = self.meter.lock() {
            // Re-initializing is the safest way to reset integrated loudness in this crate
            *meter = EbuR128::new(
                2,
                self.sample_rate,
                Mode::I | Mode::M | Mode::S | Mode::TRUE_PEAK,
            )
            .unwrap();
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LufsResults {
    pub momentary: f64,
    pub short_term: f64,
    pub integrated: f64,
    pub true_peak_l: f64,
    pub true_peak_r: f64,
    pub range: f64,
}

impl Default for LufsResults {
    fn default() -> Self {
        Self {
            momentary: -70.0,
            short_term: -70.0,
            integrated: -70.0,
            true_peak_l: -70.0,
            true_peak_r: -70.0,
            range: 0.0,
        }
    }
}
