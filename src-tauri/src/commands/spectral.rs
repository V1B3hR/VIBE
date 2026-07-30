use crate::engine::spectral::{MelProcessor, MelSpectrogramConfig, SpectralAnalysisResult};
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn analyze_spectral(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: Uuid,
) -> Result<SpectralAnalysisResult, String> {
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
        clip.head_data.as_ref().clone()
    };

    if samples.is_empty() {
        return Err("Clip data is empty or not loaded".into());
    }

    // 2. Process
    let config = MelSpectrogramConfig::default();
    let processor = MelProcessor::new(config.clone());

    let mut frames = Vec::new();
    let hop_size = config.hop_size;
    let fft_size = config.fft_size;

    let mut offset = 0;
    while offset + fft_size <= samples.len() {
        let frame_samples = &samples[offset..offset + fft_size];
        let frame = processor.process_frame(frame_samples, offset as u64);
        frames.push(frame);
        offset += hop_size;
    }

    Ok(SpectralAnalysisResult {
        frames,
        duration_samples: samples.len() as u64,
    })
}

#[tauri::command]
pub async fn convert_clip_to_drums(
    state: State<'_, AppState>,
    track_idx: usize,
    clip_id: Uuid,
) -> Result<crate::engine::graph::MidiClip, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;

    // 1. Get audio
    let (samples, sample_rate) = {
        let tracks = audio_engine.tracks.lock().map_err(|e| e.to_string())?;
        let track = tracks.get(track_idx).ok_or("Track not found")?;
        let clip = track
            .clips
            .iter()
            .find(|c| c.id == clip_id)
            .ok_or("Clip not found")?;
        (clip.head_data.as_ref().clone(), clip.sample_rate as f64)
    };

    // 2. Spectrogram
    let config = MelSpectrogramConfig::default();
    let processor = MelProcessor::new(config.clone());
    let mut frames = Vec::new();
    let mut offset = 0;
    while offset + config.fft_size <= samples.len() {
        let frame =
            processor.process_frame(&samples[offset..offset + config.fft_size], offset as u64);
        frames.push(frame);
        offset += config.hop_size;
    }

    // 3. Transcribe
    let transcriber = crate::engine::spectral::Transcriber::new();
    let midi_clip = transcriber.transcribe_drums(&frames, sample_rate);

    Ok(midi_clip)
}
