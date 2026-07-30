use crate::engine::render_engine::RenderConfig;
use crate::state::{emit_project_update, AppState};
use std::collections::HashMap;
use tauri::Emitter;

#[tauri::command]
pub fn new_project(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().new_project()?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn save_project_file(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let project_path = std::path::Path::new(&path);
    engine.save_project(project_path)
}

#[tauri::command]
pub fn save_project(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // For VIBE 1.0, we use a default project path if none is set
    // A more robust implementation would track the current project's file path.
    let default_path = "C:\\Users\\brigh\\Desktop\\VIBE_Project.vibe";
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.save_project(std::path::Path::new(default_path))
}

#[tauri::command]
pub fn load_project_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    {
        let mut engine = state.inner().audio_engine.lock().unwrap();
        let project_path = std::path::Path::new(&path);
        engine.load_project(project_path, &state.inner().plugin_manager)?;
    }
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn check_autosave_exists(_state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let autosave_path = std::path::Path::new("autosave.vibe-autosave");
    Ok(autosave_path.exists())
}

#[tauri::command]
pub fn recover_from_autosave(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let autosave_path = std::path::Path::new("autosave.vibe-autosave");

    if !autosave_path.exists() {
        return Err("No autosave file found".to_string());
    }

    {
        let mut engine = state.inner().audio_engine.lock().unwrap();
        engine.load_project(autosave_path, &state.inner().plugin_manager)?;
    }
    println!("VIBE: Recovered project from autosave");
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn delete_autosave(_state: tauri::State<'_, AppState>) -> Result<(), String> {
    let autosave_path = std::path::Path::new("autosave.vibe-autosave");

    if autosave_path.exists() {
        std::fs::remove_file(autosave_path)
            .map_err(|e| format!("Failed to delete autosave: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn export_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: RenderConfig,
) -> Result<(), String> {
    let rx = {
        let engine = state.inner().audio_engine.lock().unwrap();
        engine.export_project(config)?
    };

    let app_clone = app.clone();
    std::thread::spawn(move || {
        while let Ok(status) = rx.recv() {
            use crate::engine::render_engine::RenderStatus;
            let payload = match &status {
                RenderStatus::Progress(p) => serde_json::json!({ "type": "progress", "value": p }),
                RenderStatus::AnalysisResult { lufs, true_peak } => serde_json::json!({
                    "type": "analysis",
                    "lufs": lufs,
                    "true_peak": true_peak
                }),
                RenderStatus::Complete(path) => {
                    serde_json::json!({ "type": "complete", "path": path })
                }
                RenderStatus::Error(e) => serde_json::json!({ "type": "error", "message": e }),
            };
            let _ = app_clone.emit("export_status", payload);

            if let RenderStatus::Complete(_) | RenderStatus::Error(_) = status {
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn undo(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().undo()?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn redo(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().redo()?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_history_graph(
    state: tauri::State<'_, AppState>,
) -> Vec<(String, Option<String>, String)> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_history_graph()
}

#[tauri::command]
pub fn get_current_node(state: tauri::State<'_, AppState>) -> String {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_current_node()
}

#[tauri::command]
pub fn get_branches(state: tauri::State<'_, AppState>) -> HashMap<String, String> {
    state.inner().audio_engine.lock().unwrap().get_branches()
}

#[tauri::command]
pub fn checkout_node(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    node_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .checkout(node_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn create_branch(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .create_branch(name)
}
