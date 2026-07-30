use crate::engine::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use uuid::Uuid;

/// The "Secret Sauce" Master Processor
/// Implements Analog Summing (Crosstalk), Transient Shaping, and Psychoacoustic EQ.
pub struct PsychoacousticEngine {
    pub id: Uuid,
    pub crosstalk: Parameter,
    pub punch: Parameter,
    pub drive: Parameter,
    pub ceiling: Parameter,
    // Transient Shaper State
    prev_envelope: [f64; 2],
    // HPF State for Sidechain (Simple 1-pole)
    hpf_state: [f64; 2],
}

impl PsychoacousticEngine {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            crosstalk: Parameter::new("Summing Crosstalk", 0.005, 0.0, 0.05),
            punch: Parameter::new("Transient Punch", 0.9, 0.0, 2.0),
            drive: Parameter::new("Drive", 0.0, 0.0, 12.0), // dB
            ceiling: Parameter::new("Ceiling", -0.1, -6.0, 0.0), // dB
            prev_envelope: [0.0, 0.0],
            hpf_state: [0.0, 0.0],
        }
    }
}

impl AudioProcessor for PsychoacousticEngine {
    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        let frames = buffer.frames;
        let ct = self.crosstalk.get_current_value();
        let punch = self.punch.get_current_value();
        let drive_db = self.drive.get_current_value();
        let ceiling_db = self.ceiling.get_current_value();

        let drive_lin = 10.0f64.powf(drive_db / 20.0);
        let ceiling_lin = 10.0f64.powf(ceiling_db / 20.0);

        // Soft Clip Threshold relative to ceiling
        // We want to start saturating slightly below the ceiling
        let saturation_thresh = ceiling_lin * 0.9;

        // Optimized constants for 3-5kHz air/transient band
        let attack_coeff = 0.01;
        let release_coeff = 0.001;

        // Sidechain HPF (approx 100Hz at 48k)
        // alpha = 1 / (1 + 1 / (2 * pi * dt * fc)) roughly,
        // 1-pole HPF: y[n] = alpha * (y[n-1] + x[n] - x[n-1])
        // Simplified memoryless for envelope: just filter input to envelope
        let hpf_alpha = 0.9;

        for i in 0..frames {
            let left = buffer.channels_data[0][i];
            let right = buffer.channels_data[1][i];

            // 1. Analog Summing Emulation (Crosstalk)
            // Mixing L into R and R into L slightly to "glue" the stereo image.
            let mut l_proc = left + right * ct;
            let mut r_proc = right + left * ct;

            // Apply Drive
            l_proc *= drive_lin;
            r_proc *= drive_lin;

            // 2. Transient Shaping (Punch) with HPF Sidechain
            // Detects fast attacks and applies micro-gain to them.
            for c in 0..2 {
                let current_sample = if c == 0 { l_proc } else { r_proc };

                // HPF for Sidechain (keeps low end from pumping)
                // Simple high pass: y = x - lowpass
                // Let's use a simple state tracking for LP
                let lp_out =
                    self.hpf_state[c] + (1.0 - hpf_alpha) * (current_sample - self.hpf_state[c]);
                self.hpf_state[c] = lp_out;

                let hp_sample = current_sample - lp_out;
                let abs_sample = hp_sample.abs();

                // Simple envelope follower on HP filtered signal
                let coeff = if abs_sample > self.prev_envelope[c] {
                    attack_coeff
                } else {
                    release_coeff
                };
                self.prev_envelope[c] += coeff * (abs_sample - self.prev_envelope[c]);

                // Calculate transient gain boost
                let delta = abs_sample - self.prev_envelope[c];
                let boost = if delta > 0.0 {
                    1.0 + delta * punch
                } else {
                    1.0
                };

                if c == 0 {
                    l_proc *= boost;
                } else {
                    r_proc *= boost;
                }
            }

            // 3. Transparent Soft-Clip Limiter
            for (samp, out_buf) in [l_proc, r_proc].iter().zip(buffer.channels_data.iter_mut()) {
                let s = *samp;
                let abs_s = s.abs();

                if abs_s <= saturation_thresh {
                    // Linear region
                    out_buf[i] = s;
                } else if abs_s < ceiling_lin * 1.1 {
                    // Soft knee region using tanh for tube-like saturation
                    // Map [thresh, inf] -> [thresh, ceiling] gently
                    let over = abs_s - saturation_thresh;
                    let range = ceiling_lin - saturation_thresh;
                    let sat = (over / range).tanh() * range;
                    out_buf[i] = s.signum() * (saturation_thresh + sat);
                } else {
                    // Hard limit (safety)
                    out_buf[i] = s.signum() * ceiling_lin;
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.crosstalk,
            &mut self.punch,
            &mut self.drive,
            &mut self.ceiling,
        ]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            crosstalk: self.crosstalk.clone(),
            punch: self.punch.clone(),
            drive: self.drive.clone(),
            ceiling: self.ceiling.clone(),
            prev_envelope: self.prev_envelope,
            hpf_state: self.hpf_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::AudioBuffer;

    #[test]
    fn test_psycho_instantiation() {
        let _engine = PsychoacousticEngine::new();
    }

    #[test]
    fn test_psycho_processing_no_crash() {
        let mut engine = PsychoacousticEngine::new();
        let mut buffer = AudioBuffer::new();
        buffer.frames = 100;
        buffer.num_channels = 2;

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };

        engine.process(&mut buffer, &context);
    }

    #[test]
    fn test_crosstalk_leakage() {
        let mut engine = PsychoacousticEngine::new();
        engine.crosstalk.set_value(0.1); // 10% crosstalk

        let mut buffer = AudioBuffer::new();
        buffer.frames = 10;
        buffer.num_channels = 2;

        // Signal only on Left
        for i in 0..10 {
            buffer.channels_data[0][i] = 1.0;
            buffer.channels_data[1][i] = 0.0;
        }

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        engine.process(&mut buffer, &context);

        // Right channel should now have signal
        let mut max_r = 0.0;
        for i in 0..10 {
            if buffer.channels_data[1][i].abs() > max_r {
                max_r = buffer.channels_data[1][i].abs();
            }
        }
        assert!(max_r > 0.0, "Crosstalk failed to leak signal to R channel");
    }

    #[test]
    fn test_limiting_ceiling() {
        let mut engine = PsychoacousticEngine::new();
        // Set ceiling to -6dB (~0.5)
        engine.ceiling.set_value(-6.0);
        // Drive input hard (10.0)

        let mut buffer = AudioBuffer::new();
        buffer.frames = 10;
        buffer.num_channels = 2;
        for i in 0..10 {
            buffer.channels_data[0][i] = 10.0;
            buffer.channels_data[1][i] = 10.0;
        }

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        engine.process(&mut buffer, &context);

        // Output should be limited near 0.5 (linear)
        let limit = 10.0f64.powf(-6.0 / 20.0); // ~0.501

        for i in 0..10 {
            assert!(
                buffer.channels_data[0][i].abs() <= limit + 0.1,
                "Limiter failed to hold ceiling at index {}",
                i
            );
        }
    }
}
