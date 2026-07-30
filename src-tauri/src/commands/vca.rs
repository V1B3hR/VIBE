use crate::engine::vca_group::VcaGroup;
use uuid::Uuid;

#[tauri::command]
pub fn create_vca_group_cmd(name: String) -> Result<VcaGroup, String> {
    Ok(VcaGroup::new(name))
}

#[tauri::command]
pub fn add_track_to_vca_cmd(vca_id_str: String, track_id_str: String) -> Result<(), String> {
    let _vca_id = Uuid::parse_str(&vca_id_str).map_err(|e| e.to_string())?;
    let _track_id = Uuid::parse_str(&track_id_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn remove_track_from_vca_cmd(vca_id_str: String, track_id_str: String) -> Result<(), String> {
    let _vca_id = Uuid::parse_str(&vca_id_str).map_err(|e| e.to_string())?;
    let _track_id = Uuid::parse_str(&track_id_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_vca_gain_cmd(vca_id_str: String, gain_db: f64) -> Result<(), String> {
    let _vca_id = Uuid::parse_str(&vca_id_str).map_err(|e| e.to_string())?;
    let _gain = gain_db;
    Ok(())
}
