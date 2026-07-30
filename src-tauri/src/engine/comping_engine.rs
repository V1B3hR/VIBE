use crate::engine::take_lanes::{CompRegion, TakeLaneManager};
use uuid::Uuid;

/// The engine responsible for "Quick Swipe Comping" logic.
/// Resolves which take should be heard at any given time.
#[allow(dead_code)]
pub struct CompingEngine {
    pub managers: Vec<TakeLaneManager>,
}

#[allow(dead_code)]
impl CompingEngine {
    pub fn new() -> Self {
        Self {
            managers: Vec::new(),
        }
    }

    /// Automatically resolves overlapping regions by cutting them.
    /// Implementation of "Quick Swipe" principle: the newest swipe wins.
    pub fn swipe_region(&mut self, track_id: Uuid, new_region: CompRegion) {
        if let Some(manager) = self.managers.iter_mut().find(|m| m.track_id == track_id) {
            let start = new_region.start_beats;
            let end = new_region.end_beats;

            // 1. Filter out regions that are completely covered
            manager
                .comp_regions
                .retain(|r| !(r.start_beats >= start && r.end_beats <= end));

            // 2. Handle partial overlaps
            let mut to_add = Vec::new();
            let mut to_modify = Vec::new();

            for (i, r) in manager.comp_regions.iter().enumerate() {
                if r.start_beats < start && r.end_beats > end {
                    // Split bridge: the old region completely contains the new one
                    // We need to split the old one into two
                    let mut second_half = r.clone();
                    second_half.id = Uuid::new_v4();
                    second_half.start_beats = end;
                    to_add.push(second_half);

                    to_modify.push((i, r.start_beats, start));
                } else if r.start_beats < start && r.end_beats > start {
                    // Overlap at the end of existing region
                    to_modify.push((i, r.start_beats, start));
                } else if r.start_beats < end && r.end_beats > end {
                    // Overlap at the beginning of existing region
                    to_modify.push((i, end, r.end_beats));
                }
            }

            // Apply modifications
            for (idx, new_start, new_end) in to_modify {
                manager.comp_regions[idx].start_beats = new_start;
                manager.comp_regions[idx].end_beats = new_end;
            }

            // Add the split halves
            manager.comp_regions.extend(to_add);

            // 3. Insert the new region
            manager.add_comp_region(new_region);
        }
    }

    /// Returns the active take ID for a specific timeline position.
    pub fn get_active_take_at(&self, track_id: Uuid, position_beats: f64) -> Option<Uuid> {
        self.managers
            .iter()
            .find(|m| m.track_id == track_id)?
            .comp_regions
            .iter()
            .find(|r| position_beats >= r.start_beats && position_beats < r.end_beats)
            .map(|r| r.take_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_swipe_overlap() {
        let mut engine = CompingEngine::new();
        let track_id = Uuid::new_v4();
        let take1 = Uuid::new_v4();
        let take2 = Uuid::new_v4();

        engine.managers.push(TakeLaneManager::new(track_id));

        // Swipe 1: Beats 0 to 4 from Take 1
        engine.swipe_region(
            track_id,
            CompRegion {
                id: Uuid::new_v4(),
                take_id: take1,
                start_beats: 0.0,
                end_beats: 4.0,
                fade_in_ms: 0.0,
                fade_out_ms: 0.0,
            },
        );

        // Swipe 2: Beats 2 to 6 from Take 2 (Overlaps half of Take 1)
        engine.swipe_region(
            track_id,
            CompRegion {
                id: Uuid::new_v4(),
                take_id: take2,
                start_beats: 2.0,
                end_beats: 6.0,
                fade_in_ms: 0.0,
                fade_out_ms: 0.0,
            },
        );

        let manager = &engine.managers[0];
        assert_eq!(manager.comp_regions.len(), 2);

        // Take 1 should now be 0 to 2
        assert_eq!(manager.comp_regions[0].start_beats, 0.0);
        assert_eq!(manager.comp_regions[0].end_beats, 2.0);

        // Take 2 should be 2 to 6
        assert_eq!(manager.comp_regions[1].start_beats, 2.0);
        assert_eq!(manager.comp_regions[1].end_beats, 6.0);
    }

    #[test]
    fn test_quick_swipe_split() {
        let mut engine = CompingEngine::new();
        let track_id = Uuid::new_v4();
        let take1 = Uuid::new_v4();
        let take2 = Uuid::new_v4();

        engine.managers.push(TakeLaneManager::new(track_id));

        // Swipe 1: Beats 0 to 10 from Take 1
        engine.swipe_region(
            track_id,
            CompRegion {
                id: Uuid::new_v4(),
                take_id: take1,
                start_beats: 0.0,
                end_beats: 10.0,
                fade_in_ms: 0.0,
                fade_out_ms: 0.0,
            },
        );

        // Swipe 2: Beats 4 to 6 from Take 2 (Splits Take 1)
        engine.swipe_region(
            track_id,
            CompRegion {
                id: Uuid::new_v4(),
                take_id: take2,
                start_beats: 4.0,
                end_beats: 6.0,
                fade_in_ms: 0.0,
                fade_out_ms: 0.0,
            },
        );

        let manager = &engine.managers[0];
        // Should have 3 regions: [0-4], [4-6], [6-10]
        assert_eq!(manager.comp_regions.len(), 3);

        // After sorting by start_beats:
        assert_eq!(manager.comp_regions[0].end_beats, 4.0); // [0-4]
        assert_eq!(manager.comp_regions[1].start_beats, 4.0); // [4-6] (New swipe)
        assert_eq!(manager.comp_regions[1].end_beats, 6.0);
        assert_eq!(manager.comp_regions[2].start_beats, 6.0); // [6-10] (Split part)
    }
}
