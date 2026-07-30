use crate::engine::graph::AudioClipInfo;
use crate::engine::library_service::AudioFileMetadata;
use crate::engine::plugin_manager::{PluginCategory, PluginInfo, PluginType};
use crate::state::{emit_project_update, AppState};
use tauri::{Emitter, State};

#[tauri::command]
pub fn get_library(state: State<'_, AppState>) -> Vec<AudioClipInfo> {
    state.inner().audio_engine.lock().unwrap().get_library()
}

#[tauri::command]
pub async fn import_to_library(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    log::info!("CMD: import_to_library(path={})", path);
    let path_buf = std::path::PathBuf::from(path);
    let clip = tauri::async_runtime::spawn_blocking(move || {
        crate::engine::audio::load_audio_file(path_buf, 48000.0)
    })
    .await
    .map_err(|e| format!("Join error: {}", e))??;

    {
        let engine = state.inner().audio_engine.lock().unwrap();
        let _ = engine.add_clip_to_library(clip)?;
    }

    emit_project_update(&app, &state);
    let _ = app.emit("library-update", ());
    Ok(())
}

#[tauri::command]
pub async fn import_audio_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<AudioClipInfo, String> {
    let path_buf = std::path::PathBuf::from(path);
    let clip = tauri::async_runtime::spawn_blocking(move || {
        crate::engine::audio::load_audio_file(path_buf, 48000.0)
    })
    .await
    .map_err(|e| format!("Join error: {}", e))??;

    let info = {
        let engine = state.inner().audio_engine.lock().unwrap();
        engine.add_clip_to_library(clip)?
    };

    emit_project_update(&app, &state);
    Ok(info)
}

#[tauri::command]
pub fn library_search(
    state: State<'_, AppState>,
    query: String,
    max_results: usize,
) -> Result<Vec<AudioFileMetadata>, String> {
    let library = state.inner().library_service.lock().unwrap();
    Ok(library.fuzzy_search(&query, max_results))
}

#[tauri::command]
pub fn library_add_directory(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let mut library = state.inner().library_service.lock().unwrap();
    library.add_watch_directory(std::path::PathBuf::from(path))
}

#[tauri::command]
pub fn plugin_scan_directory(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let mut plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.add_watch_directory(std::path::PathBuf::from(path))
}

#[tauri::command]
pub fn plugin_get_all(state: State<'_, AppState>) -> Result<Vec<PluginInfo>, String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    Ok(plugin_manager.get_all_plugins())
}

#[tauri::command]
pub fn plugin_search(state: State<'_, AppState>, query: String) -> Result<Vec<PluginInfo>, String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    Ok(plugin_manager.search(&query))
}

#[tauri::command]
pub fn plugin_get_by_type(
    state: State<'_, AppState>,
    plugin_type: String,
) -> Result<Vec<PluginInfo>, String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    let ptype = match plugin_type.to_lowercase().as_str() {
        "vst2" => PluginType::VST2,
        "vst3" => PluginType::VST3,
        "clap" => PluginType::CLAP,
        "native" => PluginType::Native,
        _ => return Err(format!("Unknown plugin type: {}", plugin_type)),
    };
    Ok(plugin_manager.get_by_type(ptype))
}

#[tauri::command]
pub fn plugin_get_by_category(
    state: State<'_, AppState>,
    category: String,
) -> Result<Vec<PluginInfo>, String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    let cat = match category.to_lowercase().as_str() {
        "dynamics" => PluginCategory::Dynamics,
        "eq" => PluginCategory::EQ,
        "reverb" => PluginCategory::Reverb,
        "delay" => PluginCategory::Delay,
        "distortion" => PluginCategory::Distortion,
        "modulation" => PluginCategory::Modulation,
        "instrument" => PluginCategory::Instrument,
        "utility" => PluginCategory::Utility,
        "midifx" => PluginCategory::MidiFX,
        "other" => PluginCategory::Other,
        _ => return Err(format!("Unknown category: {}", category)),
    };
    Ok(plugin_manager.get_by_category(cat))
}

#[tauri::command]
pub fn plugin_toggle_favorite(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.toggle_favorite(&plugin_id)
}

#[tauri::command]
pub fn plugin_get_favorites(state: State<'_, AppState>) -> Result<Vec<PluginInfo>, String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    Ok(plugin_manager.get_favorites())
}

#[tauri::command]
pub fn preview_play(
    state: State<'_, AppState>,
    path: String,
    quantize: Option<String>,
    stretch: bool,
    strength: f32,
    swing: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .preview_sample_synced(path, quantize, stretch, strength, swing)
}

#[tauri::command]
pub fn preview_stop(state: State<'_, AppState>) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().stop_preview()
}

#[tauri::command]
pub fn add_wasm_plugin(
    state: State<'_, AppState>,
    track_idx: usize,
    path: String,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::AddWasmPlugin(
        track_idx, path,
    ))
}

#[tauri::command]
pub fn scan_plugins(state: State<'_, AppState>) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.rescan_all()
}

#[tauri::command]
pub fn import_plugin(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .import_plugin(path)
}

#[tauri::command]
pub fn create_audio_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    crate::commands::track::add_track(app, state, name)
}

#[tauri::command]
pub fn preview_sample_synced(
    state: State<'_, AppState>,
    path: String,
    quantize: Option<String>,
    stretch: bool,
    strength: f32,
    swing: f32,
) -> Result<(), String> {
    preview_play(state, path, quantize, stretch, strength, swing)
}

#[tauri::command]
pub fn stop_preview(state: State<'_, AppState>) -> Result<(), String> {
    preview_stop(state)
}

#[tauri::command]
pub fn plugin_delete_chain(state: State<'_, AppState>, chain_id: String) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.delete_chain(&chain_id)
}

#[tauri::command]
pub fn plugin_add_search_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.add_search_path(std::path::PathBuf::from(path));
    Ok(())
}

#[tauri::command]
pub fn plugin_remove_search_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.remove_search_path(&std::path::PathBuf::from(path));
    Ok(())
}

#[tauri::command]
pub fn plugin_handle_blacklist(
    state: State<'_, AppState>,
    plugin_id: String,
    reason: String,
) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.blacklist_plugin(&plugin_id, &reason)
}

#[tauri::command]
pub fn plugin_set_hidden(
    state: State<'_, AppState>,
    plugin_id: String,
    hidden: bool,
) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.set_hidden(&plugin_id, hidden)
}

#[tauri::command]
pub fn plugin_set_deprecated(
    state: State<'_, AppState>,
    plugin_id: String,
    deprecated: bool,
) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.set_deprecated(&plugin_id, deprecated)
}

#[tauri::command]
pub fn plugin_merge_duplicates(
    state: State<'_, AppState>,
    primary_id: String,
    duplicate_id: String,
) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.merge_duplicates(&primary_id, &duplicate_id)
}

#[tauri::command]
pub fn plugin_set_custom_folder(
    state: State<'_, AppState>,
    plugin_id: String,
    folder: Option<String>,
) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.set_custom_folder(&plugin_id, folder)
}

#[tauri::command]
pub fn plugin_update_last_used(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let plugin_manager = state.inner().plugin_manager.lock().unwrap();
    plugin_manager.update_last_used(&plugin_id)
}
