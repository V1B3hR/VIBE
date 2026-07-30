use crate::engine::midi_mapping::MidiBinding;
use crate::state::{emit_project_update, AppState};
use tauri::State;

#[tauri::command]
pub fn note_on(state: State<'_, AppState>, note: u8, velocity: u8) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .note_on(note, velocity)
}

#[tauri::command]
pub fn note_off(state: State<'_, AppState>, note: u8) -> Result<(), String> {
    state.inner().audio_engine.lock().unwrap().note_off(note)
}

#[tauri::command]
pub fn map_midi(state: State<'_, AppState>, cc: u8, param_id: String) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .map_midi(cc, param_id)
}

#[tauri::command]
pub fn start_midi_learn(state: State<'_, AppState>, param_id: String) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .start_midi_learn(param_id)
}

#[tauri::command]
pub fn remove_midi_binding(state: State<'_, AppState>, binding_id: String) -> Result<(), String> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .remove_midi_binding(binding_id)
}

#[tauri::command]
pub fn get_midi_bindings(state: State<'_, AppState>) -> Vec<MidiBinding> {
    state
        .inner()
        .audio_engine
        .lock()
        .unwrap()
        .get_midi_bindings()
}

#[tauri::command]
pub fn add_cc_event(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: String,
    sample: u64,
    cc: u8,
    value: u8,
) -> Result<(), String> {
    use crate::engine::graph::MidiCCEvent;
    let event = MidiCCEvent {
        sample,
        cc_number: cc,
        value: value as u32,
        channel: 0,
    };
    let engine = state.inner().audio_engine.lock().unwrap();
    engine.send_command(crate::engine::audio::AudioCommand::AddCCEvent(
        track_idx, clip_id, event,
    ))?;
    emit_project_update(&app, &state);
    Ok(())
}
