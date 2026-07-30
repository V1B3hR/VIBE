#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use super::graph::{AudioBuffer, AudioProcessor, ProcessingContext};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub active: bool,
    pub velocity: u8,
    pub note: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSequence {
    pub steps: Vec<Step>,
    pub num_steps: usize,
    pub rate: f64, // e.g. 0.25 for 1/16th notes
}

/// StepSequencer: Grid-based MIDI pattern generator.
/// Optimized for Drum Rack style interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSequencer {
    pub id: Uuid,
    pub name: String,
    pub sequences: Vec<StepSequence>,
    pub current_step: usize,
    pub last_tick: u64,
}

impl StepSequencer {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Step Sequencer".to_string(),
            sequences: Vec::new(),
            current_step: 0,
            last_tick: 0,
        }
    }

    pub fn add_drum_lane(&mut self, note: u8, steps: usize) {
        let seq = StepSequence {
            steps: vec![Step { active: false, velocity: 100, note }; steps],
            num_steps: steps,
            rate: 0.25, // 1/16th
        };
        self.sequences.push(seq);
    }
}

impl AudioProcessor for StepSequencer {
    fn process(&mut self, _buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        // Step sequencing logic usually happens in the MIDI dispatch layer, 
        // but here we can generate MIDI events if needed.
        // For VIBE, we'll implement this as a MIDI generator.
    }

    fn on_midi_event(&mut self, _status: u8, _data1: u16, _data2: u32) {
        // Sequencer can be triggered by MIDI input
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(self.clone())
    }
}
