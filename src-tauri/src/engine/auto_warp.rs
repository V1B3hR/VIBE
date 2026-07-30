use crate::engine::time_stretch::TimeStretcher;

/// Marker for manual and automatic warping.
/// Maps a position in original samples to a position on the project timeline (beats).
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct WarpMarker {
    pub original_pos_samples: u64, // position in original file
    pub timeline_pos_beats: f64,   // target position in musical beats
    pub locked: bool,              // prevents accidental move during auto-snap
}

/// Modes for audio warping/stretching.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum StretchMode {
    Off,
    Beats,   // Rhythmic material (drums)
    Tones,   // Melodic (vocals)
    Texture, // Ambient
    RePitch, // Classic tape-style
    Complex, // Polyphonic
}

/// Tempo detection methods for the Auto-Warp engine.
#[allow(dead_code)]
pub enum TempoMethod {
    Autocorrelation,
    BeatTracking,
    OnsetClustering,
    SpectralFlux,
}

/// Automatic tempo detection engine.
#[allow(dead_code)]
pub struct TempoDetector {
    pub method: TempoMethod,
}

#[allow(dead_code)]
impl TempoDetector {
    pub fn new(method: TempoMethod) -> Self {
        Self { method }
    }

    pub fn detect(&self, _audio: &[f32]) -> Result<f64, String> {
        // Phase 2 MVP: Placeholder for BPM detection
        Ok(120.0)
    }
}

/// Auto-Warp / Tempo Sync Engine.
#[allow(dead_code)]
pub struct AutoWarp {
    pub detector: TempoDetector,
    pub stretcher: TimeStretcher,
}

#[allow(dead_code)]
impl AutoWarp {
    pub fn new(stretcher: TimeStretcher) -> Self {
        Self {
            detector: TempoDetector::new(TempoMethod::BeatTracking),
            stretcher,
        }
    }

    /// Automatically sync audio to project tempo.
    pub fn sync_to_project(&mut self, audio: &[f32], project_bpm: f64) -> Result<Vec<f32>, String> {
        // 1. Detect audio BPM
        let detected_bpm = self.detector.detect(audio)?;

        // 2. Calculate stretch ratio
        let ratio = detected_bpm / project_bpm;

        // 3. Stretch
        Ok(self.stretcher.process(audio, ratio))
    }

    /// Warp audio based on specific markers (Ableton-style).
    /// Performs segment-based stretching between markers.
    pub fn warp_with_markers(
        &mut self,
        audio: &[f32],
        warp_markers: &[WarpMarker],
        project_bpm: f64,
        sample_rate: f64,
    ) -> Vec<f32> {
        if warp_markers.is_empty() {
            return audio.to_vec();
        }

        let mut sorted = warp_markers.to_vec();
        sorted.sort_by_key(|m| m.original_pos_samples);

        let channels = self.stretcher.channels;
        let mut final_audio = Vec::new();
        
        // Helper to convert beats to samples
        let beats_to_samples = |beats: f64| -> f64 {
            let seconds = beats / (project_bpm / 60.0);
            seconds * sample_rate
        };

        let mut current_orig_pos = 0;
        let mut current_target_pos = 0.0; // In samples

        for marker in sorted {
            // Segment from current_orig_pos to marker.original_pos_samples
            let orig_len = marker.original_pos_samples.saturating_sub(current_orig_pos);
            let target_pos_samples = beats_to_samples(marker.timeline_pos_beats);
            let target_len = target_pos_samples - current_target_pos;

            if orig_len > 0 && target_len > 0.0 {
                let ratio = orig_len as f64 / target_len;
                let start_idx = (current_orig_pos as usize * channels).min(audio.len());
                let end_idx = (marker.original_pos_samples as usize * channels).min(audio.len());
                
                if start_idx < end_idx {
                    let segment = &audio[start_idx..end_idx];
                    let mut stretched = self.stretcher.process(segment, ratio);
                    final_audio.append(&mut stretched);
                }
            } else if orig_len > 0 {
                // Target length is 0 (or negative), just drop audio or compress infinitely. 
                // We'll skip adding to final_audio.
            }

            current_orig_pos = marker.original_pos_samples;
            current_target_pos = target_pos_samples;
        }

        // Tail segment (after last marker to the end of the file)
        // We assume 1:1 stretch (ratio = 1.0) for the tail, or continue the last segment's ratio.
        // Ableton usually continues the last known ratio or 1.0. We will use 1.0 for simplicity.
        let orig_len = (audio.len() / channels) as u64 - current_orig_pos;
        if orig_len > 0 {
            let start_idx = (current_orig_pos as usize * channels).min(audio.len());
            if start_idx < audio.len() {
                let segment = &audio[start_idx..];
                // Process with 1.0 ratio
                let mut stretched = self.stretcher.process(segment, 1.0);
                final_audio.append(&mut stretched);
            }
        }

        final_audio
    }
}
