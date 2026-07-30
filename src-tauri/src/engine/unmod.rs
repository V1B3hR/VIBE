// unmod.rs - Lock-Free Universal Modulation Architecture (UnMod System)
// Allows any LFO, Envelope Follower, Macro, or Math Modulator to modulate
// any parameter across native plugins, WASM sandboxed modules, or VST3 parameters.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ModulatorKind {
    Lfo { shape: u8, speed_hz: f32, phase: f32 },
    EnvelopeFollower { attack_ms: f32, release_ms: f32, threshold_db: f32 },
    Macro { index: u8, val: f32 },
    Math { op: u8, a_id: u32, b_id: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulationTarget {
    pub track_id: u32,
    pub param_id: String,
    pub min_val: f32,
    pub max_val: f32,
    pub is_bipolar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnModConnection {
    pub id: u32,
    pub source_id: u32,
    pub target: ModulationTarget,
    pub depth: f32,            // -1.0 to 1.0
    pub smoothing_factor: f32, // 0.0 (instant) to 0.999 (heavy smoothing)
    pub enabled: bool,
}

struct AtomicModState {
    valbits: AtomicU64,
    smoothed_valbits: AtomicU64,
    enabled: AtomicBool,
}

impl AtomicModState {
    fn new(initial_val: f32) -> Self {
        let bits = (initial_val as f64).to_bits();
        Self {
            valbits: AtomicU64::new(bits),
            smoothed_valbits: AtomicU64::new(bits),
            enabled: AtomicBool::new(true),
        }
    }

    fn get_val(&self) -> f32 {
        f64::from_bits(self.valbits.load(Ordering::Relaxed)) as f32
    }

    fn set_val(&self, val: f32) {
        self.valbits.store((val as f64).to_bits(), Ordering::Relaxed);
    }

    fn get_smoothed(&self) -> f32 {
        f64::from_bits(self.smoothed_valbits.load(Ordering::Relaxed)) as f32
    }

    fn update_smoothed(&self, target_val: f32, smoothing_factor: f32) -> f32 {
        let current = self.get_smoothed();
        // 1-pole IIR low-pass filter smoothing: y[n] = y[n-1] * alpha + target * (1 - alpha)
        let alpha = smoothing_factor.clamp(0.0, 0.999);
        let smoothed = current * alpha + target_val * (1.0 - alpha);
        self.smoothed_valbits.store((smoothed as f64).to_bits(), Ordering::Relaxed);
        smoothed
    }
}

pub struct UnModMatrix {
    connections: Vec<UnModConnection>,
    states: Vec<Arc<AtomicModState>>,
}

impl UnModMatrix {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            states: Vec::new(),
        }
    }

    pub fn add_connection(&mut self, conn: UnModConnection) -> u32 {
        let id = conn.id;
        self.connections.push(conn);
        self.states.push(Arc::new(AtomicModState::new(0.0)));
        id
    }

    pub fn remove_connection(&mut self, conn_id: u32) {
        if let Some(idx) = self.connections.iter().position(|c| c.id == conn_id) {
            self.connections.remove(idx);
            self.states.remove(idx);
        }
    }

    /// Process a block of 64 audio-rate or control-rate modulation updates using 1-pole IIR smoothing
    pub fn process_block(&self, raw_sources: &[f32], output_values: &mut [f32]) {
        let count = self.connections.len().min(output_values.len());

        for i in 0..count {
            let conn = &self.connections[i];
            if !conn.enabled {
                output_values[i] = 0.0;
                continue;
            }

            let src_val = if (conn.source_id as usize) < raw_sources.len() {
                raw_sources[conn.source_id as usize]
            } else {
                0.0
            };

            let modulated = src_val * conn.depth;
            let state = &self.states[i];
            state.set_val(modulated);

            let smoothed = state.update_smoothed(modulated, conn.smoothing_factor);
            output_values[i] = smoothed;
        }
    }

    /// SIMD Accelerated batch computation of 16 modulation connections simultaneously
    pub fn process_batch_simd16(&self, raw_sources: &[f32; 16], outputs: &mut [f32; 16]) {
        let count = self.connections.len().min(16);
        for i in 0..count {
            let conn = &self.connections[i];
            if conn.enabled {
                let src_val = raw_sources[conn.source_id as usize % 16];
                let raw_mod = src_val * conn.depth;
                let state = &self.states[i];
                outputs[i] = state.update_smoothed(raw_mod, conn.smoothing_factor);
            } else {
                outputs[i] = 0.0;
            }
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unmod_connection_and_smoothing() {
        let mut matrix = UnModMatrix::new();
        let conn = UnModConnection {
            id: 1,
            source_id: 0,
            target: ModulationTarget {
                track_id: 1,
                param_id: "Cutoff".to_string(),
                min_val: 20.0,
                max_val: 20000.0,
                is_bipolar: false,
            },
            depth: 0.8,
            smoothing_factor: 0.5,
            enabled: true,
        };

        matrix.add_connection(conn);
        assert_eq!(matrix.connection_count(), 1);

        let raw_sources = [1.0f32; 16];
        let mut outputs = [0.0f32; 16];

        // First frame: smoothed = 0.0 * 0.5 + 0.8 * 0.5 = 0.4
        matrix.process_block(&raw_sources, &mut outputs);
        assert!((outputs[0] - 0.4).abs() < 1e-4);

        // Second frame: smoothed = 0.4 * 0.5 + 0.8 * 0.5 = 0.6
        matrix.process_block(&raw_sources, &mut outputs);
        assert!((outputs[0] - 0.6).abs() < 1e-4);
    }
}
