use crate::engine::spatial_panner::{BinauralHrtfGains, SpatialPanner3D, SurroundGains714};

#[tauri::command]
pub fn calculate_714_spatial_gains_cmd(x: f32, y: f32, z: f32) -> Result<SurroundGains714, String> {
    Ok(SpatialPanner3D::calculate_714_gains(x, y, z))
}

#[tauri::command]
pub fn calculate_binaural_hrtf_cmd(x: f32, y: f32, z: f32) -> Result<BinauralHrtfGains, String> {
    Ok(SpatialPanner3D::calculate_binaural_hrtf(x, y, z))
}
