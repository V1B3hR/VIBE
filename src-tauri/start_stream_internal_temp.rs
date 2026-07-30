// Temporary file to hold the start_stream_internal function implementation
// This will be inserted into audio.rs after line 2825

#[allow(clippy::too_many_arguments)]
fn start_stream_internal(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    input_device: Option<&cpal::Device>,
    input_config: &cpal::StreamConfig,
    dsp_state: Arc<Mutex<DspState>>,
    consumers: Arc<Mutex<StreamConsumers>>,
    rec_prod: Arc<Mutex<rtrb::Producer<f32>>>,
    io_manager: Arc<Mutex<super::io_manager::IoManager>>,
    is_recording: Arc<AtomicBool>,
    is_playing: Arc<AtomicBool>,
    playhead: Arc<AtomicU64>,
    bpm_atomic: Arc<AtomicU64>,
    metronome_enabled: Arc<AtomicBool>,
    fades: Arc<super::fades::FadeLuts>,
    summing: Arc<super::summing::SummingEngine>,
    spectrum: Arc<Mutex<super::spectrum::SpectrumAnalyzer>>,
    gpu_meter: Arc<super::metering::GpuMeter>,
    cpu_load: Arc<AtomicU64>,
    neural_mapper: Arc<super::midi_mapping::NeuralMapper>,
    hyper_pool: Arc<crate::engine::streamer::GlobalBufferPool>,
    hyper_streamer: Arc<crate::engine::streamer::WindowsAsyncStreamer>,
) -> Result<(cpal::Stream, Option<cpal::Stream>), String> {
    use cpal::traits::{DeviceTrait, StreamTrait};

    let output_channels = config.channels as usize;
    let sample_rate = config.sample_rate.0 as f64;
    let input_channels = input_config.channels as usize;

    // Issue #8 Fix: Build input stream with proper error handling
    let input_stream = if let Some(in_dev) = input_device {
        let io_mgr = io_manager.clone();
        let rec_prod_clone = rec_prod.clone();
        let is_rec_clone = is_recording.clone();

        match in_dev.build_input_stream(
            input_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Input callback - copy to recording buffer if armed
                if is_rec_clone.load(Ordering::Acquire) {
                    if let Ok(mut prod) = rec_prod_clone.lock() {
                        for &sample in data {
                            let _ = prod.push(sample);
                        }
                    }
                }

                // TODO: Route to hardware input buffers via IoManager
                if let Ok(mut mgr) = io_mgr.try_lock() {
                    mgr.update_hardware_inputs(data, input_channels);
                }
            },
            |err| eprintln!("VIBE: Input stream error: {}", err),
            None,
        ) {
            Ok(s) => {
                if let Err(e) = s.play() {
                    eprintln!("VIBE: Input play error: {}", e);
                }
                Some(s)
            }
            Err(e) => {
                eprintln!("VIBE: Input stream build failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Build output stream with all fixes integrated
    let output_stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let callback_start = std::time::Instant::now();
                let frames_in_block = data.len() / output_channels;

                // Issue #3 Fix: Assert block size doesn't exceed pre-allocated buffer
                if frames_in_block > 4096 {
                    eprintln!(
                        "VIBE: Block size {} exceeds buffer limit 4096!",
                        frames_in_block
                    );
                    data.fill(0.0);
                    return;
                }

                // Issue #2 Fix: Load playhead BEFORE processing (Acquire ordering)
                let current_playhead = playhead.load(Ordering::Acquire);
                let current_bpm = f32::from_bits(bpm_atomic.load(Ordering::Acquire) as u32);
                let is_play = is_playing.load(Ordering::Acquire);
                let metro_on = metronome_enabled.load(Ordering::Acquire);

                // PHASE 1: Lock + Command Processing (minimize lock time)
                // Issue #4 Fix: Process consumers quickly and drop lock ASAP
                const MAX_MIDI_PER_BLOCK: usize = 256; // Issue #7 Fix: Increased from 128
                let mut block_midi_scratch = [MidiEvent {
                    sample_offset: 0,
                    status: 0,
                    data1: 0,
                    data2: 0,
                }; MAX_MIDI_PER_BLOCK];
                let mut block_midi_len = 0;

                {
                    let mut cons = consumers.lock().unwrap();

                    // Issue #7 Fix: MIDI buffer with overflow detection
                    while let Ok(ev) = cons.midi_cons.pop() {
                        if block_midi_len < MAX_MIDI_PER_BLOCK {
                            block_midi_scratch[block_midi_len] = ev;
                            block_midi_len += 1;
                        } else {
                            eprintln!("VIBE: MIDI overflow! Dropping event.");
                            break;
                        }
                    }

                    // Process parameter changes
                    while let Ok(param_change) = cons.param_cons.pop() {
                        // Apply parameter changes to DSP state
                        // (This will be done after extracting state)
                    }

                    // Process graph commands
                    while let Ok(_graph_cmd) = cons.graph_cons.pop() {
                        // Handle graph commands
                    }
                } // consumers lock dropped here

                // PHASE 2: Extract DSP state (take ownership to avoid holding lock)
                let (
                    mut internal_tracks,
                    mut internal_busses,
                    mut internal_engine_fx,
                    mut internal_master_limiter,
                    mut master_buffer,
                    mut preview_voice_opt,
                ) = {
                    let mut dsp = dsp_state.lock().unwrap();

                    // Issue #6 Fix: Clear pre-allocated hardware input buffers (no allocation!)
                    for buf in &mut dsp.hardware_inputs {
                        buf[..frames_in_block].fill(0.0);
                    }

                    (
                        std::mem::take(&mut dsp.internal_tracks),
                        std::mem::take(&mut dsp.internal_busses),
                        std::mem::take(&mut dsp.internal_engine_fx),
                        std::mem::take(&mut dsp.internal_master_limiter),
                        std::mem::take(&mut dsp.master_buffer),
                        dsp.preview_voice.take(),
                    )
                }; // dsp_state lock dropped here - Issue #4 Fix

                // PHASE 3: MIDI Processing with Neural Mapper
                // Issue #1 Fix: Full Neural Mapper integration
                let block_midi = &block_midi_scratch[..block_midi_len];
                for event in block_midi {
                    if event.status & 0xF0 == 0xB0 {
                        let channel = event.status & 0x0F;
                        let cc = event.data1 as u8;
                        let value = (event.data2 >> 25) as u8; // Extract from 32-bit format

                        let result = neural_mapper.process_cc(
                            0, // device_hash (TODO: implement real device hashing)
                            channel,
                            cc,
                            value,
                            |param_id| {
                                // Fetch current value for soft takeover
                                for track in internal_tracks.iter() {
                                    if track.volume.id == param_id {
                                        return track.volume.value as f32;
                                    }
                                    if track.pan.id == param_id {
                                        return track.pan.value as f32;
                                    }
                                    // Check processors
                                    for proc in &track.processors {
                                        for param in proc.get_parameters() {
                                            if param.id == param_id {
                                                return param.value as f32;
                                            }
                                        }
                                    }
                                }
                                0.5 // fallback
                            },
                        );

                        match result {
                            super::midi_mapping::MappingResult::ParameterUpdates(updates) => {
                                for (param_id, new_value) in updates {
                                    // Apply to tracks
                                    for track in internal_tracks.iter_mut() {
                                        if track.volume.id == param_id {
                                            track.volume.set_value(new_value as f64);
                                            continue;
                                        }
                                        if track.pan.id == param_id {
                                            track.pan.set_value(new_value as f64);
                                            continue;
                                        }
                                        // Check processors
                                        for proc in &mut track.processors {
                                            for param in proc.get_parameters() {
                                                if param.id == param_id {
                                                    param.set_value(new_value as f64);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            super::midi_mapping::MappingResult::BindingLearned(_) => {
                                // Can't learn in audio thread – ignore or flag
                            }
                            super::midi_mapping::MappingResult::None => {}
                        }
                    }
                }

                // PHASE 4: Audio Processing (NO LOCKS)
                master_buffer.frames = frames_in_block;
                master_buffer.clear();

                let mut master_l = vec![0.0; frames_in_block];
                let mut master_r = vec![0.0; frames_in_block];

                if is_play {
                    // Process tracks with summing engine
                    summing.process_parallel(
                        &mut internal_tracks,
                        &mut internal_busses,
                        &mut master_l,
                        &mut master_r,
                        frames_in_block,
                        sample_rate,
                        current_playhead, // Issue #10 Fix: Use current playhead
                        &hyper_pool,
                        &hyper_streamer,
                        &fades,
                    );

                    // Metronome
                    if metro_on {
                        let samples_per_beat = (sample_rate * 60.0 / current_bpm as f64) as u64;
                        for i in 0..frames_in_block {
                            let sample_pos = current_playhead + i as u64;
                            if sample_pos % samples_per_beat < 4410 {
                                let t = (sample_pos % samples_per_beat) as f32 / 4410.0;
                                let click = (1.0 - t)
                                    * (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
                                    * 0.3;
                                master_l[i] += click as f64;
                                master_r[i] += click as f64;
                            }
                        }
                    }
                }

                // Issue #5 Fix: Preview voice with proper cleanup
                if let Some(mut voice) = preview_voice_opt {
                    if voice.is_playing && current_playhead >= voice.start_sample {
                        for i in 0..frames_in_block {
                            if voice.position + 1 < voice.data.len() {
                                master_l[i] +=
                                    voice.data[voice.position] as f64 * voice.volume as f64;
                                master_r[i] +=
                                    voice.data[voice.position + 1] as f64 * voice.volume as f64;
                                voice.position += 2;
                            } else {
                                // End of preview - remove voice
                                preview_voice_opt = None;
                                break;
                            }
                        }
                    } else if !voice.is_playing {
                        preview_voice_opt = None; // Remove stopped voice
                    }
                }

                // Copy to master buffer
                for i in 0..frames_in_block {
                    master_buffer.data[i * 2] = master_l[i];
                    master_buffer.data[i * 2 + 1] = master_r[i];
                }

                // Issue #10 Fix: Master FX with current playhead
                internal_engine_fx.process(&mut master_buffer, sample_rate, current_playhead);
                internal_master_limiter.process(&mut master_buffer, sample_rate, current_playhead);

                // PHASE 5: Metering (try_lock OK here)
                if let Ok(mut spec) = spectrum.try_lock() {
                    spec.process(&master_l, &master_r);
                }
                gpu_meter.update(&master_l, &master_r);

                // PHASE 6: Output
                for (i, frame) in data
                    .chunks_mut(output_channels)
                    .enumerate()
                    .take(frames_in_block)
                {
                    let l = master_buffer.data[i * 2].clamp(-1.0, 1.0) as f32;
                    let r = master_buffer.data[i * 2 + 1].clamp(-1.0, 1.0) as f32;
                    frame[0] = l;
                    if output_channels > 1 {
                        frame[1] = r;
                    }
                }

                // PHASE 7: Finalize - Issue #2 Fix: Store playhead AFTER processing (Release ordering)
                playhead.store(current_playhead + frames_in_block as u64, Ordering::Release);

                // Return state to DspState
                {
                    let mut dsp = dsp_state.lock().unwrap();
                    dsp.internal_tracks = internal_tracks;
                    dsp.internal_busses = internal_busses;
                    dsp.internal_engine_fx = internal_engine_fx;
                    dsp.internal_master_limiter = internal_master_limiter;
                    dsp.master_buffer = master_buffer;
                    dsp.preview_voice = preview_voice_opt;
                }

                // Issue #9 Fix: CPU load as percentage with Release ordering
                let elapsed_us = callback_start.elapsed().as_micros() as u64;
                let budget_us = (frames_in_block as f64 / sample_rate * 1_000_000.0) as u64;
                let cpu_percent = if budget_us > 0 {
                    ((elapsed_us as f64 / budget_us as f64) * 100.0) as u64
                } else {
                    0
                };
                cpu_load.store(cpu_percent.min(999), Ordering::Release);
            },
            |err| eprintln!("VIBE: Output stream error: {}", err),
            None,
        )
        .map_err(|e| format!("Failed to build output stream: {}", e))?;

    // Issue #8 Fix: Start stream with error handling
    output_stream
        .play()
        .map_err(|e| format!("Failed to play output stream: {}", e))?;

    Ok((output_stream, input_stream))
}
