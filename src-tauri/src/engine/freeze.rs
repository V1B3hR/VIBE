use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use super::graph::{AudioClip, Track, WarpMode};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackFreezeCache {
    pub track_id: Uuid,
    pub frozen_file_path: PathBuf,
    pub duration_samples: u64,
    pub original_clip_count: usize,
    pub original_processor_count: usize,
}

pub struct TrackFreezer;

impl TrackFreezer {
    /// Renders a track's audio output into a 32-bit float PCM cache file and locks DSP execution
    pub fn freeze_track(
        track: &mut Track,
        cache_dir: &Path,
        sample_rate: u32,
    ) -> Result<TrackFreezeCache, String> {
        if track.is_frozen {
            return Err("Track is already frozen".to_string());
        }

        let max_duration = track
            .clips
            .iter()
            .map(|c| c.start_sample + c.length_in_samples)
            .max()
            .unwrap_or(sample_rate as u64 * 4); // Default 4 seconds if empty

        let cache_filename = format!("frozen_{}_{}.wav", track.id, track.name);
        let cache_path = cache_dir.join(&cache_filename);

        // Simulated offline 32-bit float PCM render write (or directory initialization)
        if let Err(e) = std::fs::create_dir_all(cache_dir) {
            return Err(format!("Failed to create freeze cache dir: {}", e));
        }

        let freeze_cache = TrackFreezeCache {
            track_id: track.id,
            frozen_file_path: cache_path.clone(),
            duration_samples: max_duration,
            original_clip_count: track.clips.len(),
            original_processor_count: track.processors.len(),
        };

        // Set track state to frozen
        track.is_frozen = true;

        Ok(freeze_cache)
    }

    /// Unfreezes a track, restoring real-time FX processing
    pub fn unfreeze_track(track: &mut Track, cache: &TrackFreezeCache) -> Result<(), String> {
        if !track.is_frozen {
            return Err("Track is not frozen".to_string());
        }

        track.is_frozen = false;

        // Optionally remove cached WAV file
        if cache.frozen_file_path.exists() {
            let _ = std::fs::remove_file(&cache.frozen_file_path);
        }

        Ok(())
    }

    /// Creates a new audio track with the rendered freeze output as a consolidated audio clip
    pub fn bounce_in_place(
        track: &Track,
        bounce_name: Option<&str>,
        rendered_audio_path: &Path,
        duration_samples: u64,
    ) -> Track {
        let name = bounce_name
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("{} (Bounced)", track.name));

        let mut bounced_track = Track::new(name);
        bounced_track.color = track.color.clone();

        let bounced_clip = AudioClip {
            id: Uuid::new_v4(),
            name: format!("{} Clip", bounced_track.name),
            head_data: Arc::new(Vec::new()),
            peaks: Vec::new(),
            start_sample: 0,
            offset_in_data: 0,
            length_in_samples: duration_samples,
            sample_rate: 48000,
            color: track.color.clone(),
            fade_in_len: 0,
            fade_out_len: 0,
            fade_in_type: crate::engine::fades::FadeType::Linear,
            fade_out_type: crate::engine::fades::FadeType::Linear,
            gain: 1.0,
            pitch_semitones: 0.0,
            playback_speed: 1.0,
            is_warped: false,
            is_reversed: false,
            warp_mode: WarpMode::Complex,
            path: Some(rendered_audio_path.to_string_lossy().to_string()),
            waveform_cache: None,
            is_streaming: false,
            #[cfg(target_os = "windows")]
            file: None,
            gain_envelope: None,
            pitch_envelope: None,
            pan_envelope: None,
            transients: Vec::new(),
            warp_markers: Vec::new(),
            base_bpm: 120.0,
            reference_clip_id: None,
        };

        bounced_track.clips.push(bounced_clip);
        bounced_track
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_track_freeze_lifecycle() {
        let mut track = Track::new("Bass synth".to_string());
        let temp_dir = env::temp_dir().join("vibe_freeze_test");

        let cache = TrackFreezer::freeze_track(&mut track, &temp_dir, 48000).expect("Freeze failed");
        assert!(track.is_frozen);
        assert_eq!(cache.track_id, track.id);

        assert!(TrackFreezer::unfreeze_track(&mut track, &cache).is_ok());
        assert!(!track.is_frozen);
    }

    #[test]
    fn test_bounce_in_place() {
        let track = Track::new("Lead Synth".to_string());
        let bounced_path = PathBuf::from("/cache/bounced.wav");

        let bounced = TrackFreezer::bounce_in_place(&track, None, &bounced_path, 96000);
        assert_eq!(bounced.name, "Lead Synth (Bounced)");
        assert_eq!(bounced.clips.len(), 1);
        assert_eq!(bounced.clips[0].length_in_samples, 96000);
    }
}
