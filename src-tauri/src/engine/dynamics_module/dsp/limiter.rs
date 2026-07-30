use super::super::super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use std::collections::VecDeque;
use uuid::Uuid;

pub struct LookaheadLimiter {
    id: Uuid,
    name: String,
    parameters: Vec<Parameter>,

    // DSP State
    lookahead_ms: f64,
    attack_ms: f64,
    release_ms: f64,
    threshold_db: f64,
    ceiling_db: f64,

    delay_buffers: Vec<VecDeque<f64>>,

    current_gain: f64,
    sample_rate: f64,
}

impl LookaheadLimiter {
    pub fn new(sample_rate: f64) -> Self {
        let mut params = Vec::new();
        params.push(Parameter::new("Threshold", -0.1, -24.0, 0.0));
        params.push(Parameter::new("Ceiling", -0.1, -24.0, 0.0));
        params.push(Parameter::new("Release", 100.0, 1.0, 500.0));
        params.push(Parameter::new("Lookahead", 5.0, 0.0, 20.0)); // 5ms lookahead default

        Self {
            id: Uuid::new_v4(),
            name: "VIBE Limiter".to_string(),
            parameters: params,
            lookahead_ms: 5.0,
            attack_ms: 0.1, // Near instant attack for limiter
            release_ms: 100.0,
            threshold_db: -0.1,
            ceiling_db: -0.1,
            delay_buffers: (0..2).map(|_| VecDeque::new()).collect(),
            current_gain: 1.0,
            sample_rate,
        }
    }

    fn update_parameters(&mut self) {
        self.threshold_db = self.parameters[0].value;
        self.ceiling_db = self.parameters[1].value;
        self.release_ms = self.parameters[2].value;
        self.lookahead_ms = self.parameters[3].value;
    }

    pub fn process_block(&mut self, buffer: &mut AudioBuffer) {
        let frames = buffer.frames;
        let num_chans = buffer.num_channels;

        // Ensure we have enough delay buffers
        while self.delay_buffers.len() < num_chans {
            self.delay_buffers.push(VecDeque::new());
        }

        let threshold_linear = 10.0f64.powf(self.threshold_db / 20.0);
        let ceiling_linear = 10.0f64.powf(self.ceiling_db / 20.0);
        let lookahead_samples = (self.lookahead_ms * 0.001 * self.sample_rate) as usize;

        let alpha_att = (-1.0 / (0.001 * self.attack_ms * self.sample_rate)).exp();
        let alpha_rel = (-1.0 / (0.001 * self.release_ms * self.sample_rate)).exp();

        for i in 0..frames {
            // 1. Peak detection across all channels (Unified Gain Reduction)
            let mut peak_input: f64 = 0.0;
            for c in 0..num_chans {
                let s = buffer.channels_data[c][i];
                self.delay_buffers[c].push_back(s);
                peak_input = peak_input.max(s.abs());
            }

            // 2. Calculate target gain
            let mut target_gain = 1.0;
            if peak_input > threshold_linear {
                target_gain = threshold_linear / peak_input;
            }

            // 3. Smooth gain
            if target_gain < self.current_gain {
                self.current_gain = alpha_att * self.current_gain + (1.0 - alpha_att) * target_gain;
            } else {
                self.current_gain = alpha_rel * self.current_gain + (1.0 - alpha_rel) * target_gain;
            }

            // 4. Apply gain to delayed signals
            if self.delay_buffers[0].len() > lookahead_samples {
                for c in 0..num_chans {
                    let out = self.delay_buffers[c].pop_front().unwrap_or(0.0);
                    buffer.channels_data[c][i] = out * self.current_gain * ceiling_linear;
                }
            } else {
                for c in 0..num_chans {
                    buffer.channels_data[c][i] = 0.0;
                }
            }
        }
    }

    pub fn process_stereo(&mut self, l: f64, r: f64) -> (f64, f64) {
        // Ensure buffers
        while self.delay_buffers.len() < 2 {
            self.delay_buffers.push(VecDeque::new());
        }

        let threshold_linear = 10.0f64.powf(self.threshold_db / 20.0);
        let ceiling_linear = 10.0f64.powf(self.ceiling_db / 20.0);
        let lookahead_samples = (self.lookahead_ms * 0.001 * self.sample_rate) as usize;

        // Attack/Release coeffs
        let alpha_att = (-1.0 / (0.001 * self.attack_ms * self.sample_rate)).exp();
        let alpha_rel = (-1.0 / (0.001 * self.release_ms * self.sample_rate)).exp();

        // 1. Peak detection
        self.delay_buffers[0].push_back(l);
        self.delay_buffers[1].push_back(r);
        let peak_input = l.abs().max(r.abs());

        // 2. Target Gain
        let mut target_gain = 1.0;
        if peak_input > threshold_linear {
            target_gain = threshold_linear / peak_input;
        }

        // 3. Smooth Gain
        if target_gain < self.current_gain {
            self.current_gain = alpha_att * self.current_gain + (1.0 - alpha_att) * target_gain;
        } else {
            self.current_gain = alpha_rel * self.current_gain + (1.0 - alpha_rel) * target_gain;
        }

        // 4. Output
        if self.delay_buffers[0].len() > lookahead_samples {
            let out_l = self.delay_buffers[0].pop_front().unwrap_or(0.0);
            let out_r = self.delay_buffers[1].pop_front().unwrap_or(0.0);

            (
                out_l * self.current_gain * ceiling_linear,
                out_r * self.current_gain * ceiling_linear,
            )
        } else {
            (0.0, 0.0)
        }
    }
}

impl AudioProcessor for LookaheadLimiter {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
        }
        self.update_parameters();
        self.process_block(buffer);
    }

    fn id(&self) -> Uuid {
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        self.parameters.iter_mut().collect()
    }
    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self::new(self.sample_rate))
    }
}
