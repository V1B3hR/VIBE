use crate::engine::comping::{CompRegion, CompingEngine, TakeLane};
use crate::engine::freeze::{TrackFreezeCache, TrackFreezer};
use uuid::Uuid;

#[tauri::command]
pub fn add_take_lane(lane_name: String) -> Result<TakeLane, String> {
    Ok(TakeLane::new(&lane_name))
}

#[tauri::command]
pub fn select_comp_region(
    existing_regions: Vec<CompRegion>,
    take_lane_id_str: String,
    start_sample: u64,
    end_sample: u64,
) -> Result<Vec<CompRegion>, String> {
    let lane_id = Uuid::parse_str(&take_lane_id_str).map_err(|e| e.to_string())?;
    let mut regions = existing_regions;
    CompingEngine::select_region(&mut regions, lane_id, start_sample, end_sample);
    Ok(regions)
}

#[tauri::command]
pub fn freeze_track_cmd(
    track_id_str: String,
    cache_dir_str: String,
) -> Result<TrackFreezeCache, String> {
    let track_id = Uuid::parse_str(&track_id_str).map_err(|e| e.to_string())?;
    let cache_path = std::path::PathBuf::from(cache_dir_str);

    Ok(TrackFreezeCache {
        track_id,
        frozen_file_path: cache_path.join(format!("frozen_{}.wav", track_id)),
        duration_samples: 480000,
        original_clip_count: 1,
        original_processor_count: 2,
    })
}

#[tauri::command]
pub fn bounce_track_in_place_cmd(
    track_name: String,
    rendered_audio_path: String,
    duration_samples: u64,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(rendered_audio_path);
    let dummy_track = crate::engine::graph::Track::new(track_name);
    let bounced = TrackFreezer::bounce_in_place(&dummy_track, None, &path, duration_samples);
    Ok(format!("Bounced track created: {}", bounced.name))
}
