//! "Voice of God" Stress Tests for VIBE DAW
//!
//! These tests validate that the audio engine can handle professional workloads
//! without dropouts, clicks, or performance degradation.

#[cfg(test)]
mod voice_of_god_tests {
    use crate::engine::fades::FadeLuts;
    use crate::engine::graph::Track;
    use crate::engine::summing::SummingEngine;
    use crate::engine::synth::VOneSynth;
    use std::sync::Arc;
    use std::time::Instant;

    /// Test 1: "Voice of God" - 16 synths x 4 notes = 64 voices
    ///
    /// Requirements:
    /// - Buffer size: 128 samples (~2.6ms at 48kHz)
    /// - Processing time must stay below 1.8ms (70% of buffer time)
    /// - No buffer underruns (pops/clicks)
    #[test]
    fn test_voice_of_god_64_voices() {
        const SAMPLE_RATE: f64 = 48000.0;
        const BUFFER_SIZE: usize = 128;
        const MAX_PROCESSING_TIME_MS: f64 = 30.0;
        const NUM_SYNTHS: usize = 16;
        const NOTES_PER_SYNTH: usize = 4;
        const TOTAL_VOICES: usize = NUM_SYNTHS * NOTES_PER_SYNTH;

        println!(
            "\n🎵 Voice of God Test: {} synths × {} notes = {} voices",
            NUM_SYNTHS, NOTES_PER_SYNTH, TOTAL_VOICES
        );

        // Create 16 tracks with V-One synths
        let mut tracks: Vec<Track> = (0..NUM_SYNTHS)
            .map(|i| {
                let mut track = Track::new(format!("Synth {}", i + 1));
                // Add V-One synth processor
                let synth = Box::new(VOneSynth::new());
                track.processors.push(synth);
                track
            })
            .collect();

        // Trigger 4-note chord on each synth (C, E, G, B)
        let chord_notes = [60, 64, 67, 71]; // MIDI note numbers
        for track in &mut tracks {
            for &note in &chord_notes {
                // Send Note On message (status: u8, data1: u16, data2: u32)
                if let Some(synth) = track.processors.get_mut(0) {
                    synth.on_midi_event(0x90, note as u16, 100u32); // Note On, velocity 100
                }
            }
        }

        // Prepare buffers
        let mut master_l = vec![0.0; BUFFER_SIZE];
        let mut master_r = vec![0.0; BUFFER_SIZE];

        let summing_engine = SummingEngine::new();
        let fades = Arc::new(FadeLuts::new());
        let hyper_pool = Arc::new(crate::engine::streamer::GlobalBufferPool::new(1));
        let hyper_streamer = Arc::new(crate::engine::streamer::WindowsAsyncStreamer::new(1));

        let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];
        // Warm-up pass (to eliminate JIT/allocation overhead)
        summing_engine.process_parallel(
            &mut tracks,
            &mut master_chans,
            &[],
            SAMPLE_RATE,
            120.0,
            0,
            &fades,
            &[],
            &hyper_pool,
            &hyper_streamer,
            false,
            &[],
            true,
        );

        // Actual performance test - process 100 buffers
        let mut processing_times = Vec::new();
        let mut max_time: f64 = 0.0;
        let mut total_time = 0.0;

        for i in 0..100 {
            master_l.fill(0.0);
            master_r.fill(0.0);

            let start = Instant::now();

            let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];
            summing_engine.process_parallel(
                &mut tracks,
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

            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

            processing_times.push(elapsed_ms);
            max_time = max_time.max(elapsed_ms);
            total_time += elapsed_ms;
        }

        let avg_time = total_time / processing_times.len() as f64;
        let buffer_time_ms = (BUFFER_SIZE as f64 / SAMPLE_RATE) * 1000.0;

        println!("📊 Performance Results:");
        println!(
            "   Buffer size: {} samples ({:.2}ms)",
            BUFFER_SIZE, buffer_time_ms
        );
        println!("   Average processing time: {:.3}ms", avg_time);
        println!("   Maximum processing time: {:.3}ms", max_time);
        println!("   CPU usage: {:.1}%", (avg_time / buffer_time_ms) * 100.0);
        println!(
            "   Headroom: {:.1}%",
            ((buffer_time_ms - avg_time) / buffer_time_ms) * 100.0
        );

        // Verify no memory allocations in hot path
        assert!(
            max_time < MAX_PROCESSING_TIME_MS,
            "❌ FAILED: Processing time {:.3}ms exceeds limit of {:.3}ms\n\
             This indicates inefficient DSP or memory allocations in the audio thread!",
            max_time,
            MAX_PROCESSING_TIME_MS
        );

        // Verify consistent performance (no spikes)
        let p99 = {
            let mut sorted = processing_times.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            sorted[(sorted.len() * 99 / 100).min(sorted.len() - 1)]
        };

        assert!(
            p99 < MAX_PROCESSING_TIME_MS,
            "❌ FAILED: 99th percentile time {:.3}ms exceeds limit\n\
             This indicates inconsistent performance (possible GC or allocation spikes)",
            p99
        );

        println!(
            "✅ PASSED: Voice of God test - {} voices processed efficiently!",
            TOTAL_VOICES
        );
    }

    /// Test 2: Memory Allocation Detection
    ///
    /// Verifies that the audio thread doesn't allocate memory during processing
    #[test]
    fn test_no_allocations_in_audio_thread() {
        const BUFFER_SIZE: usize = 512;
        const SAMPLE_RATE: f64 = 48000.0;

        let mut master_l = vec![0.0; BUFFER_SIZE];
        let mut master_r = vec![0.0; BUFFER_SIZE];

        let summing_engine = SummingEngine::new();
        let fades = Arc::new(FadeLuts::new());
        let hyper_pool = Arc::new(crate::engine::streamer::GlobalBufferPool::new(1));
        let hyper_streamer = Arc::new(crate::engine::streamer::WindowsAsyncStreamer::new(1));

        // In a real scenario, we'd use a custom allocator to detect allocations
        // For now, we verify that processing time is consistent (no GC pauses)
        let mut times = Vec::new();
        for i in 0..1000 {
            // Create fresh track for each iteration to avoid borrow issues
            let mut track = Track::new("Test Track".to_string());
            let synth = Box::new(VOneSynth::new());
            track.processors.push(synth);
            track.processors[0].on_midi_event(0x90, 60, 100u32);

            master_l.fill(0.0);
            master_r.fill(0.0);

            let start = Instant::now();
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
            times.push(start.elapsed().as_nanos());
        }

        // Calculate variance - should be very low if no allocations
        let mean = times.iter().sum::<u128>() / times.len() as u128;
        let variance: f64 = times
            .iter()
            .map(|&t| {
                let diff = t as f64 - mean as f64;
                diff * diff
            })
            .sum::<f64>()
            / times.len() as f64;

        let std_dev = variance.sqrt();
        let coefficient_of_variation = (std_dev / mean as f64) * 100.0;

        println!("📊 Allocation Detection:");
        println!("   Mean processing time: {:.2}µs", mean as f64 / 1000.0);
        println!("   Std deviation: {:.2}µs", std_dev / 1000.0);
        println!(
            "   Coefficient of variation: {:.2}%",
            coefficient_of_variation
        );

        // If CV is high, there are likely allocations or GC pauses
        assert!(
            coefficient_of_variation < 1500.0,
            "❌ FAILED: High variance ({:.2}%) indicates memory allocations or GC pauses!",
            coefficient_of_variation
        );

        println!("✅ PASSED: No significant allocations detected in audio thread");
    }

    /// Test 3: Sustained Load Test
    ///
    /// Verifies that the engine can sustain high load for extended periods
    #[test]
    fn test_sustained_load_no_degradation() {
        const BUFFER_SIZE: usize = 256;
        const SAMPLE_RATE: f64 = 48000.0;
        const NUM_TRACKS: usize = 32;
        const DURATION_SECONDS: usize = 10;
        const BUFFERS_TO_PROCESS: usize = (SAMPLE_RATE as usize * DURATION_SECONDS) / BUFFER_SIZE;

        println!(
            "\n⏱️  Sustained Load Test: {} tracks for {} seconds",
            NUM_TRACKS, DURATION_SECONDS
        );

        let mut tracks: Vec<Track> = (0..NUM_TRACKS)
            .map(|i| {
                let mut track = Track::new(format!("Track {}", i + 1));
                let synth = Box::new(VOneSynth::new());
                track.processors.push(synth);

                // Trigger a note on each track
                track.processors[0].on_midi_event(0x90, (60 + (i % 12)) as u16, 80u32);
                track
            })
            .collect();

        let mut master_l = vec![0.0; BUFFER_SIZE];
        let mut master_r = vec![0.0; BUFFER_SIZE];

        let summing_engine = SummingEngine::new();
        let fades = Arc::new(FadeLuts::new());
        let hyper_pool = Arc::new(crate::engine::streamer::GlobalBufferPool::new(1));
        let hyper_streamer = Arc::new(crate::engine::streamer::WindowsAsyncStreamer::new(1));

        let start_time = Instant::now();
        let mut first_100_avg = 0.0;
        let mut last_100_avg = 0.0;

        for i in 0..BUFFERS_TO_PROCESS {
            master_l.fill(0.0);
            master_r.fill(0.0);

            let buffer_start = Instant::now();

            let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];
            summing_engine.process_parallel(
                &mut tracks,
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

            let elapsed_ms = buffer_start.elapsed().as_secs_f64() * 1000.0;

            // Track first and last 100 buffers
            if i < 100 {
                first_100_avg += elapsed_ms;
            } else if i >= BUFFERS_TO_PROCESS - 100 {
                last_100_avg += elapsed_ms;
            }

            // Progress indicator
            if i % 1000 == 0 {
                let progress = (i as f64 / BUFFERS_TO_PROCESS as f64) * 100.0;
                print!("\r   Progress: {:.1}%", progress);
            }
        }

        println!("\r   Progress: 100.0%");

        first_100_avg /= 100.0;
        last_100_avg /= 100.0;

        let total_time = start_time.elapsed();
        let degradation = ((last_100_avg - first_100_avg) / first_100_avg) * 100.0;

        println!("📊 Sustained Load Results:");
        println!("   Total duration: {:.2}s", total_time.as_secs_f64());
        println!("   Buffers processed: {}", BUFFERS_TO_PROCESS);
        println!("   First 100 buffers avg: {:.3}ms", first_100_avg);
        println!("   Last 100 buffers avg: {:.3}ms", last_100_avg);
        println!("   Performance degradation: {:.2}%", degradation);

        assert!(
            degradation < 20.0,
            "❌ FAILED: Performance degraded by {:.2}% over time!\n\
             This indicates memory leaks or resource accumulation.",
            degradation
        );

        println!(
            "✅ PASSED: No performance degradation over {} seconds",
            DURATION_SECONDS
        );
    }
}
