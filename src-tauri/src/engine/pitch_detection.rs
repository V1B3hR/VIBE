#![allow(dead_code)]
pub enum PitchAlgo {
    Yin,
    Autocorrelation,
}

pub struct PitchDetector {
    pub algo: PitchAlgo,
    pub sample_rate: f64,
}

impl PitchDetector {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            algo: PitchAlgo::Yin,
            sample_rate,
        }
    }

    /// Detect fundamental frequency (Hz) from a buffer.
    pub fn detect_pitch(&self, samples: &[f32]) -> Option<f32> {
        match self.algo {
            PitchAlgo::Yin => self.yin_detect(samples),
            PitchAlgo::Autocorrelation => self.autocorr_detect(samples),
        }
    }

    fn yin_detect(&self, samples: &[f32]) -> Option<f32> {
        let n = samples.len();
        let w = n / 2;
        let mut diff = vec![0.0; w];

        // 1. Difference function
        for tau in 0..w {
            for j in 0..w {
                let d = samples[j] - samples[j + tau];
                diff[tau] += d * d;
            }
        }

        // 2. Cumulative mean normalized difference
        let mut cmnd = vec![0.0; w];
        cmnd[0] = 1.0;
        let mut running_sum = 0.0;
        for tau in 1..w {
            running_sum += diff[tau];
            if running_sum > 0.0 {
                cmnd[tau] = diff[tau] / (running_sum / tau as f32);
            } else {
                cmnd[tau] = 1.0;
            }
        }

        // 3. Absolute thresholding with local minimum search
        let threshold = 0.15;
        let mut tau_found = None;
        for tau in 2..w {
            if cmnd[tau] < threshold {
                let mut best_tau = tau;
                for t in tau + 1..w {
                    if cmnd[t] < cmnd[best_tau] {
                        best_tau = t;
                    } else {
                        break;
                    }
                }
                tau_found = Some(best_tau);
                break;
            }
        }

        // 4. Fallback to global minimum
        if tau_found.is_none() {
            let mut min_val = 1.0;
            let mut best_tau = 0;
            for tau in 2..w {
                if cmnd[tau] < min_val {
                    min_val = cmnd[tau];
                    best_tau = tau;
                }
            }
            if best_tau > 0 {
                tau_found = Some(best_tau);
            }
        }

        if let Some(t) = tau_found {
            if t > 0 {
                // 5. Parabolic Interpolation
                if t < w - 1 {
                    let y1 = cmnd[t - 1];
                    let y2 = cmnd[t];
                    let y3 = cmnd[t + 1];
                    let denom = y1 + y3 - 2.0 * y2;
                    if denom.abs() > 1e-6 {
                        let delta = 0.5 * (y1 - y3) / denom;
                        return Some(self.sample_rate as f32 / (t as f32 + delta));
                    }
                }
                return Some(self.sample_rate as f32 / t as f32);
            }
        }
        None
    }

    fn autocorr_detect(&self, samples: &[f32]) -> Option<f32> {
        let n = samples.len();
        let mut max_corr = -1.0;
        let mut best_tau = 0;

        for tau in 20..(n / 2) {
            let mut corr = 0.0;
            for i in 0..(n - tau) {
                corr += samples[i] * samples[i + tau];
            }
            // Normalize by integration period
            corr /= (n - tau) as f32;

            if corr > max_corr {
                max_corr = corr;
                best_tau = tau;
            }
        }

        if best_tau > 0 {
            Some(self.sample_rate as f32 / best_tau as f32)
        } else {
            None
        }
    }

    pub fn hz_to_midi(hz: f32) -> u8 {
        (12.0 * (hz / 440.0).log2() + 69.0).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_detection_yin() {
        let detector = PitchDetector::new(48000.0);

        // Generate a 440Hz sine wave (A4)
        let freq = 440.0;
        let mut samples = vec![0.0; 4096];
        for i in 0..4096 {
            samples[i] = (2.0 * std::f32::consts::PI * freq * i as f32 / 48000.0).sin();
        }

        let detected = detector.detect_pitch(&samples);
        assert!(detected.is_some());
        let val = detected.unwrap();

        // Tolerance within 5Hz
        assert!((val - freq).abs() < 5.0, "Expected ~440Hz, got {}", val);
        assert_eq!(PitchDetector::hz_to_midi(val), 69); // A4
    }

    #[test]
    fn test_pitch_detection_autocorr() {
        let mut detector = PitchDetector::new(44100.0);
        detector.algo = PitchAlgo::Autocorrelation;

        let freq = 1000.0;
        let mut samples = vec![0.0; 1024];
        for i in 0..1024 {
            samples[i] = (2.0 * std::f32::consts::PI * freq * i as f32 / 44100.0).sin();
        }

        let detected = detector.detect_pitch(&samples);
        assert!(detected.is_some());
        let val = detected.unwrap();

        assert!((val - freq).abs() < 50.0); // Autocorr is less precise without interpolation
    }
}
