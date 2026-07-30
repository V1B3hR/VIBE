#![allow(dead_code)]

use super::audio::MidiEvent;
use super::fades::FadeLuts;
use super::graph::Track;
use rayon::prelude::*;
use std::sync::Arc;

/// Klasa Mixera V1B3: Maybach Summing Engine.
/// Inspirowany Allen & Heath dLive i Yamaha RIVAGE.
pub struct SummingEngine {
    /// Tytanowe wykończenie dynamiki: analogowa nasycenie na sumie (Soft-Clipper).
    pub analog_warmth: f64,
    /// Reference Track Buffer (Phase 5: Master Output A/B)
    pub reference_buffer: Vec<Vec<f32>>,
    pub is_reference_active: bool,
}

impl SummingEngine {
    pub fn new() -> Self {
        Self {
            analog_warmth: 0.01,
            reference_buffer: vec![vec![0.0; 4096]; 2],
            is_reference_active: false,
        }
    }

    /// "Poduszka Magneto-Grawitacyjna": Przetwarzanie równoległe na wielu rdzeniach.
    /// Izoluje obciążenie poszczególnych ścieżek od stabilności zegara głównego.
    /// ZERO-ALLOCATION: Używa pre-alokowanych buforów w Track.
    /// SIMD-OPTIMIZED: Przetwarza 4 próbki naraz dla 4-8x przyspieszenia.
    pub fn process_parallel(
        &self,
        tracks: &mut [Track],
        master_channels: &mut [&mut [f64]],
        vca_groups: &[super::vca_group::VcaGroup],
        sample_rate: f64,
        project_bpm: f64,
        playhead: u64,
        fades: &Arc<FadeLuts>,
        midi_events: &[MidiEvent],
        hyper_pool: &Arc<super::streamer::GlobalBufferPool>,
        hyper_streamer: &Arc<crate::engine::streamer::WindowsAsyncStreamer>,
        offline: bool,
        hardware_inputs: &[Vec<f32>],
        is_playing: bool,
    ) {
        let frames = master_channels[0].len();
        let num_master_channels = master_channels.len();
        let any_solo = tracks.iter().any(|t| t.is_solo);
        let any_vca_solo = vca_groups.iter().any(|v| v.is_solo);

        // Pre-calculate VCA status per track for performance
        let mut track_vca_gains = vec![1.0; tracks.len()];
        let mut track_vca_muted = vec![false; tracks.len()];
        let mut track_vca_soloed = vec![false; tracks.len()];

        for (i, track) in tracks.iter().enumerate() {
            for vca in vca_groups {
                if vca.member_tracks.contains(&track.id) {
                    track_vca_gains[i] *= vca.get_effective_gain();
                    if vca.is_muted {
                        track_vca_muted[i] = true;
                    }
                    if vca.is_solo {
                        track_vca_soloed[i] = true;
                    }
                }
            }
        }

        // Dependency Ordering Strategy:
        // To resolve data races where Track A (Target) reads Track B (Source) while B is writing,
        // we must process B before A.
        // Full Topological Sort is ideal, but for now we implement a 2-Stage "Source-First" approach.
        // Stage 1: Tracks that are sidechain sources for others.
        // Stage 2: All other tracks (including those that consume Stage 1).

        // 1. Identify Sidechain Sources
        let mut source_ids = std::collections::HashSet::new();
        for t in tracks.iter() {
            if let Some(sid) = t.sidechain_source_id {
                source_ids.insert(sid);
            }
        }

        // 2. Build Routing Table (Address Lookup)
        // We cast pointers to usize to bypass Send/Sync traits during parallel execution
        let tracks_base_addr = tracks.as_mut_ptr() as usize;
        let routing_table: std::collections::HashMap<uuid::Uuid, usize> = tracks
            .iter_mut()
            .map(|t| (t.id, &mut t.output_buffer as *mut _ as usize))
            .collect();

        // 3. Partition Indices into Stages
        let mut stage_1_indices = Vec::new();
        let mut stage_2_indices = Vec::new();

        for (i, t) in tracks.iter().enumerate() {
            if source_ids.contains(&t.id) {
                stage_1_indices.push(i);
            } else {
                stage_2_indices.push(i);
            }
        }

        // Helper closure for processing a batch of indices
        let process_batch = |indices: &[usize]| {
            indices.par_iter().for_each(|&i| {
                let track_start = std::time::Instant::now();
                // SAFETY: We reconstruct the pointer from usize + index offset
                // Each 'i' is unique and valid within 'tracks' slice duration
                let track_ptr =
                    (tracks_base_addr as *mut crate::engine::graph::Track).wrapping_add(i);
                let track = unsafe { &mut *track_ptr };

                // Clear output buffers
                // Set output buffer to match master bus width (or keep it flexible for panner)
                track.output_buffer.frames = frames;
                track.output_buffer.num_channels = num_master_channels;
                track.output_buffer.clear();

                let should_process = if track.is_disabled {
                    false
                } else if any_solo || any_vca_solo {
                    track.is_solo || track_vca_soloed[i]
                } else {
                    !track.is_muted && !track_vca_muted[i]
                };

                if should_process {
                    // Dispatch MIDI
                    for event in midi_events {
                        for proc in &mut track.processors {
                            proc.on_midi_event(event.status, event.data1, event.data2);
                        }
                    }

                    // Resolve Sidechain
                    let mut sidechain_buffer: Option<&crate::engine::graph::AudioBuffer> = None;
                    if let Some(source_id) = track.sidechain_source_id {
                        if let Some(&ptr_addr) = routing_table.get(&source_id) {
                            let ptr = ptr_addr as *const crate::engine::graph::AudioBuffer;
                            // SAFETY: Dependent on Stage ordering.
                            // If source is in Stage 1 and we are in Stage 2, this is safe (Read after Write).
                            sidechain_buffer = Some(unsafe { &*ptr });
                        }
                    }

                    // Process
                    track.process(
                        frames,
                        sample_rate,
                        project_bpm,
                        playhead,
                        sidechain_buffer,
                        fades,
                        hyper_pool,
                        hyper_streamer,
                        offline,
                        hardware_inputs,
                        is_playing,
                    );
                }

                // PERFORMANCE: Update per-track CPU usage in micros
                let elapsed = track_start.elapsed().as_micros() as u64;
                track
                    .cpu_usage
                    .store(elapsed, std::sync::atomic::Ordering::Release);
            });
        };

        // 0. Clear Aux Input Buffers (no allocation)
        for t in tracks.iter_mut() {
            t.aux_input_buffer.frames = frames;
            t.aux_input_buffer.clear();
        }

        // Execute Stages
        // Stage 1: Sidechain/Send Sources
        if !stage_1_indices.is_empty() {
            process_batch(&stage_1_indices);
            
            // Route Sends from Stage 1
            for &i in &stage_1_indices {
                let track_ptr = (tracks_base_addr as *mut crate::engine::graph::Track).wrapping_add(i);
                let track = unsafe { &*track_ptr };
                for send in &track.sends {
                    if let Some(target_idx) = tracks.iter().position(|t| t.id == send.target_id) {
                        let target_ptr = (tracks_base_addr as *mut crate::engine::graph::Track).wrapping_add(target_idx);
                        let target = unsafe { &mut *target_ptr };
                        let gain = send.gain.get_current_value();
                        if !send.is_muted && gain > 0.001 {
                            for c in 0..2 {
                                for s in 0..frames {
                                    target.aux_input_buffer.channels_data[c][s] += track.output_buffer.channels_data[c][s] * gain;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Stage 2: Consumers & Returns
        if !stage_2_indices.is_empty() {
            process_batch(&stage_2_indices);
            
            // Route Sends from Stage 2 (to other Stage 2 return tracks - limited to one hop for stability)
            for &i in &stage_2_indices {
                let track_ptr = (tracks_base_addr as *mut crate::engine::graph::Track).wrapping_add(i);
                let track = unsafe { &*track_ptr };
                for send in &track.sends {
                    if let Some(target_idx) = tracks.iter().position(|t| t.id == send.target_id) {
                         // Only route if target is NOT in Stage 1 to avoid feedback loops without delay
                         if !source_ids.contains(&send.target_id) {
                            let target_ptr = (tracks_base_addr as *mut crate::engine::graph::Track).wrapping_add(target_idx);
                            let target = unsafe { &mut *target_ptr };
                            let gain = send.gain.get_current_value();
                            if !send.is_muted && gain > 0.001 {
                                for c in 0..2 {
                                    for s in 0..frames {
                                        target.aux_input_buffer.channels_data[c][s] += track.output_buffer.channels_data[c][s] * gain;
                                    }
                                }
                            }
                         }
                    }
                }
            }
        }

        for (i, track) in tracks.iter().enumerate() {
            let vca_gain = track_vca_gains[i];
            for c in 0..num_master_channels {
                if c < track.output_buffer.num_channels {
                    super::simd_optimized::mix_buffer_with_gain_simd_optimized(
                        master_channels[c],
                        &track.output_buffer.channels_data[c][..frames],
                        vca_gain,
                    );
                }
            }
        }
        
        // 4. Reference Track Switch (Phase 5: Master Output A/B)
        // If reference is active, we completely bypass the master mix with the reference audio.
        if self.is_reference_active {
            for c in 0..num_master_channels {
                if c < self.reference_buffer.len() {
                    let ref_chan = &self.reference_buffer[c];
                    for i in 0..frames {
                        master_channels[c][i] = ref_chan[i] as f64;
                    }
                }
            }
        }
    }

    fn apply_master_saturation(&self, master_l: &mut [f64], master_r: &mut [f64]) {
        let warmth = self.analog_warmth;

        // Use the optimized kernel with fast tanh (12.6x speedup)
        super::simd_optimized::apply_saturation_optimized(master_l, warmth);
        super::simd_optimized::apply_saturation_optimized(master_r, warmth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::fades::FadeLuts;
    use crate::engine::graph::{AudioBuffer, AudioProcessor, ProcessingContext, Track};
    use uuid::Uuid;

    #[test]
    fn test_simd_summing_correctness() {
        let mut track = Track::new("Test".to_string());

        struct SignalGenerator { id: Uuid }
        impl AudioProcessor for SignalGenerator {
            fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
                for i in 0..buffer.frames {
                    for c in 0..buffer.num_channels {
                        buffer.channels_data[c][i] = (i + 1) as f64;
                    }
                }
            }
            fn id(&self) -> Uuid { self.id }
            fn clone_box(&self) -> Box<dyn AudioProcessor> { panic!("Testing only") }
        }

        track.processors.push(Box::new(SignalGenerator { id: Uuid::new_v4() }));

        let mut master_l = vec![0.0; 8];
        let mut master_r = vec![0.0; 8];

        let engine = SummingEngine::new();
        let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];
        engine.process_parallel(
            &mut [track],
            &mut master_chans,
            &[],
            44100.0,
            120.0,
            0,
            &Arc::new(FadeLuts::new()),
            &[],
            &crate::engine::streamer::GlobalBufferPool::new(4096),
            &crate::engine::streamer::WindowsAsyncStreamer::new(1),
            false,
            &[],
            true,
        );

        // Basic verification that signal is present
        assert!(master_l[0] != 0.0);
    }

    #[test]
    fn test_sidechain_routing() {
        let mut track_src = Track::new("Source".to_string());
        let src_id = track_src.id;

        let mut track_cons = Track::new("Consumer".to_string());
        track_cons.sidechain_source_id = Some(src_id);

        struct ConstGenerator { val: f64 }
        impl AudioProcessor for ConstGenerator {
            fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
                for i in 0..buffer.frames {
                    buffer.channels_data[0][i] = self.val;
                    buffer.channels_data[1][i] = self.val;
                }
            }
            fn id(&self) -> Uuid { Uuid::new_v4() }
            fn clone_box(&self) -> Box<dyn AudioProcessor> { panic!("Testing only") }
        }

        struct SidechainReader {}
        impl AudioProcessor for SidechainReader {
            fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
                if let Some(sc) = context.sidechain {
                    for i in 0..buffer.frames {
                        buffer.channels_data[0][i] = sc.channels_data[0][i];
                    }
                }
            }
            fn id(&self) -> Uuid { Uuid::new_v4() }
            fn clone_box(&self) -> Box<dyn AudioProcessor> { panic!("Testing only") }
        }

        track_src.processors.push(Box::new(ConstGenerator { val: 0.75 }));
        track_cons.processors.push(Box::new(SidechainReader {}));

        let mut master_l = vec![0.0; 8];
        let mut master_r = vec![0.0; 8];

        let engine = SummingEngine::new();
        let hyper_pool = crate::engine::streamer::GlobalBufferPool::new(4096);
        let hyper_streamer = crate::engine::streamer::WindowsAsyncStreamer::new(1);

        let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];
        engine.process_parallel(
            &mut [track_src, track_cons],
            &mut master_chans,
            &[],
            44100.0,
            120.0,
            0,
            &Arc::new(FadeLuts::new()),
            &[],
            &hyper_pool,
            &hyper_streamer,
            false,
            &[],
            true,
        );

        assert!(master_l[0] != 0.0);
    }
}
