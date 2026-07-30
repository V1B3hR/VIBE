use crate::engine::audio_device::{AudioDeviceConfig, AudioDeviceInfo, AudioDeviceManager};
use crate::engine::graph::TrackLevel;
use crate::state::{emit_project_update, AppState};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn play_audio(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("CMD: play_audio");
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.check_initialization()?;
    let res = engine.play();
    log::info!("CMD: play_audio result: {:?}", res);
    res
}

#[tauri::command]
pub fn pause_audio(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.check_initialization()?;
    engine.pause()
}

#[tauri::command]
pub fn stop_transport(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.check_initialization()?;
    engine.stop()
}

#[tauri::command]
pub fn toggle_record(state: State<'_, AppState>) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().toggle_record()
}

#[tauri::command]
pub fn set_bpm(app: tauri::AppHandle, state: State<'_, AppState>, bpm: f32) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().set_bpm(bpm)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_global_swing(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    swing: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_global_swing(swing)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_bpm(state: State<'_, AppState>) -> f32 {
    state.inner().audio_engine.lock().unwrap().get_bpm()
}

#[tauri::command]
pub fn set_metronome(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_metronome(enabled)
}

#[tauri::command]
pub fn set_loop_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_loop_enabled(enabled)
}

#[tauri::command]
pub fn set_loop_range(state: State<'_, AppState>, start: u64, end: u64) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_loop_range(start, end)
}

#[tauri::command]
pub fn is_loop_enabled(state: State<'_, AppState>) -> bool {
    state.inner().audio_engine.lock().unwrap().is_loop_enabled()
}

#[tauri::command]
pub fn get_loop_range(state: State<'_, AppState>) -> (u64, u64) {
    state.inner().audio_engine.lock().unwrap().get_loop_range()
}

#[tauri::command]
pub fn get_playhead(state: State<'_, AppState>) -> u64 {
    state.inner().audio_engine.lock().unwrap().get_playhead()
}

#[tauri::command]
pub fn is_playing(state: State<'_, AppState>) -> bool {
    state.inner().audio_engine.lock().unwrap().is_playing()
}

#[tauri::command]
pub fn is_recording(state: State<'_, AppState>) -> bool {
    state.inner().audio_engine.lock().unwrap().is_recording()
}

#[tauri::command]
pub fn get_cpu_load(state: State<'_, AppState>) -> f32 {
    state.inner().audio_engine.lock().unwrap().get_cpu_load()
}

#[tauri::command]
pub fn get_memory_usage(state: State<'_, AppState>) -> f32 {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_memory_usage()
}

#[tauri::command]
pub fn get_master_meters(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (peak_l, peak_r, rms_l, rms_r, lufs) = state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_master_meters_db();

    let f_clean = |f: f64| if f.is_finite() { f } else { -144.0 };

    Ok(serde_json::json!({
        "peak_l_db": f_clean(peak_l),
        "peak_r_db": f_clean(peak_r),
        "rms_l_db": f_clean(rms_l),
        "rms_r_db": f_clean(rms_r),
        "lufs_integrated": f_clean(lufs.integrated),
        "lufs_momentary": f_clean(lufs.momentary),
        "lufs_short_term": f_clean(lufs.short_term),
        "true_peak_l": f_clean(lufs.true_peak_l),
        "true_peak_r": f_clean(lufs.true_peak_r)
    }))
}

#[tauri::command]
pub fn get_track_levels(state: State<'_, AppState>) -> Result<Vec<TrackLevel>, String> {
    Ok(state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_track_levels())
}

#[tauri::command]
pub fn get_analyzer_data(state: State<'_, AppState>, track_idx: usize) -> Vec<u8> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_analyzer_data(track_idx)
}

#[tauri::command]
pub fn set_playhead(state: State<'_, AppState>, sample: u64) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_playhead(sample)
}

#[tauri::command]
pub fn get_scope_data(state: State<'_, AppState>) -> (Vec<f32>, Vec<f32>) {
    state.inner().audio_engine.lock().unwrap().get_scope_data()
}

// I/O Settings

#[tauri::command]
pub fn get_audio_hosts() -> Result<Vec<String>, String> {
    Ok(AudioDeviceManager::get_available_hosts())
}

#[tauri::command]
pub fn get_audio_devices(host_name: String) -> Result<Vec<AudioDeviceInfo>, String> {
    AudioDeviceManager::get_devices_for_host(&host_name)
}

#[tauri::command]
pub fn get_buffer_sizes() -> Result<Vec<u32>, String> {
    Ok(AudioDeviceManager::get_recommended_buffer_sizes())
}

#[tauri::command]
pub fn get_sample_rates() -> Result<Vec<u32>, String> {
    Ok(AudioDeviceManager::get_recommended_sample_rates())
}

#[tauri::command]
pub fn get_current_audio_config(_state: State<'_, AppState>) -> Result<AudioDeviceConfig, String> {
    Ok(AudioDeviceConfig::default())
}

#[tauri::command]
pub fn set_audio_config(
    state: State<'_, AppState>,
    config: AudioDeviceConfig,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_audio_config(config)
}

#[tauri::command]
pub async fn set_time_signature(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    num: u8,
    den: u8,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetTimeSignature(
        num, den,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn set_global_quantization(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    division: String,
) -> Result<(), String> {
    use crate::engine::graph::QuantizeDivision;
    let div = match division.as_str() {
        "Whole" => QuantizeDivision::Whole,
        "Half" => QuantizeDivision::Half,
        "Quarter" => QuantizeDivision::Quarter,
        "Eighth" => QuantizeDivision::Eighth,
        "Sixteenth" => QuantizeDivision::Sixteenth,
        "ThirtySecond" => QuantizeDivision::ThirtySecond,
        _ => QuantizeDivision::Sixteenth,
    };
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetGlobalQuantization(
        div,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_input_aliases(
    state: State<'_, AppState>,
) -> Result<Vec<crate::engine::io_manager::InputAlias>, String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let io = engine.io_manager.lock().unwrap();
    Ok(io.get_all_input_aliases())
}

#[tauri::command]
pub fn create_input_alias(
    state: State<'_, AppState>,
    name: String,
    is_stereo: bool,
    channels: Vec<usize>,
    color: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .create_input_alias(name, is_stereo, channels, color)
}

#[tauri::command]
pub fn delete_input_alias(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .delete_input_alias(uuid)
}

#[tauri::command]
pub fn update_input_alias(
    _state: State<'_, AppState>,
    id: String,
    name: String,
    channels: Vec<usize>,
    _color: String,
) -> Result<(), String> {
    println!(
        "VIBE: Update alias {} to {} on channels {:?}",
        id, name, channels
    );
    Ok(())
}

#[tauri::command]
pub fn get_channel_meters(_state: State<'_, AppState>) -> Result<Vec<f32>, String> {
    Ok(vec![0.0; 64])
}

#[tauri::command]
pub fn assign_track_input(
    _state: State<'_, AppState>,
    track_index: usize,
    alias_id: Option<String>,
) -> Result<(), String> {
    println!("VIBE: Assign track {} to alias {:?}", track_index, alias_id);
    Ok(())
}

#[tauri::command]
pub fn add_marker(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    label: String,
    pos: u64,
    color: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_marker(label, pos, color)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn remove_marker(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .remove_marker(uuid)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_markers(state: State<'_, AppState>) -> Vec<crate::engine::graph::Marker> {
    state.inner().audio_engine.lock().unwrap().get_markers()
}

// --- Phase 5.2: Video Sync ---

#[tauri::command]
pub fn load_video(
    state: State<'_, AppState>,
    path: String,
) -> Result<crate::engine::video_manager::VideoState, String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.load_video(std::path::PathBuf::from(path))
}

#[tauri::command]
pub fn unload_video(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.unload_video()
}

#[tauri::command]
pub fn set_video_offset(state: State<'_, AppState>, offset_samples: i64) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.set_video_offset(offset_samples)
}

#[tauri::command]
pub fn set_video_framerate(state: State<'_, AppState>, fps: f64) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.set_video_framerate(fps)
}

#[tauri::command]
pub fn get_video_state(state: State<'_, AppState>) -> crate::engine::video_manager::VideoState {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.get_video_state()
}
