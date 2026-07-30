use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use super::graph::AudioClip;

/// 10ms crossfade at 48kHz = 480 samples
pub const DEFAULT_CROSSFADE_SAMPLES: u64 = 480;

/// Represents an individual take lane under an audio track
#[derive(Clone, Serialize, Deserialize)]
pub struct TakeLane {
    pub id: Uuid,
    pub name: String,
    pub clips: Vec<AudioClip>,
    pub is_muted: bool,
    pub is_solo: bool,
    pub color: Option<String>,
}

impl TakeLane {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            clips: Vec::new(),
            is_muted: false,
            is_solo: false,
            color: None,
        }
    }
}

/// Represents an active selection region on a specific take lane
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompRegion {
    pub id: Uuid,
    pub take_lane_id: Uuid,
    pub start_sample: u64,
    pub end_sample: u64,
    pub crossfade_samples: u64,
}

impl CompRegion {
    pub fn new(take_lane_id: Uuid, start_sample: u64, end_sample: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            take_lane_id,
            start_sample,
            end_sample,
            crossfade_samples: DEFAULT_CROSSFADE_SAMPLES,
        }
    }
}

/// Comping engine for managing take lanes and generating composite master audio clips
pub struct CompingEngine;

impl CompingEngine {
    /// Selects a region on a take lane and automatically resolves/splits overlapping regions on other lanes
    pub fn select_region(
        existing_regions: &mut Vec<CompRegion>,
        new_lane_id: Uuid,
        start_sample: u64,
        end_sample: u64,
    ) {
        if start_sample >= end_sample {
            return;
        }

        let mut updated: Vec<CompRegion> = Vec::new();

        for region in existing_regions.drain(..) {
            // No overlap
            if region.end_sample <= start_sample || region.start_sample >= end_sample {
                updated.push(region);
                continue;
            }

            // Region completely covered by new selection -> remove
            if region.start_sample >= start_sample && region.end_sample <= end_sample {
                continue;
            }

            // Region starts before new selection and ends after -> split into two
            if region.start_sample < start_sample && region.end_sample > end_sample {
                let left = CompRegion {
                    id: Uuid::new_v4(),
                    take_lane_id: region.take_lane_id,
                    start_sample: region.start_sample,
                    end_sample: start_sample,
                    crossfade_samples: region.crossfade_samples,
                };
                let right = CompRegion {
                    id: Uuid::new_v4(),
                    take_lane_id: region.take_lane_id,
                    start_sample: end_sample,
                    end_sample: region.end_sample,
                    crossfade_samples: region.crossfade_samples,
                };
                updated.push(left);
                updated.push(right);
                continue;
            }

            // Region overlaps on the left
            if region.start_sample < start_sample && region.end_sample > start_sample {
                let left = CompRegion {
                    id: Uuid::new_v4(),
                    take_lane_id: region.take_lane_id,
                    start_sample: region.start_sample,
                    end_sample: start_sample,
                    crossfade_samples: region.crossfade_samples,
                };
                updated.push(left);
                continue;
            }

            // Region overlaps on the right
            if region.start_sample < end_sample && region.end_sample > end_sample {
                let right = CompRegion {
                    id: Uuid::new_v4(),
                    take_lane_id: region.take_lane_id,
                    start_sample: end_sample,
                    end_sample: region.end_sample,
                    crossfade_samples: region.crossfade_samples,
                };
                updated.push(right);
                continue;
            }
        }

        // Add the new active comp region
        updated.push(CompRegion::new(new_lane_id, start_sample, end_sample));
        updated.sort_by_key(|r| r.start_sample);

        *existing_regions = updated;
    }

    /// Flattens comp regions and take lanes into a single finalized audio clip list with equal-power crossfades
    pub fn flatten_comp(take_lanes: &[TakeLane], comp_regions: &[CompRegion]) -> Vec<AudioClip> {
        let mut result_clips: Vec<AudioClip> = Vec::new();

        for region in comp_regions {
            if let Some(lane) = take_lanes.iter().find(|l| l.id == region.take_lane_id) {
                for clip in &lane.clips {
                    let clip_end = clip.start_sample + clip.length_in_samples;

                    // Check if clip intersects region
                    if clip.start_sample < region.end_sample && clip_end > region.start_sample {
                        let sub_start = clip.start_sample.max(region.start_sample);
                        let sub_end = clip_end.min(region.end_sample);
                        let duration = sub_end.saturating_sub(sub_start);

                        if duration > 0 {
                            let mut comp_clip = clip.clone();
                            comp_clip.id = Uuid::new_v4();
                            comp_clip.start_sample = sub_start;
                            comp_clip.length_in_samples = duration;
                            comp_clip.name = format!("Comp ({})", lane.name);

                            // Apply equal-power 10ms crossfade boundaries
                            comp_clip.fade_in_len = region.crossfade_samples;
                            comp_clip.fade_out_len = region.crossfade_samples;

                            result_clips.push(comp_clip);
                        }
                    }
                }
            }
        }

        result_clips
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::WarpMode;

    #[test]
    fn test_select_region_overlap_resolution() {
        let mut regions: Vec<CompRegion> = Vec::new();
        let lane1 = Uuid::new_v4();
        let lane2 = Uuid::new_v4();

        // Add initial selection on Lane 1 (0 to 1000)
        CompingEngine::select_region(&mut regions, lane1, 0, 1000);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].take_lane_id, lane1);

        // Swipe selection on Lane 2 (400 to 600) -> splits Lane 1 into two (0..400 and 600..1000)
        CompingEngine::select_region(&mut regions, lane2, 400, 600);
        assert_eq!(regions.len(), 3);
        assert_eq!(regions[0].start_sample, 0);
        assert_eq!(regions[0].end_sample, 400);
        assert_eq!(regions[0].take_lane_id, lane1);

        assert_eq!(regions[1].start_sample, 400);
        assert_eq!(regions[1].end_sample, 600);
        assert_eq!(regions[1].take_lane_id, lane2);

        assert_eq!(regions[2].start_sample, 600);
        assert_eq!(regions[2].end_sample, 1000);
        assert_eq!(regions[2].take_lane_id, lane1);
    }

    #[test]
    fn test_flatten_comp_generation() {
        let lane_id = Uuid::new_v4();
        let mut lane = TakeLane::new("Take 1");

        let clip = AudioClip {
            id: Uuid::new_v4(),
            name: "Take 1 Rec".to_string(),
            head_data: Arc::new(Vec::new()),
            peaks: Vec::new(),
            start_sample: 0,
            offset_in_data: 0,
            length_in_samples: 48000,
            sample_rate: 48000,
            color: "#00ff00".to_string(),
            fade_in_len: 0,
            fade_out_len: 0,
            fade_in_type: crate::engine::fades::FadeType::Linear,
            fade_out_type: crate::engine::fades::FadeType::Linear,
            gain: 1.0,
            pitch_semitones: 0.0,
            playback_speed: 1.0,
            is_warped: false,
            is_reversed: false,
            warp_mode: WarpMode::Complex,
            path: Some("/test.wav".to_string()),
            waveform_cache: None,
            is_streaming: false,
            #[cfg(target_os = "windows")]
            file: None,
            gain_envelope: None,
            pitch_envelope: None,
            pan_envelope: None,
            transients: Vec::new(),
            warp_markers: Vec::new(),
            base_bpm: 120.0,
            reference_clip_id: None,
        };
        lane.clips.push(clip);

        let region = CompRegion::new(lane_id, 10000, 20000);
        lane.id = lane_id;

        let flattened = CompingEngine::flatten_comp(&[lane], &[region]);
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].start_sample, 10000);
        assert_eq!(flattened[0].length_in_samples, 10000);
        assert_eq!(flattened[0].fade_in_len, DEFAULT_CROSSFADE_SAMPLES);
    }
}
