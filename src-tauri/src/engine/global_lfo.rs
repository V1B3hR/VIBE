#![allow(dead_code)]
pub enum LfoWaveform {
    Sine,
    Triangle,
    Saw,
    Square,
    Random,
}

/// A Global LFO that can be assigned anywhere.
pub struct GlobalLfo {
    pub waveform: LfoWaveform,
    pub frequency: f32, // Hz
    pub phase: f32,     // 0.0 - 1.0
    pub sync_to_bpm: bool,
}

impl GlobalLfo {
    pub fn new() -> Self {
        Self {
            waveform: LfoWaveform::Sine,
            frequency: 1.0,
            phase: 0.0,
            sync_to_bpm: false,
        }
    }

    /// Calculate next LFO value.
    pub fn tick(&mut self, _sample_rate: f64) -> f32 {
        // Waveform generation logic
        0.0
    }
}
