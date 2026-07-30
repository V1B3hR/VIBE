use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Represents a single recording pass or an alternative version of a clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Take {
    pub id: Uuid,
    pub name: String,
    pub clip_id: Uuid, // Reference to the underlying AudioClip
    pub start_offset_beats: f64,
}

/// Represents a chosen segment from a specific Take.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CompRegion {
    pub id: Uuid,
    pub take_id: Uuid,
    pub start_beats: f64,
    pub end_beats: f64,
    pub fade_in_ms: f64,
    pub fade_out_ms: f64,
}

/// Manages multiple take lanes for a single track.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TakeLaneManager {
    pub track_id: Uuid,
    pub takes: Vec<Take>,
    pub comp_regions: Vec<CompRegion>,
    pub is_expanded: bool,
}

#[allow(dead_code)]
impl TakeLaneManager {
    pub fn new(track_id: Uuid) -> Self {
        Self {
            track_id,
            takes: Vec::new(),
            comp_regions: Vec::new(),
            is_expanded: false,
        }
    }

    pub fn add_take(&mut self, take: Take) {
        self.takes.push(take);
    }

    pub fn add_comp_region(&mut self, region: CompRegion) {
        // Logika zapobiegająca nakładaniu się regionów (Quick Swipe Comping logic)
        self.comp_regions.push(region);
        self.sort_regions();
    }

    fn sort_regions(&mut self) {
        self.comp_regions.sort_by(|a, b| a.start_beats.partial_cmp(&b.start_beats).unwrap());
    }
}
