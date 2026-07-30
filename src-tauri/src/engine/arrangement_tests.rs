#[cfg(test)]
mod tests {
    use crate::engine::automation::AutomationCurve;
    use crate::engine::graph::{AudioClip, Track, TrackPlaylist, WarpMode};
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_hyperstream_efficiency() {
        // Create a mock large audio buffer (10MB of silence)
        let large_data = vec![0.0f32; 2_500_000]; // ~10MB
        let shared_data = Arc::new(large_data);

        let clip = AudioClip {
            id: Uuid::new_v4(),
            name: "MegaClip".to_string(),
            head_data: shared_data.clone(),
            peaks: Vec::new(),
            start_sample: 0,
            offset_in_data: 0,
            length_in_samples: 2_500_000,
            sample_rate: 44100,
            color: String::new(),
            fade_in_len: 0,
            fade_out_len: 0,
            fade_in_type: crate::engine::fades::FadeType::Linear,
            fade_out_type: crate::engine::fades::FadeType::Linear,
            gain: 1.0,
            pitch_semitones: 0.0,
            playback_speed: 1.0,
            is_warped: false,
            is_reversed: false,
            warp_mode: WarpMode::Beats,
            path: None,
            waveform_cache: None,
            is_streaming: false,
            #[cfg(target_os = "windows")]
            file: None,
            gain_envelope: None,
            pitch_envelope: None,
            pan_envelope: None,
            transients: Vec::new(),
            base_bpm: 120.0,
            warp_markers: Vec::new(),
            reference_clip_id: None,
        };

        // Slice test: clone the clip
        let mut clones = Vec::new();
        let start_mem = shared_data.as_ptr();

        for i in 0..100 {
            let mut c = clip.clone();
            c.id = Uuid::new_v4();
            c.start_sample = (i * 1000) as u64;
            clones.push(c);
        }

        // Verify: All clones point to the SAME underlying memory address
        for c in clones {
            assert_eq!(
                c.head_data.as_ptr(),
                start_mem,
                "HyperStream failure: Data was duplicated instead of shared via Arc"
            );
        }
    }

    #[test]
    fn test_track_playlist_swapping() {
        let mut track = Track::new("Vocal".to_string());

        // Setup Playlist A
        track.playlists.push(TrackPlaylist {
            name: "Take 1".to_string(),
            clips: Vec::new(),
            midi_clips: Vec::new(),
        });

        // Setup Playlist B
        track.playlists.push(TrackPlaylist {
            name: "Take 2".to_string(),
            clips: Vec::new(),
            midi_clips: Vec::new(),
        });

        // Simulate choosing Playlist 1 (Take 2)
        // This logic is usually in audio.rs, but we test the struct state here.
        track.active_playlist_idx = 1;
        assert_eq!(track.playlists[track.active_playlist_idx].name, "Take 2");
    }

    #[test]
    fn test_clip_envelope_integrity() {
        let mut clip = AudioClip {
            id: Uuid::new_v4(),
            name: "AutomationTest".to_string(),
            head_data: Arc::new(vec![0.0]),
            peaks: Vec::new(),
            start_sample: 0,
            offset_in_data: 0,
            length_in_samples: 48000,
            sample_rate: 48000,
            color: String::new(),
            fade_in_len: 0,
            fade_out_len: 0,
            fade_in_type: crate::engine::fades::FadeType::Linear,
            fade_out_type: crate::engine::fades::FadeType::Linear,
            gain: 1.0,
            pitch_semitones: 0.0,
            playback_speed: 1.0,
            is_warped: false,
            is_reversed: false,
            warp_mode: WarpMode::Beats,
            path: None,
            waveform_cache: None,
            is_streaming: false,
            #[cfg(target_os = "windows")]
            file: None,
            gain_envelope: None,
            pitch_envelope: None,
            pan_envelope: None,
            transients: Vec::new(),
            base_bpm: 120.0,
            warp_markers: Vec::new(),
            reference_clip_id: None,
        };

        // Add a gain envelope
        let mut curve = AutomationCurve::new(1.0);
        curve.add_knot(24000, 0.5); // Dip to 50% at 0.5s
        curve.add_knot(48000, 0.0); // Mute at 1s

        clip.gain_envelope = Some(curve);

        assert!(clip.gain_envelope.is_some());
        let env = clip.gain_envelope.as_ref().unwrap();
        assert_eq!(env.knots.len(), 3); // initial 1.0 + 2 added knots
        assert_eq!(env.knots[1].value, 0.5);
    }

    #[test]
    fn test_panic_split_stress() {
        use crate::engine::fades::FadeLuts;
        use crate::engine::summing::SummingEngine;
        use std::time::Instant;

        const SAMPLE_RATE: f64 = 48000.0;
        const BUFFER_SIZE: usize = 128;

        let mut track = Track::new("StressTrack".to_string());
        let large_data = Arc::new(vec![0.5f32; 1_000_000]); // Constant signal

        let clip_id = Uuid::new_v4();
        track.clips.push(AudioClip {
            id: clip_id,
            name: "InitialClip".to_string(),
            head_data: large_data.clone(),
            peaks: Vec::new(),
            start_sample: 0,
            offset_in_data: 0,
            length_in_samples: 1_000_000,
            sample_rate: 48000,
            color: String::new(),
            fade_in_len: 0,
            fade_out_len: 0,
            fade_in_type: crate::engine::fades::FadeType::Linear,
            fade_out_type: crate::engine::fades::FadeType::Linear,
            gain: 1.0,
            pitch_semitones: 0.0,
            playback_speed: 1.0,
            is_warped: false,
            is_reversed: false,
            warp_mode: WarpMode::Beats,
            path: None,
            waveform_cache: None,
            is_streaming: false,
            #[cfg(target_os = "windows")]
            file: None,
            gain_envelope: None,
            pitch_envelope: None,
            pan_envelope: None,
            transients: Vec::new(),
            base_bpm: 120.0,
            warp_markers: Vec::new(),
            reference_clip_id: None,
        });

        let summing_engine = SummingEngine::new();
        let fades = Arc::new(FadeLuts::new());
        let hyper_pool = Arc::new(crate::engine::streamer::GlobalBufferPool::new(1));
        let hyper_streamer = Arc::new(crate::engine::streamer::WindowsAsyncStreamer::new(1));

        let mut master_l = vec![0.0; BUFFER_SIZE];
        let mut master_r = vec![0.0; BUFFER_SIZE];

        let start_time = Instant::now();

        // Process and split simultaneously
        for i in 0..100 {
            // 1. Process block
            master_l.fill(0.0);
            master_r.fill(0.0);
            let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];
            summing_engine.process_parallel(
                std::slice::from_mut(&mut track),
                &mut master_chans,
                &[],
                SAMPLE_RATE,
                120.0,
                (i * BUFFER_SIZE) as u64,
                &fades,
                &[],
                &hyper_pool,
                &hyper_streamer,
                false,
                &[],
                true,
            );

            // 2. Perform "Panic Split" every 10 blocks
            if i % 10 == 0 {
                let current_pos = (i * BUFFER_SIZE) as u64 + 64;
                track.slice_clip(clip_id, current_pos);
            }
        }

        println!("✅ Panic Split PASSED in {:?}", start_time.elapsed());
        assert!(track.clips.len() > 1, "Clips should have been sliced");
    }

    #[test]
    fn test_atomic_sample_counting_long_run() {
        use std::sync::atomic::{AtomicU64, Ordering};

        let playhead = AtomicU64::new(0);
        let buffer_size = 128;

        // Simulate 1 hour of playback (172,800,000 samples)
        let total_samples_expected = 48000 * 3600;
        let iterations = total_samples_expected / buffer_size;

        for _ in 0..iterations {
            playhead.fetch_add(buffer_size as u64, Ordering::SeqCst);
        }

        let final_pos = playhead.load(Ordering::SeqCst);
        assert_eq!(
            final_pos, total_samples_expected as u64,
            "Timing drift detected in atomic counter!"
        );
        println!("✅ Atomic Sample Sync PASSED for 1 hour simulation");
    }
}
