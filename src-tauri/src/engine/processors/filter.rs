use super::super::graph::{
    flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

pub struct BiquadFilter {
    id: Uuid,
    pub filter_type: Parameter, // 0=LP, 1=HP, 2=BP, 3=Notch
    pub cutoff: Parameter,      // 20.0 to 20000.0
    pub q: Parameter,           // 0.1 to 10.0
    pub gain: Parameter,        // For peaking/shelving

    // Coefficients
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,

    // State
    z1: [f64; 2],
    z2: [f64; 2],

    last_sample_rate: f64,
}

impl BiquadFilter {
    pub fn new(mode: FilterMode, cutoff: f64, q: f64) -> Self {
        let type_val = match mode {
            FilterMode::LowPass => 0.0,
            FilterMode::HighPass => 1.0,
            FilterMode::BandPass => 2.0,
            FilterMode::Notch => 3.0,
        };

        Self {
            id: Uuid::new_v4(),
            filter_type: Parameter::new("Type", type_val, 0.0, 3.9),
            cutoff: Parameter::new("Cutoff", cutoff, 20.0, 20000.0),
            q: Parameter::new("Q", q, 0.1, 10.0),
            gain: Parameter::new("Gain", 0.0, -24.0, 24.0),
            a1: 0.0,
            a2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            z1: [0.0; 2],
            z2: [0.0; 2],
            last_sample_rate: 44100.0,
        }
    }

    fn calculate_coefficients(&mut self, sample_rate: f64, playhead: u64) {
        let f0 = self.cutoff.get_value_at(playhead);
        let q = self.q.get_value_at(playhead);
        let type_val = self.filter_type.get_value_at(playhead).round() as i32;

        let w0 = 2.0 * PI * f0 / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2) = match type_val {
            0 => {
                // LowPass
                let b1 = 1.0 - cos_w0;
                let b0 = b1 / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            1 => {
                // HighPass
                let b1 = -(1.0 + cos_w0);
                let b0 = -b1 / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            2 => {
                // BandPass
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            _ => {
                // Notch (3 or fallback)
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
}

impl AudioProcessor for BiquadFilter {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        let playhead = context.playhead;
        // Redraw coefficients if params or sample rate changed
        // For efficiency, we only recalculate once per buffer (first sample)
        // unless we want per-sample modulation (expensive)
        self.calculate_coefficients(sample_rate, playhead);
        self.last_sample_rate = sample_rate;

        let frames = buffer.frames;
        let channels = buffer.num_channels;

        for c in 0..channels {
            let mut z1 = self.z1[c];
            let mut z2 = self.z2[c];

            for i in 0..frames {
                let x = buffer.channels_data[c][i];
                // Direct Form II
                let out = self.b0 * x + z1;
                z1 = self.b1 * x - self.a1 * out + z2;
                z2 = self.b2 * x - self.a2 * out;

                // Denormal protection
                let final_out = flush_denormal_f64(out);
                buffer.channels_data[c][i] = final_out;
            }

            self.z1[c] = flush_denormal_f64(z1);
            self.z2[c] = flush_denormal_f64(z2);
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "VIBE Filter".to_string()
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: vec![
                self.filter_type.clone(),
                self.cutoff.clone(),
                self.q.clone(),
                self.gain.clone(),
            ],
        })
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.filter_type,
            &mut self.cutoff,
            &mut self.q,
            &mut self.gain,
        ]
    }
}
