#![allow(dead_code)]
use std::collections::HashMap;
use uuid::Uuid;

// Clip Launcher: Session View logic
pub struct TrackState {
    pub active_clip_id: Option<Uuid>,
    pub queued_clip_id: Option<Uuid>,
}

pub struct ClipLauncher {
    pub track_states: HashMap<usize, TrackState>,
    pub loop_quantization: u64, // samples (e.g. 1 bar)
}

impl ClipLauncher {
    pub fn new() -> Self {
        Self {
            track_states: HashMap::new(),
            loop_quantization: 44100 * 2, // Default 1 bar at 120bpm/44.1k
        }
    }

    #[allow(dead_code)]
    pub fn trigger_clip(&mut self, track_idx: usize, clip_id: Uuid) {
        let state = self.track_states.entry(track_idx).or_insert(TrackState {
            active_clip_id: None,
            queued_clip_id: None,
        });
        state.queued_clip_id = Some(clip_id);
    }

    #[allow(dead_code)]
    pub fn stop_track(&mut self, track_idx: usize) {
        if let Some(state) = self.track_states.get_mut(&track_idx) {
            state.active_clip_id = None;
            state.queued_clip_id = None;
        }
    }

    /// Process quantized launching. Returns a list of (track_idx, clip_id) to start.
    pub fn process(&mut self, playhead: u64, frames: usize) -> Vec<(usize, Uuid)> {
        let mut launched = Vec::new();
        let next_playhead = playhead + frames as u64;

        // Check if we crossed a quantization boundary
        let current_bar = playhead / self.loop_quantization;
        let next_bar = next_playhead / self.loop_quantization;

        if next_bar > current_bar {
            // Boundary crossed! Launch queued clips.
            for (&track_idx, state) in self.track_states.iter_mut() {
                if let Some(clip_id) = state.queued_clip_id.take() {
                    state.active_clip_id = Some(clip_id);
                    launched.push((track_idx, clip_id));
                }
            }
        }
        launched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantized_launch() {
        let mut launcher = ClipLauncher::new();
        launcher.loop_quantization = 1000;

        let clip_id = Uuid::new_v4();
        launcher.trigger_clip(0, clip_id);

        // At playhead 500, with 10 frames, no launch
        let launches = launcher.process(500, 10);
        assert!(launches.is_empty());

        // At playhead 995, with 10 frames, crosses boundary 1000
        let launches = launcher.process(995, 10);
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0], (0, clip_id));

        // Subsequent calls don't re-launch
        let launches = launcher.process(1005, 10);
        assert!(launches.is_empty());
    }
}
