use super::graph::Parameter;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// A VCA Group for non-destructive relative volume control.
/// Inspirowany legendarnymi konsolami SSL i Neve.
#[derive(Clone, Serialize, Deserialize)]
pub struct VcaGroup {
    pub id: Uuid,
    pub name: String,
    pub member_tracks: Vec<Uuid>,
    pub gain: Parameter,
    pub is_muted: bool,
    pub is_solo: bool,
}

impl VcaGroup {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            member_tracks: Vec::new(),
            gain: Parameter::new("VCA Gain", 1.0, 0.0, 2.0),
            is_muted: false,
            is_solo: false,
        }
    }

    pub fn add_track(&mut self, track_id: Uuid) {
        if !self.member_tracks.contains(&track_id) {
            self.member_tracks.push(track_id);
        }
    }

    pub fn remove_track(&mut self, track_id: Uuid) {
        self.member_tracks.retain(|&id| id != track_id);
    }

    pub fn get_effective_gain(&self) -> f64 {
        if self.is_muted {
            0.0
        } else {
            self.gain.get_current_value()
        }
    }
}
