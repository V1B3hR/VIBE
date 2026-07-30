use crate::state::{emit_project_update, AppState};
use tauri::State;

#[tauri::command]
pub fn set_comp_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    enabled: bool,
) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().send_command(
        crate::engine::audio::AudioCommand::SetCompMode(track_idx, enabled),
    )?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_active_take(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    take_idx: usize,
) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().send_command(
        crate::engine::audio::AudioCommand::SetActiveTake(track_idx, take_idx),
    )?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn add_take_from_selection(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    start: u64,
    end: u64,
) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().send_command(
        crate::engine::audio::AudioCommand::AddTakeFromSelection(track_idx, start, end),
    )?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn add_playlist(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    name: String,
) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().send_command(
        crate::engine::audio::AudioCommand::AddPlaylist(track_idx, name),
    )?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_active_playlist(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    playlist_idx: usize,
) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().send_command(
        crate::engine::audio::AudioCommand::SetActivePlaylist(track_idx, playlist_idx),
    )?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn paste_time(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pos: u64,
) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().paste_time(pos)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn insert_silence(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pos: u64,
    len: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .insert_silence(pos, len)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn delete_time(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pos: u64,
    len: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .delete_time(pos, len)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn duplicate_time(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pos: u64,
    len: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .duplicate_time(pos, len)?;
    emit_project_update(&app, &state);
    Ok(())
}
