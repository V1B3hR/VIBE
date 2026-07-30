#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSurfaceProfile {
    pub id: Uuid,
    pub name: String,
    pub script_path: String,
    pub midi_input_id: Option<String>,
    pub midi_output_id: Option<String>,
}

/// ControlSurfaceManager handles external hardware controller integration.
/// Supports script-based mapping for devices like Launchpad, Push, and MIDI keyboards.
pub struct ControlSurfaceManager {
    profiles: HashMap<Uuid, ControlSurfaceProfile>,
    // TODO: Add ScriptEngine (e.g., Rhai or Python) for real-time MIDI processing
}

impl ControlSurfaceManager {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    pub fn register_profile(&mut self, profile: ControlSurfaceProfile) {
        self.profiles.insert(profile.id, profile);
    }

    /// Process incoming MIDI from a control surface
    pub fn handle_midi_in(&self, profile_id: Uuid, _status: u8, _data1: u8, _data2: u8) {
        if let Some(_profile) = self.profiles.get(&profile_id) {
            // TODO: Execute script hook: onMidiReceived(status, d1, d2)
            println!("VIBE: Control Surface [{}] received MIDI", _profile.name);
        }
    }
}
