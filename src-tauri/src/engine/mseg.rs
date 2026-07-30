#![allow(dead_code)]
pub struct MsegPoint {
    pub time_beats: f64,
    pub value: f32,
    pub curve: f32, // Concave/Convex amount
}

/// Multi-Segment Envelope Generator.
pub struct Mseg {
    pub points: Vec<MsegPoint>,
    pub loop_enabled: bool,
}

impl Mseg {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            loop_enabled: false,
        }
    }

    pub fn get_value_at(&self, _position: f64) -> f32 {
        // Interpolation logic between points
        1.0
    }
}
