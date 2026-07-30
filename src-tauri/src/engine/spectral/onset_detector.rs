use super::types::MelFrame;

pub struct OnsetDetector {
    prev_magnitude: Option<Vec<f32>>,
    threshold: f32,
    flux_history: Vec<f32>,
}

impl OnsetDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            prev_magnitude: None,
            threshold,
            flux_history: Vec::with_capacity(100),
        }
    }

    pub fn process_frame(&mut self, frame: &MelFrame) -> f32 {
        let current_mag = &frame.data;
        let mut flux = 0.0;

        if let Some(prev) = &self.prev_magnitude {
            for (&c, &p) in current_mag.iter().zip(prev.iter()) {
                let diff = c - p;
                if diff > 0.0 {
                    flux += diff;
                }
            }
        }

        self.prev_magnitude = Some(current_mag.clone());

        // Normalize flux by number of mel bins
        flux /= current_mag.len() as f32;

        self.flux_history.push(flux);
        if self.flux_history.len() > 100 {
            self.flux_history.remove(0);
        }

        flux
    }

    pub fn is_onset(&self, flux: f32) -> bool {
        // Simple adaptive thresholding could be better, but start with fixed
        flux > self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onset_detection() {
        let mut detector = OnsetDetector::new(5.0);

        // Quiet frame
        let f1 = MelFrame {
            data: vec![-90.0; 128],
            timestamp_samples: 0,
        };
        let flux1 = detector.process_frame(&f1);
        assert!(!detector.is_onset(flux1));

        // Loud frame (sudden jump)
        let f2 = MelFrame {
            data: vec![-10.0; 128],
            timestamp_samples: 512,
        };
        let flux2 = detector.process_frame(&f2);

        // Flux = sum(diff > 0) / 128
        // diff = (-10) - (-90) = 80
        // sum = 80 * 128
        // flux = (80 * 128) / 128 = 80
        assert!(flux2 > 5.0);
        assert!(detector.is_onset(flux2));

        // Sustained loud frame
        let f3 = MelFrame {
            data: vec![-10.0; 128],
            timestamp_samples: 1024,
        };
        let flux3 = detector.process_frame(&f3);
        assert!(flux3 < 1.0); // No flux if energy (dB) is constant
        assert!(!detector.is_onset(flux3));
    }
}
