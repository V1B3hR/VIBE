use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::sync::Arc;

pub const LUT_SIZE: usize = 4096;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum FadeType {
    Linear,
    EqualPower,
    SCurve,
}

#[derive(Clone)]
pub struct FadeLuts {
    pub linear: Arc<Vec<f32>>,
    pub equal_power_in: Arc<Vec<f32>>,
    pub equal_power_out: Arc<Vec<f32>>,
    pub s_curve: Arc<Vec<f32>>,
}

impl FadeLuts {
    pub fn new() -> Self {
        let mut linear = Vec::with_capacity(LUT_SIZE);
        let mut eq_in = Vec::with_capacity(LUT_SIZE);
        let mut eq_out = Vec::with_capacity(LUT_SIZE);
        let mut s_curve = Vec::with_capacity(LUT_SIZE);

        for i in 0..LUT_SIZE {
            let t = i as f32 / (LUT_SIZE - 1) as f32;

            // Linear
            linear.push(t);

            // Equal Power (Constant Power)
            // In: sin(t * PI/2)
            // Out: cos(t * PI/2)
            eq_in.push((t * PI / 2.0).sin());
            eq_out.push((t * PI / 2.0).cos());

            // S-Curve (Smoothstep / Hermite)
            // 3t^2 - 2t^3
            s_curve.push(3.0 * t * t - 2.0 * t * t * t);
        }

        Self {
            linear: Arc::new(linear),
            equal_power_in: Arc::new(eq_in),
            equal_power_out: Arc::new(eq_out),
            s_curve: Arc::new(s_curve),
        }
    }

    pub fn get_gain(&self, fade_type: &FadeType, progress: f32, is_fade_in: bool) -> f32 {
        let idx = (progress.clamp(0.0, 1.0) * (LUT_SIZE - 1) as f32) as usize;

        match fade_type {
            FadeType::Linear => {
                if is_fade_in {
                    self.linear[idx]
                } else {
                    1.0 - self.linear[idx]
                }
            }
            FadeType::EqualPower => {
                if is_fade_in {
                    self.equal_power_in[idx]
                } else {
                    self.equal_power_out[idx]
                }
            }
            FadeType::SCurve => {
                if is_fade_in {
                    self.s_curve[idx]
                } else {
                    1.0 - self.s_curve[idx]
                }
            }
        }
    }
}
