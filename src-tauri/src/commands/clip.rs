use crate::engine::graph::{GrooveTemplate, MidiClip, MidiNote, QuantizeDivision, WarpMode};
use crate::state::{emit_project_update, AppState};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn add_clip_to_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_index: usize,
    clip_id: String,
    start_pos: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_clip_to_track(track_index, clip_id, start_pos)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn slice_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_index: usize,
    clip_id: String,
    sample_pos: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .slice_clip(track_index, clip_id, sample_pos)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn move_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    src_idx: usize,
    clip_id: String,
    dest_idx: usize,
    new_pos: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .move_clip(src_idx, clip_id, dest_idx, new_pos)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn resize_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    new_start: u64,
    new_offset: u64,
    new_len: u64,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .resize_clip(track_idx, clip_id, new_start, new_offset, new_len)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn delete_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .delete_clip(track_idx, clip_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn rename_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    new_name: String,
    is_midi: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .rename_clip(track_idx, clip_id, new_name, is_midi)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn reverse_audio_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .reverse_audio_clip(track_idx, clip_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn duplicate_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    is_midi: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .duplicate_clip(track_idx, clip_id, is_midi)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_clip_gain(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    gain: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_clip_gain(track_idx, clip_id, gain)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn normalize_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .normalize_clip(track_idx, clip_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn consolidate_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    is_midi: bool,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .consolidate_clip(track_idx, clip_id, is_midi)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_audio_clip_warp_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    mode: String,
) -> Result<(), String> {
    let mode_enum = match mode.to_lowercase().as_str() {
        "beats" => WarpMode::Beats,
        "tones" => WarpMode::Tones,
        "texture" => WarpMode::Texture,
        "repitch" => WarpMode::Repitch,
        "complex" => WarpMode::Complex,
        _ => return Err(format!("Invalid warp mode: {}", mode)),
    };
    let clip_uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetAudioClipWarpMode(
        track_idx, clip_uuid, mode_enum,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_clip_color(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    color: String,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetClipColor(
        track_idx, cid, color,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

// MIDI Note Commands

#[tauri::command]
pub async fn add_midi_note(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    note: MidiNote,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_midi_note(track_idx, clip_id, note)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn delete_midi_note(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    note_idx: usize,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .delete_midi_note(track_idx, clip_id, note_idx)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn update_midi_note(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    note_idx: usize,
    note: MidiNote,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .update_midi_note(track_idx, clip_id, note_idx, note)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_track_midi_clips(state: State<'_, AppState>, track_idx: usize) -> Vec<MidiClip> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_midi_clips_for_track(track_idx)
}

#[tauri::command]
pub async fn quantize_notes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    division: String,
) -> Result<(), String> {
    let div = match division.as_str() {
        "Whole" => QuantizeDivision::Whole,
        "Half" => QuantizeDivision::Half,
        "Quarter" => QuantizeDivision::Quarter,
        "Eighth" => QuantizeDivision::Eighth,
        "Sixteenth" => QuantizeDivision::Sixteenth,
        "ThirtySecond" => QuantizeDivision::ThirtySecond,
        _ => QuantizeDivision::Sixteenth,
    };
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .quantize_notes(track_idx, clip_id, div)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn apply_groove_custom(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    template: GrooveTemplate,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::ApplyGrooveCustom(
        track_idx, clip_id, template,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn humanize_midi_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    timing: f32,
    velocity: f32,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::HumanizeMidiClip(
        track_idx, cid, timing, velocity,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn convert_audio_to_midi(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    mode: String,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::ConvertAudioToMidi(
        track_idx, cid, mode,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn extract_groove(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::ExtractGroove(
        track_idx, cid,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn transpose_midi_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    semitones: i32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .transpose_midi_clip(track_idx, clip_id, semitones)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn duplicate_midi_notes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    note_indices: Vec<usize>,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .duplicate_midi_notes(track_idx, clip_id, note_indices)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_midi_clip_data(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<MidiClip, String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_midi_clip_data(track_idx, clip_id)
}

#[tauri::command]
pub fn set_clip_scale(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    scale_info: Option<crate::engine::graph::Scale>,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_clip_scale(track_idx, clip_id, scale_info)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn detect_chords(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .detect_chords(track_idx, clip_id)
}

#[tauri::command]
pub fn add_automation_point(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    param_id: String,
    time_samples: u64,
    value: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_automation_point(param_id, time_samples, value as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn set_automation_tension(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    param_id: String,
    time_samples: u64,
    tension: f32,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_automation_tension(param_id, time_samples, tension as f64)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn clear_automation(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    param_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .clear_automation(param_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_waveform_chunk(
    state: State<'_, AppState>,
    clip_id: String,
    lod_level: u8,
) -> Result<Vec<u8>, String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_waveform_data(clip_id, lod_level)
}

#[tauri::command]
pub async fn get_raw_samples(
    state: State<'_, AppState>,
    clip_id: String,
    start_sample: u64,
    end_sample: u64,
) -> Result<Vec<f32>, String> {
    let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.get_samples_range(uuid, start_sample, end_sample)
}

#[tauri::command]
pub fn get_clip_data(state: State<'_, AppState>, clip_id: String) -> Result<Vec<f32>, String> {
    let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_clip_data(uuid)
        .map(|data| (*data).clone())
        .ok_or_else(|| "Clip not found".to_string())
}

#[tauri::command]
pub fn set_automation_interpolation(
    state: State<'_, AppState>,
    param_id: String,
    interp_type: crate::engine::automation::InterpolationType,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_automation_interpolation(param_id, interp_type)
}

#[tauri::command]
pub fn set_automation_layer(
    state: State<'_, AppState>,
    param_id: String,
    layer: crate::engine::automation::AutomationLayer,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .set_automation_layer(param_id, layer)
}

#[tauri::command]
pub fn set_clip_envelope(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    env_type: String,
    curve: crate::engine::automation::AutomationCurve,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::SetClipEnvelope(
        track_idx, uuid, env_type, curve,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn add_midi_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip: MidiClip,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .add_midi_clip(track_idx, clip)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn delete_midi_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .delete_midi_clip(track_idx, clip_id)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn update_midi_clip(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    uuid: String,
    clip: MidiClip,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .update_midi_clip(track_idx, uuid, clip)?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub async fn apply_groove_template(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    template_name: String,
) -> Result<(), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::ApplyGrooveTemplate(
        track_idx,
        clip_id,
        template_name,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn detect_transients(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::DetectTransients(
        track_idx, uuid,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn export_audio_clip(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    path: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .export_audio_clip(track_idx, clip_id, path)
}

#[tauri::command]
pub fn export_midi_clip(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    path: String,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .export_midi_clip(track_idx, clip_id, path)
}

#[tauri::command]
pub async fn generate_stress_notes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    count: usize,
) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .generate_stress_notes(track_idx, clip_id, count)?;
    emit_project_update(&app, &state);
    Ok(())
}

/// Snaps loop start and/or end to the nearest zero-crossing in the audio data.
/// This prevents clicks and cracks when looping at positions that don't cross zero.
/// `search_window_ms` controls how many milliseconds around each point to search.
#[tauri::command]
pub fn snap_loop_to_zero(
    state: State<'_, AppState>,
    loop_start: u64,
    loop_end: u64,
    search_window_ms: u32,
) -> Result<(u64, u64), String> {
    let engine = state.inner().audio_engine.lock().unwrap();
    let sample_rate = 48000u32;
    let window_samples = ((search_window_ms as u64) * sample_rate as u64) / 1000;

    // We search for zero crossings in the master mix-down buffer.
    // If not available, fall back to the original positions gracefully.
    let snapped_start = engine
        .find_zero_crossing_near(loop_start, window_samples)
        .unwrap_or(loop_start);
    let snapped_end = engine
        .find_zero_crossing_near(loop_end, window_samples)
        .unwrap_or(loop_end);

    // Ensure start < end
    if snapped_start >= snapped_end {
        return Err("Snapped loop start must be before end".to_string());
    }

    Ok((snapped_start, snapped_end))
}

/// Returns statistics about a clip: peak amplitude (dBFS), RMS (dBFS),
/// duration in samples, DC offset, and crest factor.
#[tauri::command]
pub fn get_clip_statistics(
    state: State<'_, AppState>,
    clip_id: String,
) -> Result<ClipStats, String> {
    let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
    let engine = state.inner().audio_engine.lock().unwrap();
    let samples = engine
        .get_clip_data(uuid)
        .ok_or_else(|| "Clip not found".to_string())?;

    if samples.is_empty() {
        return Ok(ClipStats::default());
    }

    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    let mut dc_sum = 0.0f64;

    for &s in samples.iter() {
        let abs_s = s.abs();
        if abs_s > peak {
            peak = abs_s;
        }
        sum_sq += (s as f64) * (s as f64);
        dc_sum += s as f64;
    }

    let n = samples.len() as f64;
    let rms = (sum_sq / n).sqrt() as f32;
    let dc_offset = (dc_sum / n) as f32;

    // dBFS conversion (−inf for silence)
    let peak_db = if peak > 1e-10 { 20.0 * peak.log10() } else { -144.0 };
    let rms_db = if rms > 1e-10 { 20.0 * rms.log10() } else { -144.0 };
    let crest_factor = if rms > 1e-10 { peak_db - rms_db } else { 0.0 };

    Ok(ClipStats {
        duration_samples: samples.len() as u64,
        peak_db,
        rms_db,
        dc_offset,
        crest_factor,
    })
}

#[derive(serde::Serialize, Default)]
pub struct ClipStats {
    pub duration_samples: u64,
    pub peak_db: f32,
    pub rms_db: f32,
    pub dc_offset: f32,
    pub crest_factor: f32,
}

