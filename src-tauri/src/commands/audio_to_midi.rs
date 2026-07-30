use crate::engine::audio_to_midi::AudioToMidiConverter;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn convert_audio_clip_to_midi(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: Uuid,
) -> Result<String, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;

    // 1. Get audio samples from clip
    let samples = {
        let tracks = audio_engine.tracks.lock().map_err(|e| e.to_string())?;
        let track = tracks.get(track_idx).ok_or("Track not found")?;
        let clip = track
            .clips
            .iter()
            .find(|c| c.id == clip_id)
            .ok_or("Clip not found")?;

        // This is a simplification; ideally we'd have a way to get full data
        // For now we use head_data
        clip.head_data.as_ref().clone()
    };

    if samples.is_empty() {
        return Err("Clip data is empty or not loaded".into());
    }

    // 2. Convert
    let sample_rate = audio_engine.get_sample_rate() as f64;
    let converter = AudioToMidiConverter::new(sample_rate);
    let mut midi_clip = converter.convert_polyphonic(&samples); // Default to polyphonic for Level 3

    midi_clip.name = format!("MIDI_{}", clip_id);

    // 3. Add to track
    {
        let mut tracks = audio_engine.tracks.lock().map_err(|e| e.to_string())?;
        let track = tracks.get_mut(track_idx).ok_or("Track not found")?;
        track.midi_clips.push(midi_clip);
    }

    Ok("Conversion successful! MIDI clip added to track.".into())
}
