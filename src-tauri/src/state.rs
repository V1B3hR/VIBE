use crate::engine::library_service::LibraryService;
use crate::engine::plugin_manager::PluginManager;
use crate::engine::AudioEngine;
use std::sync::Mutex;
use tauri::Emitter;

pub struct AppState {
    pub audio_engine: Mutex<AudioEngine>,
    pub library_service: Mutex<LibraryService>,
    pub plugin_manager: Mutex<PluginManager>,
}

/// Helper function to emit project state updates to frontend
/// This eliminates the need for frontend polling
pub fn emit_project_update(app: &tauri::AppHandle, state: &tauri::State<AppState>) {
    let engine = state.inner().audio_engine.lock().unwrap();
    let tracks = engine.get_tracks();
    let bpm = engine.get_bpm();
    let swing = engine.get_global_swing();
    let markers = engine.get_markers();

    let payload = serde_json::json!({
        "tracks": tracks,
        "bpm": bpm,
        "swing": swing,
        "markers": markers,
    });

    // Tauri 2 API
    let _ = app.emit("project_updated", payload);
}
