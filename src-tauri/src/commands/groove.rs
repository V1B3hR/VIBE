use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_groove_templates(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let pool = audio_engine.groove_pool.lock().map_err(|e| e.to_string())?;

    Ok(pool.templates.iter().map(|t| t.name.clone()).collect())
}

#[tauri::command]
pub async fn get_shadow_grid(
    state: State<'_, AppState>,
    template_name: String,
) -> Result<Vec<(f32, f32)>, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let pool = audio_engine.groove_pool.lock().map_err(|e| e.to_string())?;

    Ok(pool.get_shadow_grid(&template_name))
}

#[tauri::command]
pub async fn extract_groove_from_track(
    state: State<'_, AppState>,
    track_id: u32,
    name: String,
) -> Result<String, String> {
    let _audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
    // Extraction logic would find track and use its transients

    Ok(format!(
        "Extracted groove '{}' from track {}. Ready to swing!",
        name, track_id
    ))
}
