use crate::engine::graph::{EffectInfo, TrackInfo};
use crate::state::{emit_project_update, AppState};
use tauri::State;
use std::fs;
use std::path::Path;
use uuid::Uuid;
use tauri::Manager;

#[tauri::command]
pub fn add_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    log::info!("CMD: add_track({})", name);
    state.inner().audio_engine.lock().unwrap().add_track(name)?;
    log::info!("CMD: add_track success");
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn create_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    track_type: String,
) -> Result<(), String> {
    log::info!("CMD: create_track({}, {})", name, track_type);
    let mut track = crate::engine::graph::Track::new(name);
    track.track_type = match track_type.as_str() {
        "Audio" => crate::engine::graph::TrackType::Audio,
        "MIDI" => crate::engine::graph::TrackType::MIDI,
        "Instrument" => crate::engine::graph::TrackType::Instrument,
        "Aux" => crate::engine::graph::TrackType::Aux,
        "Group" => crate::engine::graph::TrackType::Group,
        "Folder" => crate::engine::graph::TrackType::Folder,
        _ => crate::engine::graph::TrackType::Audio,
    };
    state.inner().audio_engine.lock().unwrap().send_command(crate::engine::audio::AudioCommand::AddTrack(track))?;
    emit_project_update(&app, &state);
    Ok(())
}
#[tauri::command]
pub fn create_track_with_parent(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    track_type: String,
    parent_id: String,
) -> Result<(), String> {
    log::info!("CMD: create_track_with_parent({}, {}, {})", name, track_type, parent_id);
    let mut track = crate::engine::graph::Track::new(name);
    track.track_type = match track_type.as_str() {
        "Audio" => crate::engine::graph::TrackType::Audio,
        "MIDI" => crate::engine::graph::TrackType::MIDI,
        "Instrument" => crate::engine::graph::TrackType::Instrument,
        "Aux" => crate::engine::graph::TrackType::Aux,
        "Group" => crate::engine::graph::TrackType::Group,
        "Folder" => crate::engine::graph::TrackType::Folder,
        _ => crate::engine::graph::TrackType::Audio,
    };
    if let Ok(p_id) = uuid::Uuid::parse_str(&parent_id) {
        track.parent_id = Some(p_id);
    }
    state.inner().audio_engine.lock().unwrap().send_command(crate::engine::audio::AudioCommand::AddTrack(track))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn remove_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    idx: usize,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .remove_track(idx)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn move_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    from: usize,
    to: usize,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .send_command(crate::engine::audio::AudioCommand::MoveTrack(from, to))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn duplicate_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    idx: usize,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .duplicate_track(idx)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn rename_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    idx: usize,
    name: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .rename_track(idx, name)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_color(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    idx: usize,
    color: String,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackColor(
        idx, color,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_volume(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    volume: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_volume(index, volume as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_mute(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    muted: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_mute(index, muted)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_solo(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    solo: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_solo(index, solo)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_pan(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    pan: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_pan(index, pan as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_width(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    width: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_width(index, width as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_phase_invert(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    inverted: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_phase_invert(index, inverted)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_drive(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    val: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_track_drive(index, val as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_arm(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    armed: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_arm(index, armed)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_collapsed(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    collapsed: bool,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackCollapsed(
        index, collapsed,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_parent(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    parent_id: Option<String>,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackParent(
        index, parent_id,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_type(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    t_type: String,
) -> Result<(), String> {
    let t = match t_type.as_str() {
        "Audio" => crate::engine::graph::TrackType::Audio,
        "MIDI" => crate::engine::graph::TrackType::MIDI,
        "Instrument" => crate::engine::graph::TrackType::Instrument,
        "Aux" => crate::engine::graph::TrackType::Aux,
        "Group" => crate::engine::graph::TrackType::Group,
        "Folder" => crate::engine::graph::TrackType::Folder,
        _ => return Err("Invalid track type".to_string()),
    };
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackType(
        index, t,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_disabled(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    disabled: bool,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackDisabled(
        index, disabled,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_frozen(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    frozen: bool,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackFrozen(
        index, frozen,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_automation_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    mode: String,
) -> Result<(), String> {
    use crate::engine::graph::AutomationMode;
    let mode_enum = match mode.to_lowercase().as_str() {
        "read" => AutomationMode::Read,
        "write" => AutomationMode::Write,
        "touch" => AutomationMode::Touch,
        "latch" => AutomationMode::Latch,
        "off" => AutomationMode::Off,
        _ => return Err(format!("Invalid automation mode: {}", mode)),
    };
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTrackAutomationMode(
        index, mode_enum,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_input(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    alias_id: Option<String>,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_track_input(index, alias_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_output(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    output_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_track_output(index, Some(output_id))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_track_sidechain(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    source_id: Option<String>,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_track_sidechain(index, source_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_tracks(state: State<'_, AppState>) -> Vec<TrackInfo> {
    state.inner().audio_engine.lock().unwrap().get_tracks()
}

#[tauri::command]
pub fn add_effect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    index: usize,
    effect_type: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_effect(index, effect_type)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_effect_bypass(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
    bypass: bool,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetEffectBypass(
        track_idx,
        processor_id,
        bypass,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn move_effect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
    new_index: usize,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::MoveEffect(
        track_idx,
        processor_id,
        new_index,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn remove_effect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::RemoveEffect(
        track_idx,
        processor_id,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_parameter(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    param_id: String,
    value: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_parameter(param_id, value as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_eq_bands(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
    bands: Vec<crate::engine::eq_module::EqBand>,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.set_eq_bands(track_idx, processor_id, bands)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_eq_bands(
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
) -> Result<Vec<crate::engine::eq_module::EqBand>, String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_eq_bands(track_idx, processor_id)
}

#[tauri::command]
pub fn update_eq_band(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
    band: crate::engine::eq_module::EqBand,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::UpdateEqBand(
        track_idx,
        processor_id,
        band,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_compressor_metrics(
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
) -> (f32, f32) {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_compressor_metrics(track_idx, processor_id)
}

#[tauri::command]
pub fn load_synth_preset(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    proc_idx: usize,
    preset_path: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .load_synth_preset(track_idx, proc_idx, preset_path)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn save_synth_preset(
    state: State<'_, AppState>,
    track_idx: usize,
    proc_idx: usize,
    preset_path: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .save_synth_preset(track_idx, proc_idx, preset_path)
}

#[tauri::command]
pub fn update_mod_matrix(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    proc_idx: usize,
    slots: Vec<crate::engine::synth::ModSlot>,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .update_mod_matrix(track_idx, proc_idx, slots)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_master_info(state: State<'_, AppState>) -> Vec<EffectInfo> {
    state.inner().audio_engine.lock().unwrap().get_master_info()
}

#[tauri::command]
pub fn add_bus(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    color: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_bus(name, color)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn create_track_group(state: State<'_, AppState>, name: String) -> Result<String, String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .create_track_group(name)
}

#[tauri::command]
pub fn graph_add_node(
    state: State<'_, AppState>,
    node: crate::engine::audio_graph::GraphNode,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .graph_add_node(node)
}

#[tauri::command]
pub fn graph_remove_node(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .graph_remove_node(id)
}

#[tauri::command]
pub fn graph_connect(
    state: State<'_, AppState>,
    from_node: String,
    to_node: String,
    from_port: u32,
    to_port: u32,
    gain_db: f64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .graph_connect(from_node, to_node, from_port, to_port, gain_db)
}

#[tauri::command]
pub fn graph_disconnect(
    state: State<'_, AppState>,
    from_node: String,
    to_node: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .graph_disconnect(from_node, to_node)
}

#[tauri::command]
pub fn add_plugin_to_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_index: usize,
    plugin_path: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_plugin_to_track(track_index, plugin_path)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_eq_presets() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "Clear Vocal",
            "bands": [
                { "id": "1", "enabled": true, "filter_type": "HighPass", "freq": 100.0, "gain_db": 0.0, "q": 0.7, "mode": "Stereo", "solo": false },
                { "id": "2", "enabled": true, "filter_type": "Bell", "freq": 3000.0, "gain_db": 3.0, "q": 1.0, "mode": "Stereo", "solo": false }
            ]
        }),
        serde_json::json!({
            "name": "Bass Boost",
            "bands": [
                { "id": "1", "enabled": true, "filter_type": "LowShelf", "freq": 80.0, "gain_db": 6.0, "q": 0.7, "mode": "Stereo", "solo": false }
            ]
        }),
    ]
}

#[tauri::command]
pub async fn open_plugin_editor(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
) -> Result<(), String> {
    let (plugin_name, _plugin_id_uuid) = {
        let engine = state.audio_engine.lock().unwrap();
        let tracks = engine.tracks.lock().unwrap();
        
        if track_idx >= tracks.len() {
            return Err("Track index out of bounds".to_string());
        }
        
        let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
        let proc = tracks[track_idx].processors.iter().find(|p| p.id() == uuid)
            .ok_or_else(|| format!("Plugin {} not found on track {}", plugin_id, track_idx))?;
        
        (proc.name(), uuid)
    };

    let window_id = format!("plugin-{}-{}", track_idx, plugin_id);
    
    // Check if window already exists
    if let Some(existing) = app.get_webview_window(&window_id) {
        let _ = existing.set_focus();
        return Ok(());
    }

    // Create the window
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        &window_id,
        tauri::WebviewUrl::App("blank.html".into()) // Empty page to avoid drawing VIBE UI over plugin
    )
    .title(format!("Editor: {}", plugin_name))
    .inner_size(800.0, 600.0) // Reasonable default
    .transparent(true)
    .build()
    .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        let hwnd = window.hwnd().map_err(|_| "Failed to get HWND")?;
        
        let engine = state.audio_engine.lock().unwrap();
        let mut tracks = engine.tracks.lock().unwrap();
        let uuid = Uuid::parse_str(&plugin_id).unwrap();
        if let Some(proc) = tracks[track_idx].processors.iter_mut().find(|p| p.id() == uuid) {
            if let Some((w, h)) = proc.open_editor(hwnd.0 as *mut std::ffi::c_void) {
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: w as f64,
                    height: h as f64,
                }));
            }
        }
    }

    // Listen for window close to inform the plugin
    let app_clone = app.clone();
    let plugin_id_clone = plugin_id.clone();
    let track_idx_clone = track_idx;

    // Spawn a background task to poll for editor resize
    let app_for_spawn = app_clone.clone();
    let plugin_id_for_spawn = plugin_id_clone.clone();
    let window_clone = window.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            // Check if window still exists
            if app_for_spawn.get_webview_window(&window_clone.label()).is_none() {
                break;
            }

            let resize_req = {
                if let Ok(engine) = app_for_spawn.state::<AppState>().audio_engine.lock() {
                    if let Ok(mut tracks) = engine.tracks.lock() {
                        if let Ok(uuid) = Uuid::parse_str(&plugin_id_for_spawn) {
                            if let Some(track) = tracks.get_mut(track_idx_clone) {
                                if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == uuid) {
                                    proc.poll_editor_resize()
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None }
                } else { None }
            };

            if let Some((w, h)) = resize_req {
                let _ = window_clone.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: w as f64,
                    height: h as f64,
                }));
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            if let Ok(engine) = app_clone.state::<AppState>().audio_engine.lock() {
                if let Ok(mut tracks) = engine.tracks.lock() {
                    if let Ok(uuid) = Uuid::parse_str(&plugin_id_clone) {
                        if let Some(track) = tracks.get_mut(track_idx_clone) {
                            if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == uuid) {
                                proc.close_editor();
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn create_track_with_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    clip_path: String,
) -> Result<(), String> {
    // 1. Add Track
    add_track(app.clone(), state.clone(), name)?;

    // 2. Get the index of the newly added track (last one)
    let track_idx = {
        let engine = state.inner().audio_engine.lock().unwrap();
        let tracks = engine.get_tracks();
        if tracks.is_empty() {
            return Err("Failed to add track".to_string());
        }
        tracks.len() - 1
    };

    // 3. Import and Add Clip
    let engine = state.inner().audio_engine.lock().unwrap();
    let info = engine.import_file_internal(std::path::PathBuf::from(clip_path))?;

    // 4. Add Clip to the track at 0
    engine.add_clip_to_track(track_idx, info.id.to_string(), 0)?;

    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn list_synth_presets() -> Result<Vec<String>, String> {
    let mut presets = Vec::new();
    let path = Path::new("presets/synth");
    
    // Ensure directory exists
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
    }
    
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") || name.ends_with(".vone") {
                            presets.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    
    presets.sort();
    Ok(presets)
}

// ─── Plugin Preset / State Commands ───────────────────────────────────────────

/// Export a plugin's binary state (VST3 chunk) as base64.
#[tauri::command]
pub fn get_plugin_state(
    state: State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
) -> Result<String, String> {
    let engine = state.audio_engine.lock().unwrap();
    let tracks = engine.tracks.lock().unwrap();
    let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
    let track = tracks.get(track_idx).ok_or("Track not found")?;
    let proc = track.processors.iter().find(|p| p.id() == uuid)
        .ok_or("Plugin not found")?;
    let raw = proc.get_state();
    Ok(base64_encode(&raw))
}

/// Restore a plugin's binary state from a base64 string.
#[tauri::command]
pub fn set_plugin_state(
    state: State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
    state_b64: String,
) -> Result<(), String> {
    let raw = base64_decode(&state_b64)?;
    let engine = state.audio_engine.lock().unwrap();
    let mut tracks = engine.tracks.lock().unwrap();
    let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
    let track = tracks.get_mut(track_idx).ok_or("Track not found")?;
    let proc = track.processors.iter_mut().find(|p| p.id() == uuid)
        .ok_or("Plugin not found")?;
    proc.set_state(&raw);
    Ok(())
}

/// Returns the latency (in samples) reported by a plugin — used for PDC.
#[tauri::command]
pub fn get_plugin_latency(
    state: State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
) -> Result<usize, String> {
    let engine = state.audio_engine.lock().unwrap();
    let tracks = engine.tracks.lock().unwrap();
    let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
    let track = tracks.get(track_idx).ok_or("Track not found")?;
    let proc = track.processors.iter().find(|p| p.id() == uuid)
        .ok_or("Plugin not found")?;
    Ok(proc.latency_samples())
}

/// Save plugin state to a named preset file.
#[tauri::command]
pub fn save_plugin_preset(
    state: State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
    preset_name: String,
) -> Result<String, String> {
    let engine = state.audio_engine.lock().unwrap();
    let tracks = engine.tracks.lock().unwrap();
    let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
    let track = tracks.get(track_idx).ok_or("Track not found")?;
    let proc = track.processors.iter().find(|p| p.id() == uuid)
        .ok_or("Plugin not found")?;
    let name_slug = slugify(&proc.name());
    let dir = format!("presets/plugins/{}", name_slug);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe_preset = slugify(&preset_name);
    let path = format!("{}/{}.vst3preset", dir, safe_preset);
    let raw = proc.get_state();
    fs::write(&path, &raw).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Load a named preset file back into the plugin.
#[tauri::command]
pub fn load_plugin_preset(
    state: State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
    preset_path: String,
) -> Result<(), String> {
    let raw = fs::read(&preset_path).map_err(|e| e.to_string())?;
    let engine = state.audio_engine.lock().unwrap();
    let mut tracks = engine.tracks.lock().unwrap();
    let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
    let track = tracks.get_mut(track_idx).ok_or("Track not found")?;
    let proc = track.processors.iter_mut().find(|p| p.id() == uuid)
        .ok_or("Plugin not found")?;
    proc.set_state(&raw);
    Ok(())
}

/// List available presets for a plugin by name.
#[tauri::command]
pub fn list_plugin_presets(plugin_name: String) -> Result<Vec<String>, String> {
    let dir = format!("presets/plugins/{}", slugify(&plugin_name));
    if !Path::new(&dir).exists() {
        return Ok(vec![]);
    }
    let mut presets = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".vst3preset") {
                    presets.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    presets.sort();
    Ok(presets)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 0x3F) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 0x3F) as usize] as char } else { '=' });
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s: Vec<u8> = s.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::with_capacity(s.len() * 3 / 4 + 1);
    fn val(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("Invalid base64 char: {c}")),
        }
    }
    for chunk in s.chunks(4) {
        let v0 = val(chunk[0])?;
        let v1 = val(chunk[1])?;
        out.push(((v0 << 2) | (v1 >> 4)) as u8);
        if chunk.len() > 2 {
            let v2 = val(chunk[2])?;
            out.push(((v1 << 4) | (v2 >> 2)) as u8);
        }
        if chunk.len() > 3 {
            let v2 = val(chunk[2])?;
            let v3 = val(chunk[3])?;
            out.push(((v2 << 6) | v3) as u8);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn poll_plugin_param_changes(
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
) -> Result<Vec<(String, f64)>, String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let mut tracks = engine.tracks.lock().unwrap();
    
    if let Some(track) = tracks.get_mut(track_idx) {
        if let Ok(uuid) = Uuid::parse_str(&processor_id) {
            if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == uuid) {
                return Ok(proc.drain_plugin_feedback());
            }
        }
    }
    
    Ok(Vec::new())
}

#[tauri::command]
pub fn get_plugin_programs(
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
) -> Result<Vec<String>, String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let mut tracks = engine.tracks.lock().unwrap();
    
    if let Some(track) = tracks.get_mut(track_idx) {
        if let Ok(uuid) = Uuid::parse_str(&processor_id) {
            if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == uuid) {
                return Ok(proc.get_programs());
            }
        }
    }
    Ok(Vec::new())
}

#[tauri::command]
pub fn set_plugin_program(
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
    program_idx: i32,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let mut tracks = engine.tracks.lock().unwrap();
    
    if let Some(track) = tracks.get_mut(track_idx) {
        if let Ok(uuid) = Uuid::parse_str(&processor_id) {
            if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == uuid) {
                proc.set_program(program_idx);
                return Ok(());
            }
        }
    }
    Err("Plugin not found".into())
}

#[tauri::command]
pub fn get_plugin_cpu_usage(
    state: State<'_, AppState>,
    track_idx: usize,
    processor_id: String,
) -> Result<f32, String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let tracks = engine.tracks.lock().unwrap();
    
    if let Some(track) = tracks.get(track_idx) {
        if let Ok(uuid) = Uuid::parse_str(&processor_id) {
            if let Some(proc) = track.processors.iter().find(|p| p.id() == uuid) {
                return Ok(proc.get_cpu_usage());
            }
        }
    }
    Ok(0.0)
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '_' })
        .collect()
}

#[tauri::command]
pub fn poll_plugin_resize(
    state: tauri::State<'_, AppState>,
    track_idx: usize,
    plugin_id: String,
) -> Result<Option<(u32, u32)>, String> {
    let engine = state.audio_engine.lock().unwrap();
    let mut tracks = engine.tracks.lock().unwrap();
    let uuid = Uuid::parse_str(&plugin_id).map_err(|e| e.to_string())?;
    
    if let Some(track) = tracks.get_mut(track_idx) {
        if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == uuid) {
            return Ok(proc.poll_editor_resize());
        }
    }
    Err("Plugin not found".to_string())
}
