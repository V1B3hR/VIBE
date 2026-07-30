use super::super::graph::{
    flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext,
};
use std::f64::consts::PI;
use uuid::Uuid;

/// Vibe Professional Saturation ("Color" Update)
/// Features:
/// - Three Colors: Tube (Even harmonics), Tape (Odd/Soft), Solid (Hard)
/// - Focus Filter: High-Pass before saturation to keep low end clean (prevents "farting")
/// - Destroy Mode: Wavefolder for extreme modular-style distortion
pub struct VibeSaturation {
    id: Uuid,

    // Parameters
    pub drive: Parameter,    // 0.0 to 36.0 dB
    pub bias: Parameter,     // 0.0 to 1.0 (Asymmetry)
    pub sat_type: Parameter, // 0=Tube, 1=Tape, 2=Solid
    pub output: Parameter,   // -24 to +24 dB

    // "Color" Update Params
    pub focus: Parameter,   // 20Hz to 400Hz (Pre-Sat HPF cutoff)
    pub destroy: Parameter, // 0.0 or 1.0 (Wavefolder enable)
    pub mix: Parameter,     // 0.0 to 1.0

    // Filter State (Stereo)
    hp_l: Filter,
    hp_r: Filter,
}

struct Filter {
    z1: f64,
}
impl Filter {
    fn new() -> Self {
        Self { z1: 0.0 }
    }
    // Simple 1-pole HPF: y[n] = alpha * (y[n-1] + x[n] - x[n-1])
    fn process(&mut self, input: f64, cutoff: f64, sr: f64) -> f64 {
        if cutoff <= 20.0 {
            return input;
        }

        // Simpler 1-pole Highpass using Lowpass subtraction:
        // y[n] = x[n] - lp[n]
        let rc = 1.0 / (2.0 * PI * cutoff);
        let dt = 1.0 / sr;
        let alpha_lp = dt / (dt + rc);

        self.z1 = self.z1 + alpha_lp * (input - self.z1);
        self.z1 = flush_denormal_f64(self.z1);

        input - self.z1
    }
}

impl VibeSaturation {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            drive: Parameter::new("Drive", 0.0, 0.0, 36.0),
            bias: Parameter::new("Bias", 0.0, 0.0, 1.0),
            sat_type: Parameter::new("Type", 0.0, 0.0, 2.0),
            output: Parameter::new("Output", 0.0, -24.0, 24.0),

            focus: Parameter::new("Focus", 20.0, 20.0, 400.0),
            destroy: Parameter::new("Destroy", 0.0, 0.0, 1.0),
            mix: Parameter::new("Mix", 1.0, 0.0, 1.0),

            hp_l: Filter::new(),
            hp_r: Filter::new(),
        }
    }

    fn wavefolder(x: f64) -> f64 {
        // Simple Sine fold
        // x * sin(x)? No.
        // If |x| > 1, fold back.
        // Hardcore fold:
        let drive = 2.0; // Extra internal gain
        let val = x * drive;
        (val * 1.5).sin() // Typical modular West-coast sound
    }
}

impl AudioProcessor for VibeSaturation {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        let playhead = context.playhead;
        let drive_db = self.drive.get_value_at(playhead);
        let bias = self.bias.get_value_at(playhead);
        let mode = self.sat_type.get_value_at(playhead).round() as i32;
        let out_db = self.output.get_value_at(playhead);
        let focus_freq = self.focus.get_value_at(playhead);
        let destroy_mode = self.destroy.get_value_at(playhead) > 0.5;
        let mix = self.mix.get_value_at(playhead);

        let drive_lin = 10.0f64.powf(drive_db / 20.0);
        let out_lin = 10.0f64.powf(out_db / 20.0);

        let frames = buffer.frames;
        let channels = buffer.num_channels;

        for i in 0..frames {
            let in_l = buffer.channels_data[0][i];
            let in_r = if channels > 1 {
                buffer.channels_data[1][i]
            } else {
                in_l
            };

            // 1. Focus Filter (Split band)
            // We want to saturate only Highs, but keep Lows clean?
            // "Focus" means HPF enters saturation. Lows are bypassed?
            // Usually "Focus" filters the signal GOING into saturation, and the result is mixed?
            // Or "Multiband".
            // Implementation: Split = HP + LP. Sat = Sat(HP) + LP.

            // Calculate HP
            let hp_l = self.hp_l.process(in_l, focus_freq, sample_rate);
            let hp_r = self.hp_r.process(in_r, focus_freq, sample_rate);

            // LP is remainder (if phase coherent-ish)
            let lp_l = in_l - hp_l;
            let lp_r = in_r - hp_r;

            // 2. Drive
            let driven_l = hp_l * drive_lin;
            let driven_r = hp_r * drive_lin;

            // 3. Saturate
            let process_sat = |x: f64| -> f64 {
                if destroy_mode {
                    Self::wavefolder(x)
                } else {
                    match mode {
                        0 => {
                            // Tube
                            let biased = x + (bias * 0.5);
                            let sat = if biased > 1.0 {
                                (biased - 1.0).tanh() + 1.0
                            } else if biased < -1.0 {
                                (biased + 1.0).tanh() - 1.0
                            } else {
                                biased - (biased.powi(3) * 0.2)
                            };
                            sat - (bias * 0.5) // DC removal
                        }
                        1 => {
                            // Tape
                            let x_cl = x.clamp(-1.5, 1.5);
                            x_cl - (x_cl.powi(3) / 5.0)
                        }
                        _ => {
                            // Solid
                            x.clamp(-1.0, 1.0)
                        }
                    }
                }
            };

            let sat_l = process_sat(driven_l);
            let sat_r = process_sat(driven_r);

            // 4. Sum back (Clean Lows + Sat Highs)
            // Note: Phase issues possible if filter is not linear phase.
            // 1-pole IIR is minimum phase, summing key is 1 = HP + LP.
            // But Saturation is non-linear.

            let wet_l = sat_l + lp_l; // "Focus" keeps lows clean by not saturating them
            let wet_r = sat_r + lp_r;

            // 5. Output & Mix
            let final_wet_l = wet_l * out_lin;
            let final_wet_r = wet_r * out_lin;

            buffer.channels_data[0][i] = in_l * (1.0 - mix) + final_wet_l * mix;
            if channels > 1 {
                buffer.channels_data[1][i] = in_r * (1.0 - mix) + final_wet_r * mix;
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }
    fn name(&self) -> String {
        "VIBE Saturation".to_string()
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: vec![
                self.drive.clone(),
                self.bias.clone(),
                self.sat_type.clone(),
                self.output.clone(),
                self.focus.clone(),
                self.destroy.clone(),
                self.mix.clone(),
            ],
        })
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.drive,
            &mut self.bias,
            &mut self.sat_type,
            &mut self.output,
            &mut self.focus,
            &mut self.destroy,
            &mut self.mix,
        ]
    }
}
