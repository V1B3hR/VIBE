use super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use uuid::Uuid;

/// 4th-order Linkwitz-Riley crossover filter (24 dB/octave).
/// LR4 filters are phase-coherent and provide a flat magnitude response when summed.
#[derive(Clone)]
struct LinkwitzRiley4th {
    lp_stages: [[[f64; 5]; 2]; 2],
    hp_stages: [[[f64; 5]; 2]; 2],
    lp_state: [[[f64; 2]; 2]; 2],
    hp_state: [[[f64; 2]; 2]; 2],
}

impl LinkwitzRiley4th {
    fn new(cutoff: f64, sample_rate: f64) -> Self {
        let mut slf = Self {
            lp_stages: [[[0.0; 5]; 2]; 2],
            hp_stages: [[[0.0; 5]; 2]; 2],
            lp_state: [[[0.0; 2]; 2]; 2],
            hp_state: [[[0.0; 2]; 2]; 2],
        };
        slf.update(cutoff, sample_rate);
        slf
    }

    fn update(&mut self, cutoff: f64, sample_rate: f64) {
        let omega = 2.0 * std::f64::consts::PI * cutoff / sample_rate;
        let sn = omega.sin();
        let cs = omega.cos();
        let q = 1.0 / std::f64::consts::SQRT_2; // Butterworth Q
        let alpha = sn / (2.0 * q);

        let b0 = (1.0 - cs) / 2.0;
        let b1 = 1.0 - cs;
        let b2 = (1.0 - cs) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cs;
        let a2 = 1.0 - alpha;

        let lp = [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0];

        let b0_h = (1.0 + cs) / 2.0;
        let b1_h = -(1.0 + cs);
        let b2_h = (1.0 + cs) / 2.0;

        let hp = [b0_h / a0, b1_h / a0, b2_h / a0, a1 / a0, a2 / a0];

        for c in 0..2 {
            for s in 0..2 {
                self.lp_stages[c][s] = lp;
                self.hp_stages[c][s] = hp;
            }
        }
    }

    fn process(&mut self, input: f64, channel: usize) -> (f64, f64) {
        let mut lp = input;
        for s in 0..2 {
            let coeffs = self.lp_stages[channel][s];
            lp = Self::process_biquad(lp, coeffs, &mut self.lp_state[channel][s]);
        }

        let mut hp = input;
        for s in 0..2 {
            let coeffs = self.hp_stages[channel][s];
            hp = Self::process_biquad(hp, coeffs, &mut self.hp_state[channel][s]);
        }

        (lp, hp)
    }

    fn process_biquad(x: f64, coeffs: [f64; 5], state: &mut [f64; 2]) -> f64 {
        let y = coeffs[0] * x + state[0];
        state[0] = coeffs[1] * x - coeffs[3] * y + state[1];
        state[1] = coeffs[2] * x - coeffs[4] * y;
        y
    }
}

#[derive(Clone)]
struct BandCompressor {
    pub threshold: Parameter,
    pub ratio: Parameter,
    pub attack: Parameter,
    pub release: Parameter,
    pub gain: Parameter,
    pub knee: Parameter,

    envelope: [f64; 2],
    reduction: [f64; 2],
}

impl BandCompressor {
    fn new(name: &str, default_thr: f64) -> Self {
        Self {
            threshold: Parameter::new(&format!("{} Thr", name), default_thr, -60.0, 0.0),
            ratio: Parameter::new(&format!("{} Rat", name), 4.0, 1.0, 20.0),
            attack: Parameter::new(&format!("{} Att", name), 10.0, 0.1, 100.0),
            release: Parameter::new(&format!("{} Rel", name), 100.0, 1.0, 1000.0),
            gain: Parameter::new(&format!("{} Gain", name), 0.0, -12.0, 12.0),
            knee: Parameter::new(&format!("{} Knee", name), 6.0, 0.0, 24.0),
            envelope: [0.0; 2],
            reduction: [1.0; 2],
        }
    }

    fn process(&mut self, sample: f64, channel: usize, sample_rate: f64) -> f64 {
        let abs_sample = sample.abs();
        let att = self.attack.get_current_value() as f64;
        let rel = self.release.get_current_value() as f64;
        
        let attack_coef = (-1.0 / (sample_rate * att / 1000.0)).exp();
        let release_coef = (-1.0 / (sample_rate * rel / 1000.0)).exp();

        let coef = if abs_sample > self.envelope[channel] { attack_coef } else { release_coef };
        self.envelope[channel] = coef * self.envelope[channel] + (1.0 - coef) * abs_sample;

        let env_db = 20.0 * self.envelope[channel].max(0.000001).log10();
        let threshold = self.threshold.get_current_value() as f64;
        let ratio = self.ratio.get_current_value() as f64;
        let knee = self.knee.get_current_value() as f64;

        let mut gain_reduction_db = 0.0;
        
        if knee > 0.1 {
            // Soft Knee implementation
            let knee_half = knee / 2.0;
            if env_db > threshold + knee_half {
                gain_reduction_db = (threshold - env_db) * (1.0 - 1.0 / ratio);
            } else if env_db > threshold - knee_half {
                let diff = env_db - (threshold - knee_half);
                gain_reduction_db = (1.0 / ratio - 1.0) * diff * diff / (2.0 * knee);
            }
        } else if env_db > threshold {
            gain_reduction_db = (threshold - env_db) * (1.0 - 1.0 / ratio);
        }

        let gr_linear = 10.0f64.powf(gain_reduction_db / 20.0);
        self.reduction[channel] = gr_linear;

        let makeup_gain = 10.0f64.powf(self.gain.get_current_value() as f64 / 20.0);
        sample * gr_linear * makeup_gain
    }
}

pub struct MultibandDynamics {
    id: Uuid,
    crossovers: [LinkwitzRiley4th; 3],
    bands: [BandCompressor; 4],
    pub xover_freqs: [Parameter; 3],
}

impl MultibandDynamics {
    pub fn new(sample_rate: f64) -> Self {
        let freqs = [150.0, 2500.0, 8000.0];
        Self {
            id: Uuid::new_v4(),
            crossovers: [
                LinkwitzRiley4th::new(freqs[0], sample_rate),
                LinkwitzRiley4th::new(freqs[1], sample_rate),
                LinkwitzRiley4th::new(freqs[2], sample_rate),
            ],
            bands: [
                BandCompressor::new("Low", -10.0),
                BandCompressor::new("LowMid", -15.0),
                BandCompressor::new("HighMid", -15.0),
                BandCompressor::new("High", -10.0),
            ],
            xover_freqs: [
                Parameter::new("XOver 1", freqs[0], 20.0, 500.0),
                Parameter::new("XOver 2", freqs[1], 500.0, 5000.0),
                Parameter::new("XOver 3", freqs[2], 5000.0, 20000.0),
            ],
        }
    }
}

impl AudioProcessor for MultibandDynamics {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Multiband Dynamics".to_string()
    }

    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sr = context.sample_rate;

        // Update crossovers (slow path but safe for V1)
        for i in 0..3 {
            self.crossovers[i].update(self.xover_freqs[i].get_current_value() as f64, sr);
        }

        for c in 0..buffer.num_channels.min(2) {
            for i in 0..buffer.frames {
                let input = buffer.channels_data[c][i];

                // Crossover network (tree structure)
                let (low_half, high_half) = self.crossovers[1].process(input, c);
                let (band_0, band_1) = self.crossovers[0].process(low_half, c);
                let (band_2, band_3) = self.crossovers[2].process(high_half, c);

                // Compression per band
                let out_0 = self.bands[0].process(band_0, c, sr);
                let out_1 = self.bands[1].process(band_1, c, sr);
                let out_2 = self.bands[2].process(band_2, c, sr);
                let out_3 = self.bands[3].process(band_3, c, sr);

                // Summation (phase-coherent due to LR4)
                buffer.channels_data[c][i] = out_0 + out_1 + out_2 + out_3;
            }
        }
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        let mut p = Vec::new();
        for f in &mut self.xover_freqs { p.push(f); }
        for b in &mut self.bands {
            p.push(&mut b.threshold);
            p.push(&mut b.ratio);
            p.push(&mut b.attack);
            p.push(&mut b.release);
            p.push(&mut b.gain);
            p.push(&mut b.knee);
        }
        p
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            crossovers: self.crossovers.clone(),
            bands: self.bands.clone(),
            xover_freqs: [
                self.xover_freqs[0].clone(),
                self.xover_freqs[1].clone(),
                self.xover_freqs[2].clone(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::{AudioBuffer, ProcessingContext};

    #[test]
    fn test_multiband_dynamics_processing() {
        let mut mb = MultibandDynamics::new(44100.0);
        
        let mut buffer = AudioBuffer {
            channels_data: vec![vec![1.0; 4], vec![1.0; 4]],
            frames: 4,
            num_channels: 2,
        };
        
        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        
        // Let's set some extreme threshold to trigger gain reduction
        mb.bands[0].threshold.set_value(-40.0);
        mb.bands[0].ratio.set_value(10.0);
        mb.bands[0].attack.set_value(0.1);
        
        mb.process(&mut buffer, &context);
        
        // Output should not contain NaNs and should have a non-zero signal
        assert!(!buffer.channels_data[0][0].is_nan());
        assert!(buffer.channels_data[0][0] != 0.0);
    }
}
