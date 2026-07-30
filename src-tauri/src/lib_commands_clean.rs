// PHASE 1.6: Hyper-Library Commands

#[tauri::command]
fn library_search(
    state: tauri::State<'_, AppState>,
    query: String,
    max_results: usize,
) -> Result<Vec<engine::library_service::AudioFileMetadata>, String> {
    let library = state.inner().library_service.lock().unwrap();
    Ok(library.fuzzy_search(&query, max_results))
}

#[tauri::command]
fn library_add_directory(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let mut library = state.inner().library_service.lock().unwrap();
    library.add_watch_directory(std::path::PathBuf::from(path))
}

#[tauri::command]
fn library_get_by_category(
    state: tauri::State<'_, AppState>,
    category: String,
) -> Result<Vec<engine::library_service::AudioFileMetadata>, String> {
    use engine::library_service::AudioCategory;

    let library = state.inner().library_service.lock().unwrap();
    let cat = match category.to_lowercase().as_str() {
        "kick" => AudioCategory::Kick,
        "snare" => AudioCategory::Snare,
        "hat" => AudioCategory::Hat,
        "percussion" => AudioCategory::Percussion,
        "bass" => AudioCategory::Bass,
        "synth" => AudioCategory::Synth,
        "vocal" => AudioCategory::Vocal,
        "loop" => AudioCategory::Loop,
        "fx" => AudioCategory::FX,
        _ => AudioCategory::Unknown,
    };

    Ok(library.get_by_category(cat))
}

#[tauri::command]
fn library_get_recent_projects() -> Result<Vec<String>, String> {
    // TODO: Get last 5 projects from history
    Ok(vec![])
}

// PHASE 1.6: Audio Preview Commands

#[tauri::command]
fn preview_load_file(_state: tauri::State<'_, AppState>, path: String) -> Result<Vec<f32>, String> {
    // TODO: Integrate with AudioPreviewPlayer
    // For now return empty waveform
    Ok(vec![0.0; 100])
}

#[tauri::command]
fn preview_play(_state: tauri::State<'_, AppState>) -> Result<(), String> {
    // TODO: Start preview playback
    Ok(())
}

#[tauri::command]
fn preview_stop(_state: tauri::State<'_, AppState>) -> Result<(), String> {
    // TODO: Stop preview playback
    Ok(())
}

#[tauri::command]
fn preview_seek(_state: tauri::State<'_, AppState>, position: f32) -> Result<(), String> {
    // TODO: Seek preview to position
    Ok(())
}
