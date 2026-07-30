#![allow(dead_code)]
use crate::engine::groove_pool::GrooveTemplate;
use rand::Rng;

pub struct HumanizationEngine {
    pub random_amount: f32, // 0.0 to 1.0
}

impl HumanizationEngine {
    pub fn new() -> Self {
        Self { random_amount: 0.1 }
    }

    /// Apply humanization and groove to a list of MIDI notes/events.
    /// Returns (final_timestamp, final_velocity, is_active).
    pub fn apply_groove(
        &self,
        timestamp: f64,
        velocity: u8,
        template: &GrooveTemplate,
        _bpm: f64,
    ) -> (f64, u8, bool) {
        let mut rng = rand::thread_rng();

        // 1. Apply Template Groove & Probability
        let grid_pos = format!("{:.4}", timestamp % 1.0);
        let (template_offset, template_vel, prob) =
            if let Some(moment) = template.points.get(&grid_pos) {
                (
                    moment.offset_ms as f64 / 1000.0,
                    moment.velocity_mult,
                    moment.probability,
                )
            } else {
                (0.0, 1.0, 1.0)
            };

        // Probability Layer
        if rng.gen::<f32>() > prob {
            return (timestamp, 0, false);
        }

        // 2. Apply "Micro-timing" Humanization
        let random_offset = (rng.gen::<f64>() - 0.5) * self.random_amount as f64 * 0.02; // max +-10ms
        let random_vel = 1.0 + (rng.gen::<f32>() - 0.5) * self.random_amount;

        let final_timestamp = timestamp + template_offset + random_offset;
        let final_velocity = (velocity as f32 * template_vel * random_vel).clamp(1.0, 127.0) as u8;

        (final_timestamp, final_velocity, true)
    }

    /// Extract groove from an audio buffer (Advanced Feature).
    /// Finds peaks and maps them to a temporary template.
    pub fn extract_groove_from_audio(
        &self,
        _samples: &[f32],
        _sample_rate: f64,
    ) -> Option<GrooveTemplate> {
        // Stub for intelligent groove extraction logic
        None
    }
}
