#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a rhythmic shift and velocity deviation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrooveMoment {
    pub offset_ms: f32,     // Shift in milliseconds
    pub velocity_mult: f32, // Multiplier for velocity
    pub probability: f32,   // 0.0 to 1.0 probability of trigger
}

/// A named groove template (e.g., "MPC 60 16th Swing 62%").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrooveTemplate {
    pub name: String,
    pub description: String,
    pub quantization: u32, // e.g., 16 for 16th notes
    /// Map of grid position (0.0 to 1.0) to groove shift.
    pub points: HashMap<String, GrooveMoment>,
}

pub struct GroovePool {
    pub templates: Vec<GrooveTemplate>,
}

impl GroovePool {
    pub fn new() -> Self {
        let mut pool = Self {
            templates: Vec::new(),
        };
        pool.load_defaults();
        pool
    }

    fn load_defaults(&mut self) {
        // MPC-style Swing 16th (simplified)
        let mut mpc_points = HashMap::new();
        // Every even 16th note is shifted late
        mpc_points.insert(
            "0.0625".to_string(),
            GrooveMoment {
                offset_ms: 15.0,
                velocity_mult: 0.95,
                probability: 1.0,
            },
        );
        mpc_points.insert(
            "0.1875".to_string(),
            GrooveMoment {
                offset_ms: 15.0,
                velocity_mult: 0.95,
                probability: 1.0,
            },
        );

        self.templates.push(GrooveTemplate {
            name: "MPC 60 Classic Swing".to_string(),
            description: "Legendary 16th note swing with slight late offsets.".to_string(),
            quantization: 16,
            points: mpc_points,
        });

        // J Dilla "Lazy" Feel
        let mut dilla_points = HashMap::new();
        dilla_points.insert(
            "0.0".to_string(),
            GrooveMoment {
                offset_ms: 5.0,
                velocity_mult: 1.05,
                probability: 1.0,
            },
        ); // Early kick
        dilla_points.insert(
            "0.25".to_string(),
            GrooveMoment {
                offset_ms: -10.0,
                velocity_mult: 0.9,
                probability: 0.95,
            },
        ); // Late snare + slight ghosting

        self.templates.push(GrooveTemplate {
            name: "J Dilla Lazy Snare".to_string(),
            description: "Drunk-style groove with pushed kicks and late snares.".to_string(),
            quantization: 8,
            points: dilla_points,
        });
    }

    pub fn get_template(&self, name: &str) -> Option<&GrooveTemplate> {
        self.templates.iter().find(|t| t.name == name)
    }

    pub fn extract_from_transients(
        &self,
        name: String,
        transients_samples: Vec<u64>,
        sample_rate: f64,
        bpm: f64,
        grid_resolution: u32,
    ) -> GrooveTemplate {
        let mut points = HashMap::new();
        let samples_per_beat = (60.0 / bpm) * sample_rate;
        let grid_size = 1.0 / grid_resolution as f64;

        for sample_pos in transients_samples {
            let beat_pos = sample_pos as f64 / samples_per_beat;
            let normalized_grid_pos = beat_pos % 1.0;

            // Find nearest grid point
            let nearest_grid = (normalized_grid_pos / grid_size).round() * grid_size;
            let offset_beats = normalized_grid_pos - nearest_grid;
            let offset_ms = (offset_beats * (60.0 / bpm) * 1000.0) as f32;

            points.insert(
                format!("{:.4}", nearest_grid),
                GrooveMoment {
                    offset_ms,
                    velocity_mult: 1.0,
                    probability: 1.0,
                },
            );
        }

        GrooveTemplate {
            name,
            description: "Extracted from audio transients.".into(),
            quantization: grid_resolution,
            points,
        }
    }

    /// Generates a list of offsets for the UI "Shadow Grid"
    pub fn get_shadow_grid(&self, template_name: &str) -> Vec<(f32, f32)> {
        if let Some(template) = self.get_template(template_name) {
            template
                .points
                .iter()
                .map(|(pos, moment)| (pos.parse::<f32>().unwrap_or(0.0), moment.offset_ms))
                .collect()
        } else {
            vec![]
        }
    }
}
