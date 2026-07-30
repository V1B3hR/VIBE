use crate::engine::audio_preview::PreviewVoice;
use crate::engine::dynamics_module::Compressor;
use crate::engine::effects::{Delay, Reverb, Saturation};
use crate::engine::graph::{
    AudioBuffer, AudioClip, AudioClipInfo, AudioProcessor, Bus, ChordMarker,
    GainEffect, MidiClip, MidiNote, ProcessingContext, QuantizeDivision, Scale, Track, TrackInfo,
    MAX_CHANNELS,
};
use crate::engine::history::{HistoryManager, ProjectSnapshot as DagSnapshot};
use crate::engine::persistence::{
    ClipSnapshot, MidiClipSnapshot, MidiNoteSnapshot, PluginSnapshot,
    ProjectSnapshot as FileSnapshot, TrackSnapshot,
};
use crate::engine::psycho::PsychoacousticEngine;
use crate::engine::recovery::AutoSaveManager;
use crate::engine::convolution_reverb::ConvolutionReverb;
use crate::engine::multiband_dynamics::MultibandDynamics;
use crate::engine::spectral_gate::SpectralGate;
use crate::engine::stereo_imager::StereoImager;
use crate::engine::synth::VOneSynth;
use cpal::traits::{DeviceTrait, HostTrait};
use midir::{Ignore, MidiInput};
use crate::engine::audio_utils::decode_file_to_vec;
use crate::engine::transient_detection::TransientDetector;

use rtrb::{Producer, RingBuffer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
// use std::time::Instant;
use uuid::Uuid;

// Phase 1 refactoring: Re-export types from new modules for backward compatibility
pub use crate::engine::audio_commands::{AudioCommand, GraphCommand, MidiEvent, ParamChange};
pub use crate::engine::audio_types::{get_micros, DspState, StreamConsumers};

// Type definitions moved to audio_commands.rs and audio_types.rs (Phase 1 refactoring)

#[allow(dead_code)]
pub struct AudioEngine {
    command_tx: Sender<AudioCommand>,
    pub tracks: Arc<Mutex<Vec<Track>>>,
    busses: Arc<Mutex<Vec<Bus>>>,
    library: Arc<Mutex<Vec<AudioClip>>>,
    plugins: Arc<Mutex<Vec<super::scanner::PluginMetadata>>>,
    pub playhead: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
    is_recording: Arc<AtomicBool>,
    metronome_enabled: Arc<AtomicBool>,
    loop_enabled: Arc<AtomicBool>,
    loop_start: Arc<AtomicU64>,
    loop_end: Arc<AtomicU64>,
    bpm: Arc<Mutex<f32>>,
    bpm_atomic: Arc<AtomicU64>, // Atomic bits of f32 for lock-free audio thread
    pub global_swing: Arc<AtomicU64>,
    _midi_conn: Option<midir::MidiInputConnection<()>>,
    midi_map: Arc<Mutex<HashMap<u8, Uuid>>>,
    midi_cc_lsb_cache: Arc<Mutex<HashMap<u8, u8>>>,
    visualizer_prod: Arc<Mutex<rtrb::Producer<f32>>>,
    cpu_load_micros: Arc<AtomicU64>,
    history: Arc<Mutex<HistoryManager>>,
    fades: Arc<super::fades::FadeLuts>,
    velocity_engine: Arc<super::velocity::VelocityEngine>,
    engine_fx: Arc<Mutex<PsychoacousticEngine>>,
    master_limiter: Arc<Mutex<dyn AudioProcessor>>,
    // Maybach Summing Engine (Parallel Processing)
    // Maybach Summing Engine (Parallel Processing)
    summing_engine: Arc<super::summing::SummingEngine>,

    // Point 16: The Neural Mapper (Synapse System)
    neural_mapper: Arc<super::midi_mapping::NeuralMapper>,

    // POINT 10: Spectrum Analyzer (FFT)
    spectrum_analyzer: Arc<Mutex<super::spectrum::SpectrumAnalyzer>>,

    // POINT 9: GPU Metering (Atomic RMS/Peak)
    gpu_meter: Arc<super::metering::GpuMeter>,

    recorded_samples: Arc<Mutex<Vec<f32>>>,
    // Real-time producers covered by Mutex for management access
    midi_prod: Arc<Mutex<Producer<MidiEvent>>>,
    param_prod: Arc<Mutex<Producer<ParamChange>>>,
    graph_prod: Arc<Mutex<Producer<GraphCommand>>>,
    autosave_path: PathBuf,
    plugin_path: PathBuf,

    // Phase 1.3: IO Manager
    pub io_manager: Arc<Mutex<super::io_manager::IoManager>>,

    // Phase 3.10: Advanced Routing Matrix
    pub audio_graph: Arc<Mutex<crate::engine::audio_graph::AudioGraph>>,
    pub buffer_pool: Arc<Mutex<crate::engine::audio_graph::BufferPool>>,
    pub cached_execution_order: Arc<Mutex<Vec<petgraph::graph::NodeIndex>>>,
    pub graph_dirty: Arc<AtomicBool>,

    // Phase 4: HyperStream Engine
    pub hyper_streamer: Arc<crate::engine::streamer::WindowsAsyncStreamer>,
    pub hyper_pool: Arc<crate::engine::streamer::GlobalBufferPool>,
    pub initialization_error: Arc<Mutex<Option<String>>>,

    // Field for keeping track of running configuration
    pub current_config: Arc<Mutex<Option<super::audio_device::AudioDeviceConfig>>>,

    pub markers: Arc<Mutex<Vec<super::graph::Marker>>>,
    
    // Phase 4: Interaction & Expression
    pub scene_manager: Arc<Mutex<super::scene_manager::SceneManager>>,
    pub tempo_automation: Arc<Mutex<super::automation::AutomationCurve>>,
    pub global_quantization: Arc<Mutex<super::graph::QuantizeDivision>>,
    pub kropelka_brain: Arc<Mutex<super::kropelka_brain::KropelkaBrain>>,
    pub groove_pool: Arc<Mutex<super::groove_pool::GroovePool>>,
    pub humanization_engine: Arc<Mutex<super::humanization_engine::HumanizationEngine>>,

    // Phase 3.3 Anti-Gravity: Spectral Engine
    pub spectral_audio_tx: crossbeam_channel::Sender<Vec<f32>>,
    pub spectral_worker: super::spectral::SpectralWorker,
    pub mel_frame_rx: crossbeam_channel::Receiver<super::spectral::MelFrame>,

    // Phase 5.2: Video Sync
    pub video_manager: Arc<super::video_manager::VideoManager>,
    pub vca_groups: Arc<Mutex<Vec<crate::engine::vca_group::VcaGroup>>>,
    pub hardware_input_prods: Arc<Mutex<Vec<rtrb::Producer<f32>>>>,
    pub disk_writer: Arc<super::disk_writer::DiskWriter>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<AudioCommand>();
        let tracks = Arc::new(Mutex::new(Vec::<Track>::new()));
        let busses = Arc::new(Mutex::new(Vec::<Bus>::new()));
        let library = Arc::new(Mutex::new(Vec::<AudioClip>::new()));
        let plugins = Arc::new(Mutex::new(Vec::<super::scanner::PluginMetadata>::new()));
        let playhead = Arc::new(AtomicU64::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let is_recording = Arc::new(AtomicBool::new(false));
        let metronome_enabled = Arc::new(AtomicBool::new(false));
        let tempo_automation = Arc::new(Mutex::new(super::automation::AutomationCurve::new(120.0)));
        let global_quantization = Arc::new(Mutex::new(super::graph::QuantizeDivision::Sixteenth));
        let loop_enabled = Arc::new(AtomicBool::new(false));
        let loop_start = Arc::new(AtomicU64::new(0));
        let loop_end = Arc::new(AtomicU64::new(48000 * 4)); // Default 4 bars @ 120bpm 4/4
        let bpm = Arc::new(Mutex::new(120.0f32));
        let bpm_atomic = Arc::new(AtomicU64::new(120.0f32.to_bits() as u64));
        let global_swing = Arc::new(AtomicU64::new(0.0f32.to_bits() as u64));

        let midi_map = Arc::new(Mutex::new(HashMap::new()));
        let midi_cc_lsb_cache = Arc::new(Mutex::new(HashMap::new()));
        let cpu_load_micros = Arc::new(AtomicU64::new(0));
        let fades = Arc::new(super::fades::FadeLuts::new());
        let velocity_engine = Arc::new(super::velocity::VelocityEngine::new());
        let engine_fx = Arc::new(Mutex::new(PsychoacousticEngine::new()));
        let master_limiter = Arc::new(Mutex::new(super::effects::MasterSafetyLimiter::new()));
        let summing_engine = Arc::new(super::summing::SummingEngine::new());
        let neural_mapper = Arc::new(super::midi_mapping::NeuralMapper::new());

        // Managed State
        let recorded_samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let current_config = Arc::new(Mutex::new(None));
        let markers = Arc::new(Mutex::new(Vec::<super::graph::Marker>::new()));
        let video_manager = Arc::new(super::video_manager::VideoManager::new());
        let vca_groups = Arc::new(Mutex::new(Vec::new()));

        // POINT 10: Spectrum Analyzer (FFT) - Now Off-Threaded
        let spectrum_analyzer = Arc::new(Mutex::new(super::spectrum::SpectrumAnalyzer::new()));
        let (viz_prod_raw, mut viz_cons) = rtrb::RingBuffer::<f32>::new(16384);
        let viz_prod_shared = Arc::new(Mutex::new(viz_prod_raw));
        
        // Spawn Visualizer Background Thread
        let spec_thread = spectrum_analyzer.clone();
        thread::spawn(move || {
            let mut samples = vec![0.0f64; 4096];
            loop {
                let count = if let Ok(chunk) = viz_cons.read_chunk(4096) {
                    let (s1, s2): (&[f32], &[f32]) = chunk.as_slices();
                    let n1 = s1.len();
                    for (i, &s) in s1.iter().enumerate() {
                        samples[i] = s as f64;
                    }
                    for (i, &s) in s2.iter().enumerate() {
                        samples[n1 + i] = s as f64;
                    }
                    let total = n1 + s2.len();
                    chunk.commit_all();
                    total
                } else {
                    0
                };

                if count > 0 {
                    if let Ok(mut spec) = spec_thread.lock() {
                        spec.process(&samples[..count]);
                    }
                } else {
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        let _viz_prod_t = viz_prod_shared.clone();

        // POINT 9: GPU Metering
        let gpu_meter = Arc::new(super::metering::GpuMeter::new(44100));

        // Phase 3.10: Advanced Routing Matrix
        let audio_graph = Arc::new(Mutex::new(crate::engine::audio_graph::AudioGraph::new()));
        let buffer_pool = Arc::new(Mutex::new(crate::engine::audio_graph::BufferPool::new(
            crate::engine::graph::MAX_BUFFER_SIZE,
            64,
        )));
        let cached_execution_order = Arc::new(Mutex::new(Vec::new()));
        let graph_dirty = Arc::new(AtomicBool::new(false));

        // Phase 4: HyperStream Engine Initialization (1GB Pool)
        let hyper_pool = crate::engine::streamer::GlobalBufferPool::new(1024);
        let hyper_streamer = crate::engine::streamer::WindowsAsyncStreamer::new(8);
        println!("VIBE: HyperStreamer initialized.");

        let (midi_prod_raw, midi_cons) = RingBuffer::new(1024);
        let (param_prod_raw, param_cons) = RingBuffer::new(2048);
        let (graph_prod_raw, graph_cons) = RingBuffer::new(256);

        // Phase 5: Input Monitoring Modes (Auto/In/Off)
        let mut hardware_input_prods = Vec::with_capacity(64);
        let mut hardware_input_cons = Vec::with_capacity(64);
        for _ in 0..64 {
            let (prod, cons) = RingBuffer::<f32>::new(16384);
            hardware_input_prods.push(prod);
            hardware_input_cons.push(cons);
        }

        let autosave_path = std::env::temp_dir().join("vibe_autosave.json");
        let plugin_path = PathBuf::from("plugins");
        let initialization_error = Arc::new(Mutex::new(None));

        // IO Manager (Default 64 Channels)
        let io_manager = Arc::new(Mutex::new(super::io_manager::IoManager::new(64)));

        // Initial Snapshot for Analysis
        let kropelka_brain = Arc::new(Mutex::new(super::kropelka_brain::KropelkaBrain::new()));
        // Try to load knowledge base async or just let it fail silently and retry later
        // In a real app we'd handle this better, for now just print error
        if let Ok(mut brain) = kropelka_brain.lock() {
             if let Err(e) = brain.load_knowledge_base() {
                 println!("VIBE: Failed to load Kropelka Knowledge Base: {}", e);
             }
        }
        
        let groove_pool = Arc::new(Mutex::new(super::groove_pool::GroovePool::new()));
        let humanization_engine = Arc::new(Mutex::new(super::humanization_engine::HumanizationEngine::new()));
        let scene_manager = Arc::new(Mutex::new(super::scene_manager::SceneManager::new()));
        
        // Initialize channels early for ownership
        let (spectral_audio_tx_init, spectral_audio_rx) = crossbeam_channel::bounded::<Vec<f32>>(100);
        let (mel_frame_tx, mel_frame_rx_init) = crossbeam_channel::bounded::<super::spectral::MelFrame>(1000);
        
        let spectral_worker = super::spectral::SpectralWorker::new(super::spectral::MelSpectrogramConfig::default());
        let spectral_audio_tx = spectral_audio_tx_init.clone();
        let mel_frame_rx = mel_frame_rx_init.clone();
        
        // --- End of Initialization Logic ---
        
        let initial_snapshot = DagSnapshot {
            id: Uuid::new_v4(),
            timestamp: get_micros(),
            action_name: "Initial State".to_string(),
            tracks: Vec::new(),
            bpm: 120.0,
            parent_id: None,
        };
        let history = Arc::new(Mutex::new(HistoryManager::new(initial_snapshot)));

        // Start Crash Recovery System with binary .vibe format
        let autosave_vibe_path = autosave_path.with_extension("vibe-autosave");
        let autosave = AutoSaveManager::new(autosave_vibe_path.clone());

        // Clone Arc references for auto-save callback
        let tracks_autosave = Arc::clone(&tracks);
        let bpm_autosave = Arc::clone(&bpm);
        let neural_mapper_autosave = Arc::clone(&neural_mapper);
        let loop_enabled_autosave = Arc::clone(&loop_enabled);
        let loop_start_autosave = Arc::clone(&loop_start);
        let loop_end_autosave = Arc::clone(&loop_end);

        autosave.start(move || {
            // Create snapshot from current state
            use super::persistence::save_project;

            let mut tracks_lock = tracks_autosave.lock().unwrap();
            let bpm_lock = bpm_autosave.lock().unwrap();
            let bindings = neural_mapper_autosave.get_bindings();

            let snapshot = FileSnapshot {
                name: "AutoSave".to_string(),
                bpm: *bpm_lock as f64,
                sample_rate: 48000.0,
                master_volume: 1.0,
                master_pan: 0.0,
                input_aliases: vec![], // TODO: Get from IoManager
                midi_bindings: bindings,
                loop_enabled: loop_enabled_autosave.load(std::sync::atomic::Ordering::Relaxed),
                loop_start: loop_start_autosave.load(std::sync::atomic::Ordering::Relaxed),
                loop_end: loop_end_autosave.load(std::sync::atomic::Ordering::Relaxed),
                vca_groups: vec![], // TODO: Snapshot VCA groups
                tracks: tracks_lock
                    .iter_mut()
                    .map(|track| TrackSnapshot {
                        id: track.id.to_string(),
                        name: track.name.clone(),
                        volume: super::persistence::ParameterSnapshot {
                        id: track.volume.id.to_string(),
                        name: track.volume.name.clone(),
                        value: track.volume.get_current_value(),
                        automation: track.volume.curve.load().knots.clone(),
                    },
                    pan: super::persistence::ParameterSnapshot {
                        id: track.pan.id.to_string(),
                        name: track.pan.name.clone(),
                        value: track.pan.get_current_value(),
                        automation: track.pan.curve.load().knots.clone(),
                    },
                    width: super::persistence::ParameterSnapshot {
                        id: track.width.id.to_string(),
                        name: track.width.name.clone(),
                        value: track.width.get_current_value(),
                        automation: track.width.curve.load().knots.clone(),
                    },
                    input_drive: super::persistence::ParameterSnapshot {
                        id: track.input_drive.id.to_string(),
                        name: track.input_drive.name.clone(),
                        value: track.input_drive.get_current_value(),
                        automation: track.input_drive.curve.load().knots.clone(),
                    },
                    muted: track.is_muted,
                    solo: track.is_solo,
                    is_armed: track.is_armed,
                    phase_inverted: track.phase_inverted,
                        color: track.color.clone(),
                        clips: track
                            .clips
                            .iter()
                            .map(|clip| ClipSnapshot {
                                id: clip.id.to_string(),
                                audio_path: clip.path.clone().unwrap_or_default(),
                                start_sample: clip.start_sample,
                                duration_samples: clip.length_in_samples,
                                offset_in_data: clip.offset_in_data,
                                fade_in_len: clip.fade_in_len,
                                fade_out_len: clip.fade_out_len,
                                fade_in_type: clip.fade_in_type,
                                fade_out_type: clip.fade_out_type,
                            })
                            .collect(),
                        plugins: track
                            .processors
                            .iter_mut()
                            .map(|p| {
                                PluginSnapshot {
                                    id: p.id().to_string(),
                                    plugin_path: p.name(), // name acts as identifier for internal, path for VST
                                    state_blob: p.get_state(),
                                    parameters: p
                                        .get_parameters()
                                        .iter()
                                        .map(|param| super::persistence::ParameterSnapshot {
                                            id: param.id.to_string(),
                                            name: param.name.clone(),
                                            value: param.get_current_value(),
                                            automation: param.curve.load().knots.clone(),
                                        })
                                        .collect(),
                                }
                            })
                            .collect(),
                        input_alias_id: track.input_alias_id.map(|id| id.to_string()),
                        // MIDI Sequencer
                        midi_clips: track
                            .midi_clips
                            .iter()
                            .map(|midi_clip| MidiClipSnapshot {
                                id: midi_clip.id.to_string(),
                                name: midi_clip.name.clone(),
                                start_sample: midi_clip.start_sample,
                                length_samples: midi_clip.length_samples,
                                color: midi_clip.color.clone(),
                                is_muted: midi_clip.is_muted,
                                is_looped: midi_clip.is_looped,
                                scale: midi_clip.scale.clone(),
                                chord_markers: midi_clip.chord_markers.clone(),
                                groove_template: midi_clip.groove_template.clone(),
                                pattern_id: midi_clip.pattern_id.clone(),
                                tuning_steps: midi_clip.tuning_steps,
                                time_signature_num: midi_clip.time_signature_num,
                                time_signature_den: midi_clip.time_signature_den,
                                cc_events: midi_clip
                                    .cc_events
                                    .iter()
                                    .map(|cc| super::persistence::MidiCCSnapshot {
                                        sample: cc.sample,
                                        cc_number: cc.cc_number,
                                        value: cc.value,
                                        channel: cc.channel,
                                    })
                                    .collect(),
                                notes: midi_clip
                                    .notes
                                    .iter()
                                    .map(|note| MidiNoteSnapshot {
                                        start_sample: note.start_sample,
                                        length_samples: note.length_samples,
                                        note: note.note,
                                        velocity: note.velocity,
                                        channel: note.channel,
                                        pitch_bend: note.pitch_bend,
                                        pressure: note.pressure,
                                        timbre: note.timbre,
                                        probability: note.probability,
                                        velocity_random: note.velocity_random,
                                        timing_random: note.timing_random,
                                    })
                                    .collect(),
                            })
                            .collect(),
                        quantize_division: track.quantize_division,
                    })
                    .collect(),
            };

            save_project(&snapshot, &autosave_vibe_path)
        });

        // Phase 6: Disk Streaming (Record to Disk)
        let (disk_writer_raw, _disk_prod) = super::disk_writer::DiskWriter::new(1024 * 1024); // 1M samples buffer
        let disk_writer = Arc::new(disk_writer_raw);
        let hw_prods_management = Arc::new(Mutex::new(hardware_input_prods));

        // Producers covered by Mutex for multi-thread sharing in management
        let midi_prod = Arc::new(Mutex::new(midi_prod_raw));
        let param_prod = Arc::new(Mutex::new(param_prod_raw));
        let graph_prod = Arc::new(Mutex::new(graph_prod_raw));

        // Anti-Gravity: Spawn Spectral Worker now that midi_prod is ready
        spectral_worker.spawn(spectral_audio_rx, mel_frame_tx, Arc::clone(&midi_prod));
        println!("VIBE: Spectral Engine initialized (Antigravity Mode).");

        // Clones for the management thread (audio processing & command loop)
        let tracks_management = Arc::clone(&tracks);
        let groove_pool_management = Arc::clone(&groove_pool);
        let busses_management = Arc::clone(&busses);
        let library_management = Arc::clone(&library);
        let plugins_management = Arc::clone(&plugins);
        let playhead_shared = Arc::clone(&playhead);
        let is_playing_shared = Arc::clone(&is_playing);
        let is_recording_shared = Arc::clone(&is_recording);
        let metronome_enabled_shared = Arc::clone(&metronome_enabled);
        let bpm_shared = Arc::clone(&bpm_atomic);
        let bpm_management = Arc::clone(&bpm);
        let midi_map_management = Arc::clone(&midi_map);
        let _lsb_cache_management = Arc::clone(&midi_cc_lsb_cache);
        let cpu_load_shared = Arc::clone(&cpu_load_micros);
        let history_management = Arc::clone(&history);
        let _fades_management = Arc::clone(&fades);
        let _summing_shared = Arc::clone(&summing_engine);
        let _spectrum_shared = Arc::clone(&spectrum_analyzer);
        let gpu_meter_shared = Arc::clone(&gpu_meter);
        let neural_mapper_shared = Arc::clone(&neural_mapper);
        let loop_enabled_shared = Arc::clone(&loop_enabled);
        let loop_start_shared = Arc::clone(&loop_start);
        let loop_end_shared = Arc::clone(&loop_end);
        let _velocity_engine_shared = Arc::clone(&velocity_engine);
        let midi_prod_management = Arc::clone(&midi_prod);
        let param_prod_management = Arc::clone(&param_prod);
        let graph_prod_management = Arc::clone(&graph_prod);
        let engine_fx_management = Arc::clone(&engine_fx);
        let master_limiter_management = Arc::clone(&master_limiter);
        let plugin_path_management = plugin_path.clone();
        let markers_management = Arc::clone(&markers);
        let vca_groups_management = Arc::clone(&vca_groups);
        let disk_writer_management = Arc::clone(&disk_writer);

        let midi_prod_for_conn = Arc::clone(&midi_prod_management);
        let param_prod_for_conn = Arc::clone(&param_prod_management);

        // MIDI Initialization
        let mut midi_conn = None;
        let mut midi_input = MidiInput::new("VIBE MIDI Input").unwrap();
        midi_input.ignore(Ignore::None);
        let ports = midi_input.ports();
        if let Some(port) = ports.first() {
            let port_name = midi_input.port_name(port).unwrap();
            println!("VIBE: Detected MIDI Controller: {}", port_name);

            // MIDI Input Handler with 14-bit CC support and Lock-Free Queue
            // MIDI Input Handler with 14-bit CC support and Lock-Free Queue
            let neural_mapper_handler = Arc::clone(&neural_mapper_shared);
            let midi_prod_handler = Arc::clone(&midi_prod_for_conn);
            let param_prod_handler = Arc::clone(&param_prod_for_conn);

            let _conn = midi_input
                .connect(
                    port,
                    "vibe-control-port",
                    move |_stamp, message, _| {
                        if message.len() >= 3 {
                            let status = message[0] & 0xF0;
                            // CC Message (0xB0 - 0xBF)
                            if status == 0xB0 {
                                let channel = message[0] & 0x0F;
                                let cc = message[1];
                                let value = message[2];
                                
                                // Neural Mapper Processing
                                let result = neural_mapper_handler.process_cc(
                                    0, // TODO: Device Hash
                                    channel,
                                    cc,
                                    value,
                                    |_| 0.5 // TODO: Fetch real value for Soft Takeover
                                );

                                match result {
                                    crate::engine::midi_mapping::MappingResult::ParameterUpdates(updates) => {
                                        let mut prod = param_prod_handler.lock().unwrap();
                                        for (id, val) in updates {
                                            let _ = prod.push(ParamChange {
                                                id,
                                                value: val,
                                            });
                                        }
                                    },
                                    crate::engine::midi_mapping::MappingResult::BindingLearned(binding) => {
                                        println!("VIBE: Synapse Linked! Binding: {:?}", binding);
                                        neural_mapper_handler.add_binding(binding);
                                        // Auto-exit learn mode
                                        neural_mapper_handler.is_learning.store(false, Ordering::Release);
                                        *neural_mapper_handler.learning_target.lock().unwrap() = None;
                                    },
                                    crate::engine::midi_mapping::MappingResult::None => {
                                        // Pass through to generic MIDI queue if no binding found
                                         let _ = midi_prod_handler.lock().unwrap().push(MidiEvent {
                                            sample_offset: 0,
                                            status: 0xB0,
                                            data1: cc as u16,
                                            data2: (value as u32) << 25,
                                        });
                                    }
                                }
                            } else {
                                // Pass through non-CC messages (Notes, etc.)
                                let _ = midi_prod_handler.lock().unwrap().push(MidiEvent {
                                    sample_offset: 0,
                                    status,
                                    data1: message[1] as u16,
                                    data2: if message.len() > 2 { (message[2] as u32) << 25 } else { 0 },
                                });
                            }
                        }
                    },
                    (),
                )
                .ok();
            midi_conn = _conn;
        }

        let audio_graph_thread = Arc::clone(&audio_graph);
        let graph_dirty_thread = Arc::clone(&graph_dirty);

        let hyper_pool_thread = Arc::clone(&hyper_pool);
        let hyper_streamer_thread = Arc::clone(&hyper_streamer);

        let io_manager_handle = Arc::clone(&io_manager);
        let init_error = Arc::clone(&initialization_error); // Captured for thread

        // --- Phase 4: Persistence Init ---
        // Initialize Persistent DSP State
        let initial_internal_fx = engine_fx_management.lock().unwrap().clone_box();
        let initial_master_limiter = master_limiter_management.lock().unwrap().clone_box();

        let dsp_state = Arc::new(Mutex::new(DspState {
            internal_tracks: Vec::new(),
            internal_busses: Vec::new(),
            internal_vca_groups: Vec::new(),
            internal_engine_fx: Some(initial_internal_fx),
            internal_master_limiter: Some(initial_master_limiter),
            master_buffer: AudioBuffer::new(), // Issue #3 Fix: Pre-allocated buffer (MAX_BUFFER_SIZE)
            master_channels: vec![vec![0.0; 4096]; 16], // Support up to 16 channels (9.1.6)
            num_master_channels: 2, // Default to stereo, will be updated by device config
            monitor_input_buffer: vec![0.0; 4096],
            pdc_dirty: true,
            preview_voice: None,
            hardware_inputs: vec![vec![0.0; 4096]; 64], // Issue #6 Fix: Pre-allocated input buffers
            
            // Phase 4: Interaction & Expression
            mpe_handler: super::mpe_handler::MpeHandler::new(0, 1..=14), // Zone 1: Master Chan 1, Members 2-15
            macro_engine: super::macro_engine::MacroEngine::new(),
            clip_launcher: super::clip_launcher::ClipLauncher::new(),
        }));
        
        let consumers = Arc::new(Mutex::new(StreamConsumers {
            midi_cons,
            param_cons,
            graph_cons,
            hardware_input_cons,
        }));

        // Phase 5.3: Direct-to-Disk Recording (hound-based)
        let (disk_writer_raw, disk_writer_prod) = super::disk_writer::DiskWriter::new(1024 * 1024 * 8); // 8M samples (~160s @ 48k mono)
        let disk_writer = Arc::new(disk_writer_raw);
        let rec_prod_arc = Arc::new(Mutex::new(disk_writer_prod));
        
        #[allow(clippy::arc_with_non_send_sync)]
        let stream_arc = Arc::new(Mutex::new(None::<cpal::Stream>));
        #[allow(clippy::arc_with_non_send_sync)]
        let input_stream_arc = Arc::new(Mutex::new(None::<cpal::Stream>));

        // Clones for Management Thread
        let dsp_state_t = dsp_state.clone();
        let consumers_t = consumers.clone();
        let rec_prod_t = rec_prod_arc.clone();
        let _stream_t = stream_arc.clone();
        let _input_stream_t = input_stream_arc.clone();
        
        // Capture context arcs for start_stream_internal
        let io_manager_t = io_manager_handle.clone();
        let is_rec_t = is_recording_shared.clone();
        let is_play_t = is_playing_shared.clone();
        let playhead_t = playhead.clone();
        let bpm_atomic_t = bpm_atomic.clone();
        let global_swing_shared = global_swing.clone();
        let metro_t = metronome_enabled.clone();
        let fades_t = fades.clone();
        let summing_t = summing_engine.clone();
        let viz_prod_t = viz_prod_shared.clone();
        let gpu_t = gpu_meter_shared.clone(); // Was gpu_meter_t, renamed back to gpu_t
        let cpu_t = cpu_load_shared.clone();
        let neural_t = neural_mapper_shared.clone();
        let loop_en_t = loop_enabled_shared.clone();
        let loop_st_t = loop_start_shared.clone();
        let loop_ed_t = loop_end_shared.clone();
        let hw_prods_t = hw_prods_management.clone();
        let hw_prods_management = hw_prods_t.clone();

        // Shared State Cloning (for Stream Start from Logic Thread)
        let _stream_t = stream_arc.clone();
        let _input_stream_t = input_stream_arc.clone();

        let _consumers_t = consumers.clone();
        let hp_t = hyper_pool_thread.clone();
        let hs_t = hyper_streamer_thread.clone();
        let init_error_thread = init_error.clone();
        // let markers_management = markers.clone(); // Currently unused in loop but held for future marker events
        let audio_graph_t = audio_graph_thread.clone();
        let tempo_automation_t = tempo_automation.clone();
        let global_quantization_t = global_quantization.clone();
        let spectral_audio_tx_t = spectral_audio_tx.clone();
        let command_tx_t = tx.clone();


        // Use a custom 32MB stack to prevent stack overflow in the management/audio thread
        thread::Builder::new()
            .name("vibe-audio-mgmt".to_string())
            .stack_size(32 * 1024 * 1024) // 32 MB
            .spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                     let msg = "No output device found.".to_string();
                     *init_error_thread.lock().unwrap() = Some(msg);
                     while let Ok(_cmd) = rx.recv() { 
                         // Consume loop to prevent blocking if needed
                     }
                     return;
                }
            };

            let config: cpal::StreamConfig = match device.default_output_config() {
                Ok(c) => c.into(),
                Err(e) => {
                     let msg = format!("Config Error: {}", e);
                     *init_error_thread.lock().unwrap() = Some(msg);
                     return; 
                }
            };
            
            let sample_rate = config.sample_rate.0 as f64;
            let _output_channels = config.channels as usize;
            
            // Phase 4: Local Stream Management (Fixes !Send issue)
            let mut _active_stream: Option<cpal::Stream> = None;
            let mut _active_input_stream: Option<cpal::Stream> = None;
            
            let in_dev = host.default_input_device();
            let in_conf = in_dev.as_ref().map(|d| d.default_input_config().unwrap().into()).unwrap_or(config.clone());

            match Self::start_stream_internal(
                &device, &config, in_dev.as_ref(), &in_conf,
                dsp_state_t.clone(), consumers_t.clone(), rec_prod_t.clone(),
                io_manager_t.clone(), is_rec_t.clone(), is_play_t.clone(), playhead_t.clone(),
                bpm_atomic_t.clone(), metro_t.clone(), fades_t.clone(), summing_t.clone(),
                viz_prod_t.clone(),
                gpu_t.clone(), cpu_t.clone(), neural_t.clone(),
                loop_en_t.clone(), loop_st_t.clone(), loop_ed_t.clone(),
                hp_t.clone(), hs_t.clone(),
                spectral_audio_tx_t.clone(),
                hw_prods_t.clone()
            ) {
                Ok((s, is)) => {
                    _active_stream = Some(s);
                    _active_input_stream = is;
                }
                Err(e) => {
                    eprintln!("VIBE: Stream Start Failed: {}", e);
                    *init_error_thread.lock().unwrap() = Some(e);
                }
            }

            // Command Loop
            while let Ok(cmd) = rx.recv() {
                // Intercept SetAudioConfig
                if let AudioCommand::SetAudioConfig(new_conf) = &cmd {
                     println!("VIBE: Switching Audio Config to Host:{} Device:{} @{}Hz Buffer:{}", 
                        new_conf.host_name, new_conf.device_name, new_conf.sample_rate, new_conf.buffer_size);
                     _active_stream = None;
                     _active_input_stream = None;
                     
                     // Use the helper to find the physical device
                     if let Ok(d) = super::audio_device::AudioDeviceManager::find_device(&new_conf.host_name, &new_conf.device_name) {
                          let c_out = cpal::StreamConfig {
                              channels: new_conf.output_channels as u16,
                              sample_rate: cpal::SampleRate(new_conf.sample_rate),
                              buffer_size: cpal::BufferSize::Fixed(new_conf.buffer_size),
                          };
                          
                          // Optional: Find input device if it's separate or keep it if it's the same
                          let i_dev = Some(d.clone()); // Simplified for MVP: same device
                          let c_in = cpal::StreamConfig {
                              channels: new_conf.input_channels as u16,
                              sample_rate: cpal::SampleRate(new_conf.sample_rate),
                              buffer_size: cpal::BufferSize::Fixed(new_conf.buffer_size),
                          };

                          match Self::start_stream_internal(
                                &d, &c_out, i_dev.as_ref(), &c_in,
                                dsp_state_t.clone(), consumers_t.clone(), rec_prod_t.clone(),
                                io_manager_t.clone(), is_rec_t.clone(), is_play_t.clone(), playhead_t.clone(),
                                bpm_atomic_t.clone(), metro_t.clone(), fades_t.clone(), summing_t.clone(),
                                viz_prod_t.clone(),
                                gpu_t.clone(), cpu_t.clone(), neural_t.clone(),
                                loop_en_t.clone(), loop_st_t.clone(), loop_ed_t.clone(),
                                hp_t.clone(), hs_t.clone(),
                                spectral_audio_tx_t.clone(),
                                hw_prods_t.clone()
                            ) {
                                Ok((s, is)) => {
                                    _active_stream = Some(s);
                                    _active_input_stream = is;
                                    println!("VIBE: Audio Engine Successfully Reconfigured.");
                                }
                                Err(e) => eprintln!("VIBE: Reconfiguration Failed: {}", e),
                            }
                     } else {
                         eprintln!("VIBE: Could not find requested audio device.");
                     }
                }
                match cmd {
                    AudioCommand::SetParamSmooth(id, param, val, time) => {
                         println!("VIBE: SetParamSmooth {} {} {} {}", id, param, val, time); 
                    },
                    AudioCommand::AddWarpMarker(clip_id, sample, beats) => {
                         let mut tracks = tracks_management.lock().unwrap();
                         for track in tracks.iter_mut() {
                             if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                 clip.warp_markers.push(super::graph::WarpMarker {
                                     id: Uuid::new_v4(),
                                     original_pos_samples: sample,
                                     timeline_pos_beats: beats,
                                 });
                                 clip.warp_markers.sort_by_key(|m| m.original_pos_samples);
                                 println!("VIBE: Added Warp Marker to clip {} at {} samples / {} beats", clip_id, sample, beats);
                                 
                                 // Note: We don't necessarily need a GraphCommand for markers 
                                 // IF the audio thread reads the shared clips. 
                                 // But usually we should sync it.
                                 break;
                             }
                         }
                    },
                    AudioCommand::ApplyEQBand(node, band, freq, gain, q) => {
                         println!("VIBE: ApplyEQBand {} {} {} {} {}", node, band, freq, gain, q);
                    },
                    AudioCommand::ToggleBypass(node) => {
                         println!("VIBE: ToggleBypass {}", node);
                    },
                    AudioCommand::SuggestAndPreview(sugg_id, duration) => {
                         println!("VIBE: SuggestAndPreview {} {}", sugg_id, duration);
                    },
                    AudioCommand::Play => {
                        is_playing_shared.store(true, Ordering::Release);
                    }
                    AudioCommand::Pause => {
                        is_playing_shared.store(false, Ordering::Release);
                    }
                    AudioCommand::Stop => {
                        println!("VIBE: AudioCommand::Stop received");
                        is_playing_shared.store(false, Ordering::Release);
                        playhead_shared.store(0, Ordering::Release);
                        is_recording_shared.store(false, Ordering::Release);
                        gpu_t.reset_lufs(); // Reset LUFS meters when stopping
                        dsp_state_t.lock().unwrap().preview_voice = None;
                    }
                    AudioCommand::SetAudioConfig(config) => {
                        println!("VIBE: SetAudioConfig TODO: {:?}", config);
                    }
                    AudioCommand::DeleteInputAlias(_) => {
                        // TODO: Implement logic to clear input alias from tracks if needed
                        // For now we just acknowledge the command
                    }
                    AudioCommand::PreviewSampleSynced(data, quantize, _stretch, strength, swing) => {
                        // 1. Calculate Sync (Quantize)
                        let current_sample = playhead_shared.load(Ordering::Acquire);
                        let bpm = f32::from_bits(bpm_shared.load(Ordering::Relaxed) as u32);
                        let samples_per_beat = (sample_rate * 60.0 / bpm as f64) as u64;

                        // Determine trigger sample
                        let start_sample = if let Some(q) = quantize {
                            let interval = match q {
                                QuantizeDivision::Whole => samples_per_beat * 4,
                                QuantizeDivision::Half => samples_per_beat * 2,
                                QuantizeDivision::Quarter => samples_per_beat,
                                QuantizeDivision::Eighth => samples_per_beat / 2,
                                QuantizeDivision::Sixteenth => samples_per_beat / 4,
                                _ => samples_per_beat * 4, // Default to 1 Bar
                            };

                            let mut next_grid = ((current_sample + interval) / interval) * interval;
                            
                            // Apply Swing (only for 8th or 16th sub-divisions)
                            if (q == QuantizeDivision::Eighth || q == QuantizeDivision::Sixteenth) && swing > 0.0 {
                                let is_offbeat = (next_grid / interval) % 2 == 1;
                                if is_offbeat {
                                    let swing_offset = (interval as f32 * 0.33 * swing) as u64;
                                    next_grid += swing_offset;
                                }
                            }

                            // Apply Quantize Strength
                            // next_grid is 100% snap. raw click is current_sample.
                            let target = current_sample as f64 + (next_grid as f64 - current_sample as f64) * strength as f64;
                            target as u64
                        } else {
                            current_sample // Play immediately
                        };

                        println!(
                            "VIBE: Preview Queued @ {} (Curr: {}, Strength: {}, Swing: {})",
                            start_sample, current_sample, strength, swing
                        );

                        dsp_state_t.lock().unwrap().preview_voice = Some(PreviewVoice {
                            data,
                            position: 0,
                            start_sample,
                            volume: 0.8,
                            is_playing: true,
                        });
                    }
                    AudioCommand::PreviewSeek(pos_ratio) => {
                        if let Some(voice) = dsp_state_t.lock().unwrap().preview_voice.as_mut() {
                            if voice.data.len() >= 2 {
                                let total_frames = voice.data.len() / 2;
                                let target_frame = (total_frames as f32 * pos_ratio) as usize;
                                let target_idx = target_frame * 2;
                                voice.position = target_idx.min(voice.data.len() - 2);
                            }
                        }
                    }
                    AudioCommand::StopPreview => {
                        dsp_state_t.lock().unwrap().preview_voice = None;
                    }
                    AudioCommand::CreateInputAlias(name, stereo, chans, color) => {
                        let _ = io_manager_handle
                            .lock()
                            .unwrap()
                            .create_input_alias(name, stereo, chans, color);
                    }
                    AudioCommand::SetTrackInput(idx, alias_str) => {
                        let mut resolved_channels = None;
                        let mut resolved_uuid = None;

                        if let Some(id_str) = &alias_str {
                            if let Ok(uuid) = Uuid::parse_str(id_str) {
                                resolved_uuid = Some(uuid);
                                if let Ok(guard) = io_manager_handle.lock() {
                                    if let Some(alias) = guard.get_input_alias(uuid) {
                                        resolved_channels = Some(alias.hardware_channels.clone());
                                    }
                                }
                            }
                        }

                        // Update Management Track
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(idx) {
                            track.input_alias_id = resolved_uuid;
                            track.input_channels = resolved_channels.clone();
                        }

                        // Send to Audio Thread
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackInput(idx, resolved_uuid, resolved_channels));
                    }
                    AudioCommand::SetTrackSidechain(idx, source_id_str) => {
                        let mut resolved_uuid = None;
                        if let Some(id_str) = &source_id_str {
                            if let Ok(uuid) = Uuid::parse_str(id_str) {
                                resolved_uuid = Some(uuid);
                            }
                        }

                        // Update Management Track
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(idx) {
                            track.sidechain_source_id = resolved_uuid;
                        }

                        // Send to Audio Thread
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackSidechain(idx, resolved_uuid));
                    }
                    AudioCommand::ToggleRecord => {
                        let current_rec = is_recording_shared.load(Ordering::Acquire);
                        let next_rec = !current_rec;
                        is_recording_shared.store(next_rec, Ordering::Release);
                        
                        if next_rec {
                            let timestamp = get_micros();
                            let path = std::env::temp_dir().join(format!("vibe_recording_{}.wav", timestamp));
                            disk_writer_management.start_recording(path, 48000, 2); // Default to stereo 48k
                        } else {
                            disk_writer_management.stop_recording();
                        }

                        println!(
                            "VIBE: Recording {}",
                            if next_rec { "ON" } else { "OFF" }
                        );
                    }
                    AudioCommand::StartMidiLearn(param_id) => {
                        println!("VIBE: Entering Synapse Learning Mode for {}", param_id);
                        neural_mapper_shared
                            .is_learning
                            .store(true, Ordering::Release);
                        *neural_mapper_shared.learning_target.lock().unwrap() = Some(param_id);
                    }
                    AudioCommand::AddBinding(binding) => {
                        neural_mapper_shared.add_binding(binding);
                        neural_mapper_shared
                            .is_learning
                            .store(false, Ordering::Release);
                        *neural_mapper_shared.learning_target.lock().unwrap() = None;
                    }
                    AudioCommand::RemoveBinding(binding_id) => {
                        neural_mapper_shared.remove_binding(binding_id);
                    }
                    AudioCommand::AddTrack(track) => {
                        // Update management list (DTO-like for History)
                        tracks_management
                            .lock()
                            .unwrap()
                            .push(track.clone_as_dummy());
                        // Send to Audio Thread
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::AddTrack(track)); // Moves the track
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }

                    AudioCommand::SetTrackMute(index, muted) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_muted = muted;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackMute(index, muted));
                    }
                    AudioCommand::SetTrackSolo(index, solo) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_solo = solo;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackSolo(index, solo));
                    }
                    AudioCommand::SetTrackFrozen(index, frozen) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_frozen = frozen;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackFrozen(index, frozen));
                    }
                    AudioCommand::SetTrackDisabled(index, disabled) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_disabled = disabled;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackDisabled(index, disabled));
                    }
                    AudioCommand::SetTrackAutomationMode(index, mode) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.automation_mode = mode;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackAutomationMode(index, mode));
                    }
                    AudioCommand::SetTrackPan(index, val) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.pan.set_value(val);
                            // Check for Automation Recording
                            if is_recording_shared.load(Ordering::Acquire)
                                && is_playing_shared.load(Ordering::Acquire)
                            {
                                let pos = playhead_shared.load(Ordering::Acquire);
                                track.pan.record_value(pos, val);
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackPan(index, val));
                    }
                    AudioCommand::SetTrackWidth(index, val) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.width.set_value(val);
                            // Check for Automation Recording
                            if is_recording_shared.load(Ordering::Acquire)
                                && is_playing_shared.load(Ordering::Acquire)
                            {
                                let pos = playhead_shared.load(Ordering::Acquire);
                                track.width.record_value(pos, val);
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackWidth(index, val));
                    }
                    AudioCommand::SetTrackPhaseInvert(index, inverted) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.phase_inverted = inverted;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackPhaseInvert(index, inverted));
                    }
                    AudioCommand::SetTrackDrive(index, drive) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.input_drive.value = drive;
                            // Automation Recording
                            if is_recording_shared.load(Ordering::Acquire)
                                && is_playing_shared.load(Ordering::Acquire)
                            {
                                let pos = playhead_shared.load(Ordering::Acquire);
                                track.input_drive.record_value(pos, drive);
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackDrive(index, drive));
                    }
                    AudioCommand::SetTrackArm(index, armed) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_armed = armed;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackArm(index, armed));
                    }
                    AudioCommand::SetAudioClipWarpMode(track_idx, clip_id, mode) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(track_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                clip.warp_mode = mode;
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetAudioClipWarpMode(track_idx, clip_id, mode));
                    }
                    AudioCommand::SetTrackType(index, t_type) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.track_type = t_type;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackType(index, t_type));
                        Self::commit_history(&history_management, &tracks_management, &bpm_management);
                    }
                    AudioCommand::SetTrackParent(index, parent_id_str) => {
                        let parent_uuid = parent_id_str.and_then(|id| Uuid::parse_str(&id).ok());
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.parent_id = parent_uuid;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackParent(index, parent_uuid));
                        Self::commit_history(&history_management, &tracks_management, &bpm_management);
                    }
                    AudioCommand::SetTrackCollapsed(index, collapsed) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_collapsed = collapsed;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackCollapsed(index, collapsed));
                    }
                    AudioCommand::SetTrackHeight(index, height) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.height = height;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackHeight(index, height));
                    }
                    AudioCommand::SetTrackAutomationArmed(index, armed) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.is_automation_armed = armed;
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackAutomationArmed(index, armed));
                    }
                    AudioCommand::SetTrackColor(index, color) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.color = color.clone();
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetTrackColor(index, color));
                    }
                    AudioCommand::SetAudioClipPitch(t_idx, c_id, pitch) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                clip.pitch_semitones = pitch;
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetAudioClipPitch(t_idx, c_id, pitch));
                    }
                    AudioCommand::SetAudioClipWarp(t_idx, c_id, warped, speed) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                clip.is_warped = warped;
                                clip.playback_speed = speed;
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetAudioClipWarp(t_idx, c_id, warped, speed));
                    }
                    AudioCommand::SetAudioClipGain(t_idx, c_id, gain) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                clip.gain = gain;
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::SetAudioClipGain(t_idx, c_id, gain));
                    }
                    AudioCommand::HumanizeMidiClip(t_idx, c_id, timing, velocity) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.midi_clips.iter_mut().find(|c| c.id == c_id) {
                                // Logic handled in graph.rs, but we apply to management for persistence
                                clip.humanize(timing, velocity, sample_rate);
                            }
                        }
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::HumanizeMidiClip(t_idx, c_id, timing, velocity));
                    }
                    AudioCommand::ReEnableAutomation(param_id) => {
                        // For V1, automation is always active if points exist.
                        // "Re-enable" would mean clearing any manual override that isn't persistent yet.
                        println!("VIBE: Re-enable automation for {}", param_id);
                    }
                    AudioCommand::SetTrackOutput(index, target) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.output_target = target;
                        }
                    }
                    AudioCommand::SetTrackVolume(index, volume) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                            track.volume.value = volume;
                            // Check for Automation Recording
                            if is_recording_shared.load(Ordering::Acquire)
                                && is_playing_shared.load(Ordering::Acquire)
                                && track.is_automation_armed
                            {
                                let pos = playhead_shared.load(Ordering::Acquire);
                                track.volume.record_value(pos, volume);
                            }
                        }
                    }
                    AudioCommand::SetParameter(param_id, value) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        let recording = is_recording_shared.load(Ordering::Acquire);
                        let playing = is_playing_shared.load(Ordering::Acquire);
                        let pos = playhead_shared.load(Ordering::Acquire);

                        // 1. Check Global Engine FX
                        {
                            let mut fx = engine_fx_management.lock().unwrap();
                            for param in fx.get_parameters() {
                                if param.id == param_id {
                                    param.set_value(value);
                                    if recording && playing {
                                        param.record_value(pos, value);
                                    }
                                }
                            }
                        }

                        // 2. Check Master Limiter
                        {
                            let mut lim = master_limiter_management.lock().unwrap();
                            for param in lim.get_parameters() {
                                if param.id == param_id {
                                    param.set_value(value);
                                    if recording && playing {
                                        param.record_value(pos, value);
                                    }
                                }
                            }
                        }

                        // 3. Check Tracks
                        for track in track_list.iter_mut() {
                            // Track Level Parameters
                            let track_params = vec![
                                &mut track.volume,
                                &mut track.pan,
                                &mut track.width,
                                &mut track.eq_pre_dynamics,
                            ];

                            let mut found = false;
                            for p in track_params {
                                if p.id == param_id {
                                    p.set_value(value);
                                    if recording && playing && track.is_automation_armed {
                                        p.record_value(pos, value);
                                    }
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            // Console FX
                            for p in track.equalizer.get_parameters() {
                                if p.id == param_id {
                                    p.set_value(value);
                                    if recording && playing && track.is_automation_armed {
                                        p.record_value(pos, value);
                                    }
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for p in track.compressor.get_parameters() {
                                if p.id == param_id {
                                    p.set_value(value);
                                    if recording && playing && track.is_automation_armed {
                                        p.record_value(pos, value);
                                    }
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            // Inserts
                            for processor in &mut track.processors {
                                for param in processor.get_parameters() {
                                    if param.id == param_id {
                                        param.set_value(value);
                                        if recording && playing && track.is_automation_armed {
                                            param.record_value(pos, value);
                                        }
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    break;
                                }
                            }
                            if found {
                                break;
                            }
                        }
                    }
                    AudioCommand::AddAutomationPoint(param_id, pos, value) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        for track in track_list.iter_mut() {
                            let track_params = vec![
                                &mut track.volume,
                                &mut track.pan,
                                &mut track.width,
                                &mut track.eq_pre_dynamics,
                            ];

                            let mut found = false;
                            for p in track_params {
                                if p.id == param_id {
                                    p.add_knot(pos, value);
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for p in track.equalizer.get_parameters() {
                                if p.id == param_id {
                                    p.add_knot(pos, value);
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for p in track.compressor.get_parameters() {
                                if p.id == param_id {
                                    p.add_knot(pos, value);
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for processor in &mut track.processors {
                                for param in processor.get_parameters() {
                                    if param.id == param_id {
                                        param.add_knot(pos, value);
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    break;
                                }
                            }
                            if found {
                                break;
                            }
                        }
                    }
                    AudioCommand::SetAutomationTension(param_id, pos, tension) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        for track in track_list.iter_mut() {
                            let track_params = vec![
                                &mut track.volume,
                                &mut track.pan,
                                &mut track.width,
                                &mut track.eq_pre_dynamics,
                            ];

                            let mut found = false;
                            for p in track_params {
                                if p.id == param_id {
                                    p.set_automation_tension(pos, tension);
                                    p.set_automation_interpolation(crate::engine::automation::InterpolationType::Bezier);
                                    found = true;
                                    break;
                                }
                            }
                            if found { break; }

                            for p in track.equalizer.get_parameters() {
                                if p.id == param_id {
                                    p.set_automation_tension(pos, tension);
                                    p.set_automation_interpolation(crate::engine::automation::InterpolationType::Bezier);
                                    found = true;
                                    break;
                                }
                            }
                            if found { break; }

                            for p in track.compressor.get_parameters() {
                                if p.id == param_id {
                                    p.set_automation_tension(pos, tension);
                                    p.set_automation_interpolation(crate::engine::automation::InterpolationType::Bezier);
                                    found = true;
                                    break;
                                }
                            }
                            if found { break; }

                            for processor in &mut track.processors {
                                for param in processor.get_parameters() {
                                    if param.id == param_id {
                                        param.set_automation_tension(pos, tension);
                                        param.set_automation_interpolation(crate::engine::automation::InterpolationType::Bezier);
                                        found = true;
                                        break;
                                    }
                                }
                                if found { break; }
                            }
                            if found { break; }
                        }
                    }
                    AudioCommand::ClearAutomation(param_id) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        for track in track_list.iter_mut() {
                            let track_params = vec![
                                &mut track.volume,
                                &mut track.pan,
                                &mut track.width,
                                &mut track.eq_pre_dynamics,
                            ];

                            let mut found = false;
                            for p in track_params {
                                if p.id == param_id {
                                    p.clear_automation();
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for p in track.equalizer.get_parameters() {
                                if p.id == param_id {
                                    p.clear_automation();
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for p in track.compressor.get_parameters() {
                                if p.id == param_id {
                                    p.clear_automation();
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }

                            for processor in &mut track.processors {
                                for param in processor.get_parameters() {
                                    if param.id == param_id {
                                        param.clear_automation();
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    break;
                                }
                            }
                            if found {
                                break;
                            }
                        }
                    }
                    AudioCommand::SetAutomationInterpolation(param_id, interp_type) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        // Search in Tracks
                        'search: for track in track_list.iter_mut() {
                            if track.volume.id == param_id {
                                // Need to access Arc<ArcSwap<AutomationCurve>>
                                // Load, clone, modify, store.
                                let mut curve = (**track.volume.curve.load()).clone();
                                curve.interpolation = interp_type;
                                track.volume.curve.store(Arc::new(curve));
                                break 'search;
                            }
                            for processor in &mut track.processors {
                                for param in processor.get_parameters() {
                                    if param.id == param_id {
                                        let mut curve = (**param.curve.load()).clone();
                                        curve.interpolation = interp_type;
                                        param.curve.store(Arc::new(curve));
                                        break 'search;
                                    }
                                }
                            }
                        }
                        // Search Busses logic would go here
                    }
                    AudioCommand::SetAutomationLayer(param_id, layer) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        // Search in Tracks
                        'search: for track in track_list.iter_mut() {
                            if track.volume.id == param_id {
                                let mut curve = (**track.volume.curve.load()).clone();
                                if let Some(existing) =
                                    curve.layers.iter_mut().find(|l| l.id == layer.id)
                                {
                                    *existing = layer;
                                } else {
                                    curve.layers.push(layer);
                                }
                                track.volume.curve.store(Arc::new(curve));
                                break 'search;
                            }
                            for processor in &mut track.processors {
                                for param in processor.get_parameters() {
                                    if param.id == param_id {
                                        let mut curve = (**param.curve.load()).clone();
                                        if let Some(existing) =
                                            curve.layers.iter_mut().find(|l| l.id == layer.id)
                                        {
                                            *existing = layer;
                                        } else {
                                            curve.layers.push(layer);
                                        }
                                        param.curve.store(Arc::new(curve));
                                        break 'search;
                                    }
                                }
                            }
                        }
                    }
                    AudioCommand::ImportToLibrary(path) => {
                        if let Ok(clip) = load_audio_file(path.clone(), 48000.0) {
                            let clip_id = clip.id;
                            let library_clone = library_management.clone();

                            // Push to library immediately (LOD0/Head data only)
                            library_management.lock().unwrap().push(clip);

                            // Background Pyramid Generation
                            std::thread::Builder::new()
                                .name("waveform-gen".to_string())
                                .spawn(move || {
                                    match crate::engine::waveform::PyramidCache::load_cache(&path) {
                                        Ok(cache) => {
                                            if let Ok(mut lib) = library_clone.lock() {
                                                if let Some(c) =
                                                    lib.iter_mut().find(|c| c.id == clip_id)
                                                {
                                                    c.waveform_cache = Some(Arc::new(cache));
                                                    println!(
                                                        "VIBE: Waveform pyramid generated for {}",
                                                        c.name
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            println!(
                                                "VIBE: Failed to generate waveform pyramid: {}",
                                                e
                                            );
                                        }
                                    }
                                })
                                .unwrap();
                        }
                    }
                    AudioCommand::SetClipFades(track_idx, clip_id, in_len, out_len) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(track_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                clip.fade_in_len = in_len;
                                clip.fade_out_len = out_len;
                            }
                        }
                    }
                    AudioCommand::AddClipToTrack(track_index, clip_id, start_pos) => {
                        let lib = library_management.lock().unwrap();
                        if let Some(template) = lib.iter().find(|c| c.id == clip_id) {
                            let mut track_list = tracks_management.lock().unwrap();
                            if let Some(track) = track_list.get_mut(track_index) {
                                let mut new_clip = template.clone();
                                new_clip.id = Uuid::new_v4();
                                new_clip.start_sample = start_pos;
                                track.clips.push(new_clip);
                            }
                        }
                    }
                    AudioCommand::DeleteClip(track_index, clip_id) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(track_index) {
                            if let Some(pos) = track.clips.iter().position(|c| c.id == clip_id) {
                                track.clips.remove(pos);
                            }
                        }
                    }
                    AudioCommand::SliceClip(track_index, clip_instance_id, slice_sample_pos) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(track_index) {
                            track.slice_clip(clip_instance_id, slice_sample_pos);
                        }
                    }
                    AudioCommand::MoveClip(src_idx, clip_id, dest_idx, new_pos) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        let mut clip_to_move = None;

                        if src_idx < track_list.len() {
                            if let Some(pos) = track_list[src_idx]
                                .clips
                                .iter()
                                .position(|c| c.id == clip_id)
                            {
                                clip_to_move = Some(track_list[src_idx].clips.remove(pos));
                            }
                        }

                        if let Some(mut clip) = clip_to_move {
                            if dest_idx < track_list.len() {
                                clip.start_sample = new_pos;
                                track_list[dest_idx].clips.push(clip);
                                println!(
                                    "VIBE: Moved audio clip {} to track {} at {}",
                                    clip_id, dest_idx, new_pos
                                );
                            }
                        } else {
                            // Try MIDI
                            let mut midi_clip_to_move = None;
                            if src_idx < track_list.len() {
                                if let Some(pos) = track_list[src_idx]
                                    .midi_clips
                                    .iter()
                                    .position(|c| c.id == clip_id)
                                {
                                    midi_clip_to_move =
                                        Some(track_list[src_idx].midi_clips.remove(pos));
                                }
                            }
                            if let Some(mut clip) = midi_clip_to_move {
                                if dest_idx < track_list.len() {
                                    clip.start_sample = new_pos;
                                    track_list[dest_idx].midi_clips.push(clip);
                                    println!(
                                        "VIBE: Moved MIDI clip {} to track {} at {}",
                                        clip_id, dest_idx, new_pos
                                    );
                                }
                            }
                        }
                    }
                    AudioCommand::ResizeClip(
                        track_idx,
                        clip_id,
                        new_start,
                        new_offset,
                        new_len,
                    ) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(track_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                clip.start_sample = new_start;
                                clip.offset_in_data = new_offset;
                                clip.length_in_samples = new_len;
                            } else if let Some(clip) =
                                track.midi_clips.iter_mut().find(|c| c.id == clip_id)
                            {
                                clip.start_sample = new_start;
                                clip.length_samples = new_len;
                            }
                        }
                    }
                    AudioCommand::InsertSilence(pos, len) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        for track in track_list.iter_mut() {
                            for clip in track.clips.iter_mut() {
                                if clip.start_sample >= pos {
                                    clip.start_sample += len;
                                }
                            }
                            for clip in track.midi_clips.iter_mut() {
                                if clip.start_sample >= pos {
                                    clip.start_sample += len;
                                }
                            }
                        }
                        let mut markers_lock = markers_management.lock().unwrap();
                        for marker in markers_lock.iter_mut() {
                            if marker.position >= pos {
                                marker.position += len;
                            }
                        }
                    }
                    AudioCommand::DeleteTime(pos, len) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        for track in track_list.iter_mut() {
                            track.clips.retain_mut(|clip| {
                                if clip.start_sample + clip.length_in_samples <= pos {
                                    true
                                } else if clip.start_sample >= pos + len {
                                    clip.start_sample -= len;
                                    true
                                } else {
                                    false
                                }
                            });
                            track.midi_clips.retain_mut(|clip| {
                                if clip.start_sample + clip.length_samples <= pos {
                                    true
                                } else if clip.start_sample >= pos + len {
                                    clip.start_sample -= len;
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                        let mut markers_lock = markers_management.lock().unwrap();
                        markers_lock.retain_mut(|marker| {
                            if marker.position < pos {
                                true
                            } else if marker.position >= pos + len {
                                marker.position -= len;
                                true
                            } else {
                                false
                            }
                        });
                    }
                    AudioCommand::DuplicateTime(pos, len) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        for track in track_list.iter_mut() {
                            let mut audio_clones = Vec::new();
                            for clip in &track.clips {
                                if clip.start_sample >= pos && clip.start_sample < pos + len {
                                    let mut c = clip.clone();
                                    c.id = Uuid::new_v4();
                                    c.start_sample += len;
                                    audio_clones.push(c);
                                }
                            }
                            let mut midi_clones = Vec::new();
                            for clip in &track.midi_clips {
                                if clip.start_sample >= pos && clip.start_sample < pos + len {
                                    let mut c = clip.clone();
                                    c.id = Uuid::new_v4();
                                    c.start_sample += len;
                                    midi_clones.push(c);
                                }
                            }
                            for clip in track.clips.iter_mut() {
                                if clip.start_sample >= pos + len {
                                    clip.start_sample += len;
                                }
                            }
                            for clip in track.midi_clips.iter_mut() {
                                if clip.start_sample >= pos + len {
                                    clip.start_sample += len;
                                }
                            }
                            track.clips.extend(audio_clones);
                            track.midi_clips.extend(midi_clones);
                        }
                    }
                    AudioCommand::AddMarker(label, pos, color) => {
                        let mut markers_lock = markers_management.lock().unwrap();
                        markers_lock.push(super::graph::Marker {
                            id: Uuid::new_v4(),
                            label,
                            position: pos,
                            color,
                        });
                    }
                    AudioCommand::RemoveMarker(id) => {
                        let mut markers_lock = markers_management.lock().unwrap();
                        markers_lock.retain(|m| m.id != id);
                    }
                    AudioCommand::RenameTrack(idx, name) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(idx) {
                            track.name = name;
                        }
                    }
                    AudioCommand::DuplicateTrack(idx) => {
                        let mut new_track_opt = None;
                        {
                            let mut track_list = tracks_management.lock().unwrap();
                            if let Some(track) = track_list.get(idx) {
                                let mut new_track = track.clone_as_dummy();
                                new_track.id = Uuid::new_v4();
                                new_track.name = format!("{} (Copy)", new_track.name);
                                track_list.push(new_track.clone_as_dummy());
                                new_track_opt = Some(new_track);
                            }
                        }
                        if let Some(t) = new_track_opt {
                            let _ = graph_prod_management.lock().unwrap().push(GraphCommand::AddTrack(t));
                        }
                    }
                    AudioCommand::RemoveTrack(idx) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        if idx < track_list.len() {
                            track_list.remove(idx);
                            let _ = graph_prod_management.lock().unwrap().push(GraphCommand::RemoveTrack(idx));
                        }
                    }
                    AudioCommand::MoveTrack(from, to) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        let len = track_list.len();
                        if from < len && to < len && from != to {
                            let track = track_list.remove(from);
                            track_list.insert(to, track);
                            let _ = graph_prod_management.lock().unwrap().push(GraphCommand::MoveTrack(from, to));
                        }
                    }
                    AudioCommand::RenameClip(t_idx, c_id, name) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                clip.name = name;
                            } else if let Some(clip) = track.midi_clips.iter_mut().find(|c| c.id == c_id) {
                                clip.name = name;
                            }
                        }
                    }
                    AudioCommand::SetClipColor(t_idx, c_id, color) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                clip.color = color;
                            } else if let Some(clip) = track.midi_clips.iter_mut().find(|c| c.id == c_id) {
                                clip.color = color;
                            }
                        }
                    }
                    AudioCommand::ReverseClip(t_idx, c_id) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(_clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                let _ = graph_prod_management.lock().unwrap().push(GraphCommand::ReverseAudioClip(t_idx, c_id));
                            }
                        }
                    }
                    AudioCommand::NormalizeClip(t_idx, c_id, target_db) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(_clip) = track.clips.iter_mut().find(|c| c.id == c_id) {
                                let _ = graph_prod_management.lock().unwrap().push(GraphCommand::NormalizeAudioClip(t_idx, c_id, target_db));
                            }
                        }
                    }
                    AudioCommand::ConsolidateClips(t_idx, clip_ids) => {
                        if clip_ids.is_empty() { return; }
                        
                        // 1. Find bounding range and snapshot
                        let mut start_sample = u64::MAX;
                        let mut end_sample = 0;
                        
                        let (track_snapshot, graph_snapshot) = {
                            let tracks = tracks_management.lock().unwrap();
                            let track = match tracks.get(t_idx) {
                                Some(t) => t,
                                None => return,
                            };
                            
                            // Calculate range and verify clips exist
                            for &id in &clip_ids {
                                if let Some(ac) = track.clips.iter().find(|c| c.id == id) {
                                    start_sample = start_sample.min(ac.start_sample);
                                    end_sample = end_sample.max(ac.start_sample + ac.length_in_samples);
                                } else if let Some(mc) = track.midi_clips.iter().find(|c| c.id == id) {
                                    start_sample = start_sample.min(mc.start_sample);
                                    end_sample = end_sample.max(mc.start_sample + mc.length_samples);
                                }
                            }
                            
                            if start_sample == u64::MAX { return; }
                            
                            let mut dummy_track = track.clone_for_audio_thread();
                            // Keep ONLY the clips to be consolidated
                            dummy_track.clips.retain(|c| clip_ids.contains(&c.id));
                            dummy_track.midi_clips.retain(|c| clip_ids.contains(&c.id));
                            // Shift clips to start at 0 relative to consolidation range
                            for c in &mut dummy_track.clips { c.start_sample -= start_sample; }
                            for c in &mut dummy_track.midi_clips { c.start_sample -= start_sample; }
                            
                            let graph = audio_graph_t.lock().unwrap().clone();
                            (vec![dummy_track], graph)
                        };

                        let mut path = std::env::temp_dir();
                        path.push(format!("VIBE_Consolidate_{}.wav", Uuid::new_v4()));
                        let path_str = path.to_string_lossy().into_owned();
                        
                        // 2. Render
                        let config = crate::engine::render_engine::RenderConfig {
                            output_path: PathBuf::from(&path_str),
                            format: crate::engine::render_engine::ExportFormat::Wav,
                            sample_rate: 48000,
                            bit_depth: crate::engine::render_engine::BitDepth::Integer24,
                            dithering: crate::engine::render_engine::DitherMode::None,
                            normalize_lufs: None,
                            range: crate::engine::render_engine::RenderRange::Selection(0, end_sample - start_sample),
                            stem_export: vec![],
                            dry_run: false,
                            mp3_bitrate: 320,
                        };

                        let (tx, _rx) = crossbeam_channel::unbounded();
                        let mut render_engine = crate::engine::render_engine::RenderEngine::new(
                            graph_snapshot,
                            track_snapshot,
                            hp_t.clone(),
                            hs_t.clone(),
                            fades_t.clone(),
                            config,
                            tx,
                        );
                        
                        render_engine.render();

                        // 3. Replace clips
                        if let Ok(mut new_clip) = load_audio_file(PathBuf::from(&path_str), 48000.0) {
                            new_clip.start_sample = start_sample;
                            new_clip.name = format!("Consolidated_{}", t_idx);
                            
                            let mut tracks = tracks_management.lock().unwrap();
                            let mut library = library_management.lock().unwrap();
                            
                            if let Some(track) = tracks.get_mut(t_idx) {
                                // Remove old clips
                                track.clips.retain(|c| !clip_ids.contains(&c.id));
                                track.midi_clips.retain(|c| !clip_ids.contains(&c.id));
                                // Add new clip
                                let new_clip_clone = new_clip.clone();
                                track.clips.push(new_clip_clone);
                                // Add to library
                                library.push(new_clip);
                            }
                        }
                    }
                    AudioCommand::SetCrossfade(t_idx, c_a, c_b, duration) => {
                        // Backend support for crossfades
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetCrossfade(t_idx, c_a, c_b, duration));
                    }
                    AudioCommand::TransposeMidiClip(t_idx, c_id, semitones) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.midi_clips.iter_mut().find(|c| c.id == c_id) {
                                for note in &mut clip.notes {
                                    note.note = (note.note as i32 + semitones).clamp(0, 127) as u16;
                                }
                            }
                        }
                    }
                    AudioCommand::LegatoMidiClip(t_idx, c_id) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.midi_clips.iter_mut().find(|c| c.id == c_id) {
                                clip.notes.sort_by_key(|n| n.start_sample);
                                for i in 0..clip.notes.len() {
                                    if i + 1 < clip.notes.len() {
                                        let next_start = clip.notes[i+1].start_sample;
                                        if clip.notes[i].start_sample + clip.notes[i].length_samples < next_start {
                                             clip.notes[i].length_samples = next_start - clip.notes[i].start_sample;
                                        }
                                    } else {
                                        // Extend last note to clip end if desired
                                        let clip_end = clip.length_samples;
                                        if clip.notes[i].start_sample < clip_end {
                                            clip.notes[i].length_samples = clip_end - clip.notes[i].start_sample;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AudioCommand::DuplicateMidiNotes(t_idx, c_id, note_indices) => {
                        if note_indices.is_empty() { return; }
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(t_idx) {
                            if let Some(clip) = track.midi_clips.iter_mut().find(|c| c.id == c_id) {
                                let mut new_notes = Vec::new();
                                let mut min_start = u64::MAX;
                                let mut max_end = 0;

                                for &idx in &note_indices {
                                    if let Some(note) = clip.notes.get(idx) {
                                        min_start = min_start.min(note.start_sample);
                                        max_end = max_end.max(note.start_sample + note.length_samples);
                                        new_notes.push(note.clone());
                                    }
                                }

                                if !new_notes.is_empty() {
                                    let offset = max_end - min_start;
                                    for note in &mut new_notes {
                                        note.start_sample += offset;
                                    }
                                    clip.notes.extend(new_notes);
                                }
                            }
                        }
                    }
                    AudioCommand::ConvertMidiToAudio(t_idx, c_id) => {
                        let mut path = std::env::temp_dir();
                        path.push(format!("VIBE_Bounce_{}.wav", Uuid::new_v4()));
                        let path_str = path.to_string_lossy().into_owned();
                        
                        // 1. Snapshot the track and graph
                        let (track_snapshot, graph_snapshot) = {
                            let tracks = tracks_management.lock().unwrap();
                            let mut dummy_track = match tracks.get(t_idx) {
                                Some(t) => t.clone_for_audio_thread(),
                                None => return,
                            };
                            // Keep ONLY the target MIDI clip
                            dummy_track.midi_clips.retain(|c| c.id == c_id);
                            dummy_track.clips.clear();
                            
                            let graph = audio_graph_t.lock().unwrap().clone();
                            (vec![dummy_track], graph)
                        };

                        if track_snapshot[0].midi_clips.is_empty() { return; }
                        let range_end = track_snapshot[0].midi_clips[0].length_samples;

                        // 2. Setup Render
                        let config = crate::engine::render_engine::RenderConfig {
                            output_path: PathBuf::from(&path_str),
                            format: crate::engine::render_engine::ExportFormat::Wav,
                            sample_rate: 48000,
                            bit_depth: crate::engine::render_engine::BitDepth::Integer24,
                            dithering: crate::engine::render_engine::DitherMode::None,
                            normalize_lufs: None,
                            range: crate::engine::render_engine::RenderRange::Selection(0, range_end),
                            stem_export: vec![],
                            dry_run: false,
                            mp3_bitrate: 320,
                        };

                        let (tx, _rx) = crossbeam_channel::unbounded();
                        let mut render_engine = crate::engine::render_engine::RenderEngine::new(
                            graph_snapshot,
                            track_snapshot,
                            hp_t.clone(),
                            hs_t.clone(),
                            fades_t.clone(),
                            config,
                            tx,
                        );

                        // 3. Render
                        render_engine.render();

                        // 4. Import back
                        if let Ok(mut new_clip) = load_audio_file(PathBuf::from(&path_str), 48000.0) {
                            new_clip.start_sample = 0; 
                            let new_clip_clone = new_clip.clone();
                            library_management.lock().unwrap().push(new_clip);
                            let mut tracks = tracks_management.lock().unwrap();
                            if let Some(track) = tracks.get_mut(t_idx) {
                                track.clips.push(new_clip_clone);
                            }
                        }
                    }
                    AudioCommand::SetEffectBypass(track_idx, effect_id_str, bypass) => {
                        let mut resolved_idx = None;
                        if let Ok(uuid) = Uuid::parse_str(&effect_id_str) {
                             if let Some(track) = tracks_management.lock().unwrap().get(track_idx) {
                                 resolved_idx = track.processors.iter().position(|p| p.id() == uuid);
                             }
                        }
                        if let Some(idx) = resolved_idx {
                            let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetEffectBypass(track_idx, idx, bypass));
                            // Update Management (Dummy)
                            if let Some(track) = tracks_management.lock().unwrap().get_mut(track_idx) {
                                if let Some(p) = track.processors.get_mut(idx) {
                                    p.set_bypass(bypass);
                                }
                            }
                        }
                    }
                    AudioCommand::MoveEffect(track_idx, effect_id_str, to_idx) => {
                        let mut from_idx = None;
                        if let Ok(uuid) = Uuid::parse_str(&effect_id_str) {
                             if let Some(track) = tracks_management.lock().unwrap().get(track_idx) {
                                 from_idx = track.processors.iter().position(|p| p.id() == uuid);
                             }
                        }
                        if let Some(from) = from_idx {
                             // Update Management
                             let mut updated = false;
                             if let Some(track) = tracks_management.lock().unwrap().get_mut(track_idx) {
                                 if from < track.processors.len() {
                                     let p = track.processors.remove(from);
                                     let insert_at = if to_idx > from { to_idx - 1 } else { to_idx };
                                     if insert_at <= track.processors.len() {
                                         track.processors.insert(insert_at, p);
                                         updated = true;
                                     }
                                 }
                             }
                             if updated {
                                 let _ = graph_prod_management.lock().unwrap().push(GraphCommand::MoveEffect(track_idx, from, to_idx));
                             }
                        }
                    }
                    AudioCommand::RemoveEffect(track_idx, effect_id_str) => {
                        let mut resolved_idx = None;
                        if let Ok(uuid) = Uuid::parse_str(&effect_id_str) {
                            if let Some(track) = tracks_management.lock().unwrap().get(track_idx) {
                                resolved_idx = track.processors.iter().position(|p| p.id() == uuid);
                            }
                        }
                        if let Some(idx) = resolved_idx {
                            if let Some(track) = tracks_management.lock().unwrap().get_mut(track_idx) {
                                if idx < track.processors.len() {
                                    track.processors.remove(idx);
                                }
                            }
                            let _ = graph_prod_management.lock().unwrap().push(GraphCommand::RemoveEffect(track_idx, idx));
                        }
                    }
                    AudioCommand::AddWasmPlugin(track_idx, path) => {
                        if let Ok(bytes) = std::fs::read(&path) {
                             let name = std::path::Path::new(&path).file_stem().unwrap_or_default().to_string_lossy().to_string();
                             if let Ok(proc) = crate::engine::wasm_processor::WasmAudioProcessor::new(&bytes, name, sample_rate) {
                                 let wrapper = crate::engine::processors::wrapper::SmartProcessorWrapper::new(Box::new(proc));
                                 let b = Box::new(wrapper);
                                 
                                 // Add to Management (Dummy)
                                 if let Some(track) = tracks_management.lock().unwrap().get_mut(track_idx) {
                                      let dummy = b.clone_box();
                                      track.processors.push(dummy);
                                 }
                                 
                                 // Add to Audio Thread (Real)
                                 let _ = graph_prod_management.lock().unwrap().push(GraphCommand::AddProcessor(track_idx, b));
                             } else {
                                 eprintln!("VIBE: Failed to load WASM plugin: {}", path);
                             }
                        }
                    }
                    AudioCommand::SetLoopRange(start, end) => {
                        loop_start_shared.store(start, Ordering::Release);
                        loop_end_shared.store(end, Ordering::Release);
                        println!("VIBE: Loop Range Set: {} - {}", start, end);
                    }
                    AudioCommand::SetLoopEnabled(enabled) => {
                        loop_enabled_shared.store(enabled, Ordering::Release);
                        println!("VIBE: Loop {}", if enabled { "ENABLED" } else { "DISABLED" });
                    }
                    AudioCommand::AddEffect(index, effect_type) => {
                        let effect: Option<Box<dyn AudioProcessor>> = match effect_type.as_str() {
                            "lowpass" => {
                                Some(Box::new(crate::engine::processors::BiquadFilter::new(
                                    crate::engine::processors::FilterMode::LowPass,
                                    1000.0,
                                    0.707,
                                )))
                            }
                            "highpass" => {
                                Some(Box::new(crate::engine::processors::BiquadFilter::new(
                                    crate::engine::processors::FilterMode::HighPass,
                                    100.0,
                                    0.707,
                                )))
                            }
                            "filter" => {
                                Some(Box::new(crate::engine::processors::BiquadFilter::new(
                                    crate::engine::processors::FilterMode::LowPass,
                                    20000.0,
                                    0.707,
                                )))
                            }
                            "gain" => Some(Box::new(GainEffect::new(1.0))),
                            "delay" => {
                                Some(Box::new(crate::engine::processors::StereoDelay::new()))
                            }
                            "saturation" => {
                                Some(Box::new(crate::engine::processors::VibeSaturation::new()))
                            }
                            "compressor" => Some(Box::new(Compressor::new(sample_rate))),
                            "reverb" => Some(Box::new(Reverb::new())),
                            "vonesynth" => Some(Box::new(VOneSynth::new())),
                            "eq" => Some(Box::new(crate::engine::eq_module::Equalizer::new(
                                sample_rate,
                            ))),
                            "tubelimiter" => {
                                Some(Box::new(crate::engine::processors::TubeLimiter::new()))
                            }
                            "spectralgate" => {
                                Some(Box::new(SpectralGate::new(512))) // Fixed block size for spectral gate
                            }
                            "stereoimager" => {
                                Some(Box::new(StereoImager::new()))
                            }
                            "multiband" => {
                                Some(Box::new(MultibandDynamics::new(sample_rate as f64)))
                            }
                            "convolution" => {
                                // Create a default 1s exponential decay IR for initialization
                                let ir_len = sample_rate as usize;
                                let mut ir_l = vec![0.0; ir_len];
                                let mut ir_r = vec![0.0; ir_len];
                                for i in 0..ir_len {
                                    let t = i as f64 / (sample_rate as f64);
                                    let decay = (-t * 8.0).exp();
                                    ir_l[i] = (rand::random::<f64>() * 2.0 - 1.0) * decay;
                                    ir_r[i] = (rand::random::<f64>() * 2.0 - 1.0) * decay;
                                }
                                Some(Box::new(ConvolutionReverb::new(&ir_l, &ir_r, 512))) // 512 is a safe default block size
                            }
                            "psycho" => {
                                Some(Box::new(crate::engine::psycho::PsychoacousticEngine::new()))
                            }
                            _ => None,
                        };

                        if let Some(e) = effect {
                            // Management
                            if let Some(track) = tracks_management.lock().unwrap().get_mut(index) {
                                let mgmt_proc = e.clone_box();
                                track.processors.push(Box::new(
                                    super::processors::SmartProcessorWrapper::new(mgmt_proc),
                                ));
                            }
                            // Audio Thread
                            let wrapped =
                                Box::new(super::processors::SmartProcessorWrapper::new(e));
                            let _ = graph_prod_management
                                .lock()
                                .unwrap()
                                .push(GraphCommand::AddProcessor(index, wrapped));
                        }
                    }
                    AudioCommand::LoadSynthPreset(track_idx, proc_idx, path) => {
                        // 1. Read & Parse JSON (Management Thread - Blocking I/O OK here)
                        // Minimize risk by doing I/O before locks
                        let preset_result = std::fs::File::open(&path)
                            .and_then(|f| serde_json::from_reader::<_, crate::engine::synth::SynthPreset>(f).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)));

                        match preset_result {
                            Ok(preset) => {
                                // 2. Send to Audio Thread (Realtime - Zero Allocation/No IO)
                                let _ = graph_prod_management
                                    .lock()
                                    .unwrap()
                                    .push(GraphCommand::LoadPreset(track_idx, proc_idx, preset.clone()));

                                // 3. Update Management Copy (Persisted State)
                                let mut track_list = tracks_management.lock().unwrap();
                                if let Some(track) = track_list.get_mut(track_idx) {
                                    if let Some(proc) = track.processors.get_mut(proc_idx) {
                                        if let Some(synth) = proc
                                            .as_any()
                                            .downcast_mut::<crate::engine::synth::VOneSynth>()
                                        {
                                            // helper method on VOneSynth or direct set
                                            synth.set_patch(&preset);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("VIBE: Failed to load synth preset '{}': {}", path, e);
                            }
                        }
                    }
                    AudioCommand::UpdateModMatrix(track_idx, proc_idx, slots) => {
                        // 1. Send to Audio Thread
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::UpdateModMatrix(track_idx, proc_idx, slots.clone()));

                        // 2. Update Management Copy (Persisted State)
                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(track_idx) {
                            if let Some(proc) = track.processors.get_mut(proc_idx) {
                                if let Some(synth) = proc
                                    .as_any()
                                    .downcast_mut::<crate::engine::synth::VOneSynth>()
                                {
                                    for (i, slot) in slots.iter().enumerate() {
                                        if i < 8 {
                                            synth.mod_matrix[i] = *slot;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AudioCommand::SaveSynthPreset(track_idx, proc_idx, path) => {
                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(track_idx) {
                            if let Some(proc) = track.processors.get_mut(proc_idx) {
                                if let Some(synth) = proc
                                    .as_any()
                                    .downcast_mut::<crate::engine::synth::VOneSynth>()
                                {
                                    let _ = synth.save_to_json(&path);
                                }
                            }
                        }
                    }
                    AudioCommand::SetPlayhead(pos) => {
                        println!("VIBE: AudioCommand::SetPlayhead set to {}", pos);
                        playhead_shared.store(pos, Ordering::Release);
                    }
                    AudioCommand::SetBPM(new_bpm) => {
                        if (1.0..=999.0).contains(&new_bpm) {
                            *bpm_management.lock().unwrap() = new_bpm;
                            bpm_shared.store(new_bpm.to_bits() as u64, Ordering::Release);
                        // Update atomic BPM
                        } else {
                            println!("VIBE: Invalid BPM requested: {}", new_bpm);
                        }
                    }
                    AudioCommand::AddBus(name, color) => {
                        let bus = Bus::new(name, color);
                        let bus_clone = bus.clone();
                        busses_management.lock().unwrap().push(bus);
                        let _ = graph_prod_management
                            .lock()
                            .unwrap()
                            .push(GraphCommand::AddBus(bus_clone));
                    }
                    AudioCommand::RouteTrackToBus(track_index, bus_id) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(track_index)
                        {
                            track.bus_id = Some(bus_id);
                        }
                    }
                    AudioCommand::MidiNoteOn(note, velocity) => {
                        let _ = midi_prod_management.lock().unwrap().push(MidiEvent {
                            sample_offset: 0,
                            status: 0x90,
                            data1: note as u16,
                            data2: (velocity as u32) << 25,
                        });
                    }
                    AudioCommand::MidiNoteOff(note) => {
                        let _ = midi_prod_management.lock().unwrap().push(MidiEvent {
                            sample_offset: 0,
                            status: 0x80,
                            data1: note as u16,
                            data2: 0,
                        });
                    }
                    AudioCommand::MidiCC(cc, val) => {
                        let map = midi_map_management.lock().unwrap();
                        if let Some(param_id) = map.get(&cc) {
                            let mut track_list = tracks_management.lock().unwrap();
                            let recording = is_recording_shared.load(Ordering::Acquire);
                            let playing = is_playing_shared.load(Ordering::Acquire);
                            let pos = playhead_shared.load(Ordering::Acquire);

                            // Find the parameter across all tracks/processors
                            // Optimization: Could store track_index/proc_index in map too?
                            // For now simple traversal.
                            for track in track_list.iter_mut() {
                                if track.volume.id == *param_id {
                                    let range = track.volume.max_value - track.volume.min_value;
                                    let normalized = val as f64 / 127.0;
                                    let new_val = track.volume.min_value + normalized * range;
                                    track.volume.value = new_val;
                                    if recording && playing {
                                        track.volume.record_value(pos, new_val);
                                    }
                                    break;
                                }
                                for processor in &mut track.processors {
                                    for param in processor.get_parameters() {
                                        if param.id == *param_id {
                                            let range = param.max_value - param.min_value;
                                            let normalized = val as f64 / 127.0;
                                            let new_val = param.min_value + normalized * range;
                                            param.set_value(new_val);
                                            if recording && playing {
                                                param.record_value(pos, new_val);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AudioCommand::MidiControl(_channel, cc, val_norm) => {
                        let map = midi_map_management.lock().unwrap();
                        if let Some(param_id) = map.get(&cc) {
                            let mut track_list = tracks_management.lock().unwrap();
                            let recording = is_recording_shared.load(Ordering::Acquire);
                            let playing = is_playing_shared.load(Ordering::Acquire);
                            let pos = playhead_shared.load(Ordering::Acquire);

                            for track in track_list.iter_mut() {
                                if track.volume.id == *param_id {
                                    let range = track.volume.max_value - track.volume.min_value;
                                    let new_val = track.volume.min_value + val_norm * range;
                                    track.volume.value = new_val;
                                    if recording && playing {
                                        track.volume.record_value(pos, new_val);
                                    }
                                    break;
                                }
                                for processor in &mut track.processors {
                                    for param in processor.get_parameters() {
                                        if param.id == *param_id {
                                            let range = param.max_value - param.min_value;
                                            let new_val = param.min_value + val_norm * range;
                                            param.set_value(new_val);
                                            if recording && playing {
                                                param.record_value(pos, new_val);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    AudioCommand::SetGlobalSwing(swing) => {
                        global_swing_shared.store(swing.to_bits() as u64, Ordering::Release);
                        println!("VIBE: Global Swing set to {}%", (swing * 100.0) as i32);
                    }
                    AudioCommand::MapMidi(cc, param_id) => {
                        midi_map_management.lock().unwrap().insert(cc, param_id);
                        println!("VIBE: Mapped CC {} to Param {}", cc, param_id);
                    }
                    AudioCommand::Undo => {
                        let mut hist = history_management.lock().unwrap();
                        let lib = library_management.lock().unwrap();
                        if let Some(snapshot) = hist.undo() {
                            *tracks_management.lock().unwrap() = snapshot
                                .tracks
                                .into_iter()
                                .map(|ti| {
                                    let mut t = Track::new(ti.name);
                                    t.id = Uuid::parse_str(&ti.id).unwrap_or(Uuid::new_v4());
                                    t.volume.set_value(ti.volume.value);
                                    let mut v_curve =
                                        crate::engine::automation::AutomationCurve::new(
                                            ti.volume.value,
                                        );
                                    v_curve.knots = ti.volume.automation;
                                    t.volume.curve.store(Arc::new(v_curve));
                                    t.is_muted = ti.muted;
                                    t.is_solo = ti.solo;
                                    t.color = ti.color;

                                    // Restore Clips
                                    for ci in ti.clips {
                                        if let Some(template) =
                                            lib.iter().find(|c| c.name == ci.audio_path)
                                        {
                                            let mut clip = template.clone();
                                            clip.id =
                                                Uuid::parse_str(&ci.id).unwrap_or(Uuid::new_v4());
                                            clip.start_sample = ci.start_sample;
                                            clip.offset_in_data = ci.offset_in_data;
                                            clip.length_in_samples = ci.duration_samples;
                                            clip.fade_in_len = ci.fade_in_len;
                                            clip.fade_out_len = ci.fade_out_len;
                                            clip.fade_in_type = ci.fade_in_type;
                                            clip.fade_out_type = ci.fade_out_type;
                                            t.clips.push(clip);
                                        }
                                    }

                                    // Restore Effects
                                    for ei in ti.plugins {
                                        let effect: Option<Box<dyn AudioProcessor>> = match ei
                                            .plugin_path
                                            .as_str()
                                        {
                                            "Low Pass Filter" | "VIBE Filter" => Some(Box::new(
                                                crate::engine::processors::BiquadFilter::new(
                                                    crate::engine::processors::FilterMode::LowPass,
                                                    1000.0,
                                                    0.707,
                                                ),
                                            )),
                                            "High Pass Filter" => Some(Box::new(
                                                crate::engine::processors::BiquadFilter::new(
                                                    crate::engine::processors::FilterMode::HighPass,
                                                    100.0,
                                                    0.707,
                                                ),
                                            )),
                                            "Gain" => Some(Box::new(GainEffect::new(1.0))),
                                            "Delay" => Some(Box::new(Delay::new(0.5, 0.4, 0.3))),
                                            "Saturation" => Some(Box::new(Saturation::new(2.0))),
                                            "VIBE Saturation" => Some(Box::new(
                                                crate::engine::processors::VibeSaturation::new(),
                                            )),
                                            "Compressor" => {
                                                Some(Box::new(Compressor::new(sample_rate)))
                                            }
                                            "Reverb" => Some(Box::new(Reverb::new())),
                                            "VOneSynth" => Some(Box::new(VOneSynth::new())),
                                            "Prisma EQ" => Some(Box::new(
                                                crate::engine::eq_module::Equalizer::new(
                                                    sample_rate,
                                                ),
                                            )),
                                            _ => None,
                                        };

                                        if let Some(mut e) = effect {
                                            for (p, pi) in
                                                e.get_parameters().into_iter().zip(ei.parameters)
                                            {
                                                p.set_value(pi.value);
                                                let mut curve =
                                                    crate::engine::automation::AutomationCurve::new(
                                                        pi.value,
                                                    );
                                                curve.knots = pi.automation;
                                                p.curve.store(Arc::new(curve));
                                            }
                                            t.processors.push(e);
                                        }
                                    }

                                    // Restore MIDI Clips
                                    for mi in ti.midi_clips {
                                        let mc = super::graph::MidiClip {
                                            id: Uuid::parse_str(&mi.id).unwrap_or(Uuid::new_v4()),
                                            name: mi.name,
                                            start_sample: mi.start_sample,
                                            length_samples: mi.length_samples,
                                            notes: mi
                                                .notes
                                                .into_iter()
                                                .map(|n| super::graph::MidiNote {
                                                    start_sample: n.start_sample,
                                                    length_samples: n.length_samples,
                                                    note: n.note,
                                                    velocity: n.velocity,
                                                    channel: n.channel,
                                                    pitch_bend: n.pitch_bend,
                                                    pressure: n.pressure,
                                                    timbre: n.timbre,
                                                    probability: n.probability,
                                                    velocity_random: n.velocity_random,
                                                    timing_random: n.timing_random,
                                                })
                                                .collect(),
                                            cc_events: mi
                                                .cc_events
                                                .into_iter()
                                                .map(|cc| super::graph::MidiCCEvent {
                                                    sample: cc.sample,
                                                    cc_number: cc.cc_number,
                                                    value: cc.value,
                                                    channel: cc.channel,
                                                })
                                                .collect(),
                                            color: mi.color,
                                            is_muted: mi.is_muted,
                                            is_looped: mi.is_looped,
                                            scale: mi.scale,
                                            chord_markers: mi.chord_markers,
                                            groove_template: mi.groove_template,
                                            pattern_id: mi.pattern_id,
                                            tuning_steps: mi.tuning_steps,
                                            time_signature_num: mi.time_signature_num,
                                            time_signature_den: mi.time_signature_den,
                                            reference_clip_id: None,
                                        };
                                        t.midi_clips.push(mc);
                                    }

                                    t
                                })
                                .collect();
                            *bpm_management.lock().unwrap() = snapshot.bpm;
                        }
                    }
                    AudioCommand::Redo => {
                        let mut hist = history_management.lock().unwrap();
                        let lib = library_management.lock().unwrap();
                        if let Some(snapshot) = hist.redo() {
                            *tracks_management.lock().unwrap() = snapshot
                                .tracks
                                .into_iter()
                                .map(|ti| {
                                    let mut t = Track::new(ti.name);
                                    t.id = Uuid::parse_str(&ti.id).unwrap_or(Uuid::new_v4());
                                    t.volume.set_value(ti.volume.value);
                                    let mut v_curve =
                                        crate::engine::automation::AutomationCurve::new(
                                            ti.volume.value,
                                        );
                                    v_curve.knots = ti.volume.automation;
                                    t.volume.curve.store(Arc::new(v_curve));
                                    t.is_muted = ti.muted;
                                    t.is_solo = ti.solo;
                                    t.color = ti.color;

                                    for ci in ti.clips {
                                        if let Some(template) =
                                            lib.iter().find(|c| c.name == ci.audio_path)
                                        {
                                            let mut clip = template.clone();
                                            clip.id =
                                                Uuid::parse_str(&ci.id).unwrap_or(Uuid::new_v4());
                                            clip.start_sample = ci.start_sample;
                                            clip.offset_in_data = ci.offset_in_data;
                                            clip.length_in_samples = ci.duration_samples;
                                            clip.fade_in_len = ci.fade_in_len;
                                            clip.fade_out_len = ci.fade_out_len;
                                            t.clips.push(clip);
                                        }
                                    }

                                    for ei in ti.plugins {
                                        let effect: Option<Box<dyn AudioProcessor>> = match ei
                                            .plugin_path
                                            .as_str()
                                        {
                                            "Low Pass Filter" | "VIBE Filter" => Some(Box::new(
                                                crate::engine::processors::BiquadFilter::new(
                                                    crate::engine::processors::FilterMode::LowPass,
                                                    1000.0,
                                                    0.707,
                                                ),
                                            )),
                                            "High Pass Filter" => Some(Box::new(
                                                crate::engine::processors::BiquadFilter::new(
                                                    crate::engine::processors::FilterMode::HighPass,
                                                    100.0,
                                                    0.707,
                                                ),
                                            )),
                                            "Gain" => Some(Box::new(GainEffect::new(1.0))),
                                            "Delay" => Some(Box::new(Delay::new(0.5, 0.4, 0.3))),
                                            "Saturation" => Some(Box::new(Saturation::new(2.0))),
                                            "VIBE Saturation" => Some(Box::new(
                                                crate::engine::processors::VibeSaturation::new(),
                                            )),
                                            "Compressor" => {
                                                Some(Box::new(Compressor::new(sample_rate)))
                                            }
                                            "Reverb" => Some(Box::new(Reverb::new())),
                                            "VOneSynth" => Some(Box::new(VOneSynth::new())),
                                            "Prisma EQ" => Some(Box::new(
                                                crate::engine::eq_module::Equalizer::new(
                                                    sample_rate,
                                                ),
                                            )),
                                            _ => None,
                                        };

                                        if let Some(mut e) = effect {
                                            for (p, pi) in
                                                e.get_parameters().into_iter().zip(ei.parameters)
                                            {
                                                p.set_value(pi.value);
                                                let mut curve =
                                                    crate::engine::automation::AutomationCurve::new(
                                                        pi.value,
                                                    );
                                                curve.knots = pi.automation;
                                                p.curve.store(Arc::new(curve));
                                            }
                                            t.processors.push(e);
                                        }
                                    }

                                    // Restore MIDI Clips
                                    for mi in ti.midi_clips {
                                        let mc = super::graph::MidiClip {
                                            id: Uuid::parse_str(&mi.id).unwrap_or(Uuid::new_v4()),
                                            name: mi.name,
                                            start_sample: mi.start_sample,
                                            length_samples: mi.length_samples,
                                            notes: mi
                                                .notes
                                                .into_iter()
                                                .map(|n| super::graph::MidiNote {
                                                    start_sample: n.start_sample,
                                                    length_samples: n.length_samples,
                                                    note: n.note,
                                                    velocity: n.velocity,
                                                    channel: n.channel,
                                                    pitch_bend: n.pitch_bend,
                                                    pressure: n.pressure,
                                                    timbre: n.timbre,
                                                    probability: n.probability,
                                                    velocity_random: n.velocity_random,
                                                    timing_random: n.timing_random,
                                                })
                                                .collect(),
                                            cc_events: mi
                                                .cc_events
                                                .into_iter()
                                                .map(|cc| super::graph::MidiCCEvent {
                                                    sample: cc.sample,
                                                    cc_number: cc.cc_number,
                                                    value: cc.value,
                                                    channel: cc.channel,
                                                })
                                                .collect(),
                                            color: mi.color,
                                            is_muted: mi.is_muted,
                                            is_looped: mi.is_looped,
                                            scale: mi.scale,
                                            chord_markers: mi.chord_markers,
                                            groove_template: mi.groove_template,
                                            pattern_id: mi.pattern_id,
                                            tuning_steps: mi.tuning_steps,
                                            time_signature_num: mi.time_signature_num,
                                            time_signature_den: mi.time_signature_den,
                                            reference_clip_id: None,
                                        };
                                        t.midi_clips.push(mc);
                                    }

                                    t
                                })
                                .collect();
                            *bpm_management.lock().unwrap() = snapshot.bpm;
                        }
                    }
                    AudioCommand::Checkout(node_id) => {
                        let mut hist = history_management.lock().unwrap();
                        let lib = library_management.lock().unwrap();
                        if let Some(snapshot) = hist.checkout(node_id) {
                            *tracks_management.lock().unwrap() = snapshot
                                .tracks
                                .into_iter()
                                .map(|ti| {
                                    let mut t = Track::new(ti.name);
                                    t.id = Uuid::parse_str(&ti.id).unwrap_or(Uuid::new_v4());
                                    t.volume.set_value(ti.volume.value);
                                    let mut v_curve =
                                        crate::engine::automation::AutomationCurve::new(
                                            ti.volume.value,
                                        );
                                    v_curve.knots = ti.volume.automation;
                                    t.volume.curve.store(Arc::new(v_curve));
                                    t.is_muted = ti.muted;
                                    t.is_solo = ti.solo;
                                    t.color = ti.color;

                                    for ci in ti.clips {
                                        if let Some(template) =
                                            lib.iter().find(|c| c.name == ci.audio_path)
                                        {
                                            let mut clip = template.clone();
                                            clip.id =
                                                Uuid::parse_str(&ci.id).unwrap_or(Uuid::new_v4());
                                            clip.start_sample = ci.start_sample;
                                            clip.offset_in_data = ci.offset_in_data;
                                            clip.length_in_samples = ci.duration_samples;
                                            clip.fade_in_len = ci.fade_in_len;
                                            clip.fade_out_len = ci.fade_out_len;
                                            t.clips.push(clip);
                                        }
                                    }

                                    for ei in ti.plugins {
                                        let effect: Option<Box<dyn AudioProcessor>> = match ei
                                            .plugin_path
                                            .as_str()
                                        {
                                            "Low Pass Filter" | "VIBE Filter" => Some(Box::new(
                                                crate::engine::processors::BiquadFilter::new(
                                                    crate::engine::processors::FilterMode::LowPass,
                                                    1000.0,
                                                    0.707,
                                                ),
                                            )),
                                            "High Pass Filter" => Some(Box::new(
                                                crate::engine::processors::BiquadFilter::new(
                                                    crate::engine::processors::FilterMode::HighPass,
                                                    100.0,
                                                    0.707,
                                                ),
                                            )),
                                            "Gain" => Some(Box::new(GainEffect::new(1.0))),
                                            "Delay" => Some(Box::new(Delay::new(0.5, 0.4, 0.3))),
                                            "Saturation" => Some(Box::new(Saturation::new(2.0))),
                                            "VIBE Saturation" => Some(Box::new(
                                                crate::engine::processors::VibeSaturation::new(),
                                            )),
                                            "Compressor" => {
                                                Some(Box::new(Compressor::new(sample_rate)))
                                            }
                                            "Reverb" => Some(Box::new(Reverb::new())),
                                            "VOneSynth" => Some(Box::new(VOneSynth::new())),
                                            "Prisma EQ" => Some(Box::new(
                                                crate::engine::eq_module::Equalizer::new(
                                                    sample_rate,
                                                ),
                                            )),
                                            _ => None,
                                        };

                                        if let Some(mut e) = effect {
                                            // Restore binary state (VST chunks, etc.)
                                            e.set_state(&ei.state_blob);

                                            for (p, pi) in
                                                e.get_parameters().into_iter().zip(ei.parameters)
                                            {
                                                p.set_value(pi.value);
                                                let mut curve =
                                                    crate::engine::automation::AutomationCurve::new(
                                                        pi.value,
                                                    );
                                                curve.knots = pi.automation;
                                                p.curve.store(Arc::new(curve));
                                            }
                                            t.processors.push(e);
                                        }
                                    }

                                    // Restore MIDI Clips
                                    for mi in ti.midi_clips {
                                        let mc = super::graph::MidiClip {
                                            id: Uuid::parse_str(&mi.id).unwrap_or(Uuid::new_v4()),
                                            name: mi.name,
                                            start_sample: mi.start_sample,
                                            length_samples: mi.length_samples,
                                            notes: mi
                                                .notes
                                                .into_iter()
                                                .map(|n| super::graph::MidiNote {
                                                    start_sample: n.start_sample,
                                                    length_samples: n.length_samples,
                                                    note: n.note,
                                                    velocity: n.velocity,
                                                    channel: n.channel,
                                                    pitch_bend: n.pitch_bend,
                                                    pressure: n.pressure,
                                                    timbre: n.timbre,
                                                    probability: n.probability,
                                                    velocity_random: n.velocity_random,
                                                    timing_random: n.timing_random,
                                                })
                                                .collect(),
                                            cc_events: mi
                                                .cc_events
                                                .into_iter()
                                                .map(|cc| super::graph::MidiCCEvent {
                                                    sample: cc.sample,
                                                    cc_number: cc.cc_number,
                                                    value: cc.value,
                                                    channel: cc.channel,
                                                })
                                                .collect(),
                                            color: mi.color,
                                            is_muted: mi.is_muted,
                                            is_looped: mi.is_looped,
                                            scale: mi.scale,
                                            chord_markers: mi.chord_markers,
                                            groove_template: mi.groove_template,
                                            pattern_id: mi.pattern_id,
                                            tuning_steps: mi.tuning_steps,
                                            time_signature_num: mi.time_signature_num,
                                            time_signature_den: mi.time_signature_den,
                                            reference_clip_id: None,
                                        };
                                        t.midi_clips.push(mc);
                                    }

                                    t
                                })
                                .collect();
                            *bpm_management.lock().unwrap() = snapshot.bpm;
                        }
                    }
                    AudioCommand::CreateBranch(name) => {
                        history_management.lock().unwrap().create_branch(name);
                    }
                    AudioCommand::ScanPlugins => {
                        let scanner =
                            super::scanner::PluginScanner::new(plugin_path_management.clone());
                        let found = scanner.scan();
                        println!("VIBE: Found {} plugins", found.len());
                        *plugins_management.lock().unwrap() = found;
                    }
                    AudioCommand::AddPluginToTrack(index, path) => {
                        let path_str = path.to_string_lossy();
                        if path_str.starts_with("native://") {
                            let effect_type = path_str.trim_start_matches("native://");
                            if !effect_type.is_empty() {
                                command_tx_t.send(AudioCommand::AddEffect(index, effect_type.to_string())).unwrap();
                                return;
                            }
                        }

                        let mut track_list = tracks_management.lock().unwrap();
                        if let Some(track) = track_list.get_mut(index) {
                            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                            let name = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();

                            if ext == "wasm" {
                                if let Ok(wasm_bytes) = std::fs::read(&path) {
                                    match super::wasm_processor::WasmAudioProcessor::new(
                                        &wasm_bytes,
                                        name,
                                        sample_rate,
                                    ) {
                                        Ok(plugin) => {
                                            track.processors.push(Box::new(
                                                super::processors::SmartProcessorWrapper::new(
                                                    Box::new(plugin),
                                                ),
                                            ));
                                            println!(
                                                "VIBE: Loaded WASM plugin: {}",
                                                path.display()
                                            );
                                        }
                                        Err(e) => {
                                            println!("VIBE: Failed to init WASM plugin: {}", e)
                                        }
                                    }
                                }
                            } else if ext == "dll" {
                                println!("VIBE: Legacy VST2 (.dll) no longer supported. Please use VST3.");
                            } else if ext == "vst3" {
                                match super::vst3_bridge::Vst3Bridge::new(
                                    &path.to_string_lossy(),
                                    sample_rate,
                                    512,
                                ) {
                                    Ok(plugin) => {
                                        track.processors.push(Box::new(
                                            super::processors::SmartProcessorWrapper::new(
                                                Box::new(plugin),
                                            ),
                                        ));
                                        println!("VIBE: Loaded VST3 plugin: {}", path.display());
                                    }
                                    Err(e) => {
                                        println!(
                                            "VIBE: VST3 Load failed, attempting Sandboxed Mode..."
                                        );
                                        // Fallback to Sandbox for unstable/unsupported plugins
                                        if let Ok(safe_plugin) =
                                            super::sandbox::SandboxedPlugin::new(name)
                                        {
                                            track.processors.push(Box::new(
                                                super::processors::SmartProcessorWrapper::new(
                                                    Box::new(safe_plugin),
                                                ),
                                            ));
                                            println!("VIBE: Loaded plugin in Magneto-Gravitational Sandbox");
                                        } else {
                                            println!("VIBE: Failed to init VST3 plugin: {}", e);
                                        }
                                    }
                                }
                            } else {
                                println!("VIBE: Unsupported plugin format: .{}", ext);
                            }
                        }
                    }
                    AudioCommand::SetMetronome(enabled) => {
                        metronome_enabled_shared.store(enabled, Ordering::Release);
                    }

                    // --- Advanced Routing Matrix Commands ---
                    AudioCommand::GraphAddNode(node) => {
                        let manager = crate::engine::routing::GraphManager::new(Arc::clone(
                            &audio_graph_thread,
                        ));
                        let _ = manager.add_node(node);
                        graph_dirty_thread.store(true, Ordering::Release);
                    }
                    AudioCommand::GraphRemoveNode(id) => {
                        let manager = crate::engine::routing::GraphManager::new(Arc::clone(
                            &audio_graph_thread,
                        ));
                        if let Err(e) = manager.remove_node(id) {
                            println!("VIBE: Graph remove error: {}", e);
                        } else {
                            graph_dirty_thread.store(true, Ordering::Release);
                        }
                    }
                    AudioCommand::GraphConnect {
                        from_node,
                        to_node,
                        from_port,
                        to_port,
                        gain_db,
                    } => {
                        let manager = crate::engine::routing::GraphManager::new(Arc::clone(
                            &audio_graph_thread,
                        ));
                        if let (Some(src), Some(dst)) = (
                            manager.find_node_by_id(from_node),
                            manager.find_node_by_id(to_node),
                        ) {
                            let edge = crate::engine::audio_graph::GraphEdge {
                                from_port,
                                to_port,
                                gain_db,
                                signal_type: crate::engine::audio_graph::SignalType::Audio {
                                    pre_fader: false,
                                },
                            };
                            match manager.connect(src, dst, edge) {
                                Ok(_) => graph_dirty_thread.store(true, Ordering::Release),
                                Err(e) => println!("VIBE: Graph connect error: {}", e),
                            }
                        } else {
                            println!("VIBE: Connect failed - node not found");
                        }
                    }
                    AudioCommand::GraphDisconnect { from_node, to_node } => {
                        let mut graph = audio_graph_thread.lock().unwrap();
                        if let (Some(src), Some(dst)) = (
                            graph.node_indices().find(|i| graph[*i].id == from_node),
                            graph.node_indices().find(|i| graph[*i].id == to_node),
                        ) {
                            if let Some(edge_idx) = graph.find_edge(src, dst) {
                                graph.remove_edge(edge_idx);
                                graph_dirty_thread.store(true, Ordering::Release);
                            }
                        }
                    }

                    // --- MIDI Sequencer Commands (Phase 2) ---
                    AudioCommand::AddMidiClip(track_idx, clip) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            track.midi_clips.push(clip);
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::DeleteMidiClip(track_idx, clip_id) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            track.midi_clips.retain(|c| c.id != clip_id);
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::UpdateMidiClip(track_idx, clip_id, updated_clip) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) =
                                track.midi_clips.iter_mut().find(|c| c.id == clip_id)
                            {
                                *clip = updated_clip;
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::StartMidiRecording(track_idx) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            track.midi_recording_buffer.clear();
                        }
                        // Note: Logic to actually capture MIDI is in the audio callback or PortMidi loop
                    }
                    AudioCommand::StopMidiRecording(track_idx) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if !track.midi_recording_buffer.is_empty() {
                                let new_clip = MidiClip {
                                    id: Uuid::new_v4(),
                                    name: format!(
                                        "Recorded MIDI {}",
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs()
                                    ),
                                    start_sample: 0, // Should be set based on first note or recording start
                                    length_samples: 48000 * 4, // Placeholder 4 seconds
                                    notes: track.midi_recording_buffer.clone(),
                                    cc_events: Vec::new(),
                                    color: track.color.clone(),
                                    is_muted: false,
                                    is_looped: false,
                                    scale: None,
                                    chord_markers: Vec::new(),
                                    groove_template: None,
                                    pattern_id: None,
                                    tuning_steps: None,
                                    time_signature_num: None,
                                    time_signature_den: None,
                                    reference_clip_id: None,
                                };
                                track.midi_clips.push(new_clip);
                                track.midi_recording_buffer.clear();
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::SetQuantization(track_idx, division) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            track.quantize_division = division;
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }

                    // MIDI Note CRUD
                    AudioCommand::AddMidiNote(track_idx, clip_id, note) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                clip.notes.push(note);
                                clip.notes.sort_by_key(|n| n.start_sample);
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::DeleteMidiNote(track_idx, clip_id, note_idx) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                if note_idx < clip.notes.len() {
                                    clip.notes.remove(note_idx);
                                }
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::UpdateMidiNote(track_idx, clip_id, note_idx, new_note) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                if note_idx < clip.notes.len() {
                                    clip.notes[note_idx] = new_note;
                                    clip.notes.sort_by_key(|n| n.start_sample);
                                }
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }

                    // CC Event CRUD
                    AudioCommand::AddCCEvent(track_idx, clip_id, cc_event) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                clip.cc_events.push(cc_event);
                                clip.cc_events.sort_by_key(|e| e.sample);
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::DeleteCCEvent(track_idx, clip_id, cc_idx) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                if cc_idx < clip.cc_events.len() {
                                    clip.cc_events.remove(cc_idx);
                                }
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::GenerateStressNotes(track_idx, clip_id, count) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                use rand::Rng;
                                let mut rng = rand::thread_rng();
                                let clip_len = clip.length_samples;

                                for _ in 0..count {
                                    let start = rng.gen_range(0..clip_len);
                                    let len = rng.gen_range(12000..48000); // 0.25s to 1s at 48k
                                    let note_val = rng.gen_range(20..100);
                                    let vel = rng.gen_range(20..127);

                                    clip.notes.push(super::graph::MidiNote {
                                        start_sample: start,
                                        length_samples: len,
                                        note: note_val,
                                        velocity: vel,
                                        channel: 0,
                                        pitch_bend: Some(0),
                                        pressure: Some(0),
                                        timbre: Some(64),
                                        probability: 1.0,
                                        velocity_random: 0,
                                        timing_random: 0,
                                    });
                                }
                                clip.notes.sort_by_key(|n| n.start_sample);
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }

                    AudioCommand::SetEqBands(track_idx, processor_id, bands) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            let proc_uuid = Uuid::parse_str(&processor_id).unwrap_or(Uuid::nil());
                            if let Some(proc) =
                                track.processors.iter_mut().find(|p| p.id() == proc_uuid)
                            {
                                if let Some(eq) = proc
                                    .as_any()
                                    .downcast_mut::<crate::engine::eq_module::Equalizer>()
                                {
                                    eq.set_bands(bands);
                                }
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }

                    AudioCommand::UpdateEqBand(track_idx, processor_id, band) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            let proc_uuid = Uuid::parse_str(&processor_id).unwrap_or(Uuid::nil());
                            if let Some(proc) =
                                track.processors.iter_mut().find(|p| p.id() == proc_uuid)
                            {
                                if let Some(eq) = proc
                                    .as_any()
                                    .downcast_mut::<crate::engine::eq_module::Equalizer>()
                                {
                                    let mut current_bands = eq.get_bands();
                                    if let Some(existing) =
                                        current_bands.iter_mut().find(|b| b.id == band.id)
                                    {
                                        *existing = band;
                                    } else {
                                        current_bands.push(band);
                                    }
                                    eq.set_bands(current_bands);
                                }
                            }
                        }
                    }

                    // Composition Tools
                    AudioCommand::SetClipScale(track_idx, clip_id, scale) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                clip.scale = scale;
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::DetectChords(track_idx, clip_id) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                // Simple chord detection stub
                                clip.chord_markers = vec![ChordMarker {
                                    sample: 0,
                                    chord_name: "Cmaj".to_string(),
                                    confidence: 0.9,
                                }];
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::ApplyGrooveTemplate(track_idx, clip_id, template_name) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        let current_bpm = *bpm_management.lock().unwrap();

                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                // Mock Groove Library Lookup
                                let template = match template_name.as_str() {
                                    "Swing 50%" => Some(super::graph::GrooveTemplate {
                                        name: "Swing 50%".to_string(),
                                        timing_offsets: [0.0; 16],
                                        velocity_scale: [1.0; 16],
                                    }),
                                    "Swing 58%" => Some(super::graph::GrooveTemplate {
                                        name: "Swing 58%".to_string(),
                                        // Delay every second 16th note by ~8% of a beat (approx)
                                        timing_offsets: [
                                            0.0, 0.16, 0.0, 0.16, 0.0, 0.16, 0.0, 0.16, 0.0, 0.16,
                                            0.0, 0.16, 0.0, 0.16, 0.0, 0.16,
                                        ],
                                        velocity_scale: [
                                            1.0, 0.9, 1.0, 0.9, 1.0, 0.9, 1.0, 0.9, 1.0, 0.9, 1.0,
                                            0.9, 1.0, 0.9, 1.0, 0.9,
                                        ],
                                    }),
                                    _ => None, // Unknown template
                                };

                                if let Some(tmpl) = template {
                                    clip.apply_groove(&tmpl, current_bpm, 48000.0);
                                }
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::ApplyGrooveCustom(track_idx, clip_id, template) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        let current_bpm = *bpm_management.lock().unwrap();

                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                clip.apply_groove(&template, current_bpm, 48000.0);
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::QuantizeNotes(track_idx, clip_id, division) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        let current_bpm = *bpm_management.lock().unwrap();

                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track
                                .midi_clips
                                .iter_mut()
                                .find(|c| c.id.to_string() == clip_id)
                            {
                                clip.quantize(division, current_bpm, 48000.0);
                            }
                        }
                        Self::commit_history(
                            &history_management,
                            &tracks_management,
                            &bpm_management,
                        );
                    }
                    AudioCommand::NewProject => {
                        is_playing_shared.store(false, Ordering::Release);
                        playhead_shared.store(0, Ordering::Release);
                        if let Ok(mut dsp) = dsp_state_t.lock() {
                            dsp.preview_voice = None;
                        }
                    }
                    AudioCommand::ConvertAudioToMidi(track_idx, clip_id, mode) => {
                        let tracks_t = Arc::clone(&tracks_management);
                        thread::spawn(move || {
                             // Outside lock: get path
                             let path = {
                                 let tracks = tracks_t.lock().unwrap();
                                 tracks.get(track_idx).and_then(|t| t.clips.iter().find(|c| c.id == clip_id).and_then(|c| c.path.clone()))
                             };

                             if let Some(p) = path {
                                 if let Ok(samples) = decode_file_to_vec(&p) {
                                     let mut new_midi_clip = if mode == "poly" {
                                         let converter = crate::engine::audio_to_midi::AudioToMidiConverter::new(48000.0);
                                         converter.convert_polyphonic(&samples)
                                     } else {
                                         // Percussive/Transient mode
                                         let detector = TransientDetector::default();
                                         let transients = detector.detect(&samples, 48000.0);
                                         let clip = super::graph::MidiClip {
                                             id: Uuid::new_v4(),
                                             name: "Extracted Percussion".to_string(),
                                             start_sample: 0,
                                             length_samples: samples.len() as u64,
                                             notes: transients.into_iter().map(|t| super::graph::MidiNote {
                                                 start_sample: t.position_samples,
                                                 length_samples: 4800, // 100ms
                                                 note: 36, // C1 (Kick) default
                                                 velocity: (t.strength * 127.0) as u32,
                                                 channel: 1,
                                                 ..Default::default()
                                             }).collect(),
                                             cc_events: Vec::new(),
                                             color: "#3498db".to_string(), // default bright blue
                                             is_muted: false,
                                             is_looped: false,
                                             scale: None,
                                             chord_markers: Vec::new(),
                                             groove_template: None,
                                             pattern_id: None,
                                             tuning_steps: None,
                                             time_signature_num: None,
                                             time_signature_den: None,
                                             reference_clip_id: None,
                                         };
                                         clip
                                     };
                                     
                                     let mut tracks = tracks_t.lock().unwrap();
                                     if let Some(track) = tracks.get_mut(track_idx) {
                                         new_midi_clip.start_sample = track.clips.iter().find(|c| c.id == clip_id).map(|c| c.start_sample).unwrap_or(0);
                                         track.midi_clips.push(new_midi_clip);
                                         println!("VIBE: Audio-to-MIDI conversion complete for {}", clip_id);
                                     }
                                 }
                             }
                        });
                    }
                    AudioCommand::ExtractGroove(track_idx, clip_id) => {
                        let tracks_t = Arc::clone(&tracks_management);
                        let groove_pool_t = Arc::clone(&groove_pool_management);
                        let bpm_t = Arc::clone(&bpm_management);
                        
                        thread::spawn(move || {
                             let (path, name) = {
                                 let tracks = tracks_t.lock().unwrap();
                                 tracks.get(track_idx).and_then(|t| t.clips.iter().find(|c| c.id == clip_id).map(|c| (c.path.clone(), c.name.clone()))).unwrap_or((None, "clip".to_string()))
                             };

                             if let Some(p) = path {
                                 if let Ok(samples) = decode_file_to_vec(&p) {
                                     let bpm = *bpm_t.lock().unwrap();
                                     let detector = TransientDetector::default();
                                     let transients: Vec<u64> = detector.detect(&samples, 48000.0).into_iter().map(|t| t.position_samples).collect();
                                     
                                     if !transients.is_empty() {
                                         let mut pool = groove_pool_t.lock().unwrap();
                                         let template = pool.extract_from_transients(
                                             format!("{}_groove", name),
                                             transients,
                                             48000.0,
                                             bpm as f64,
                                             16
                                         );
                                         pool.templates.push(template);
                                         println!("VIBE: Extracted groove from {}", name);
                                     }
                                 }
                             }
                        });
                    }
                    AudioCommand::SetTimeSignature(num, den) => {
                        println!("VIBE: Time Signature changed to {}/{}", num, den);
                        // TODO: Update global metronome and grid state
                    }
                    AudioCommand::PasteTime(pos) => {
                        println!("VIBE: Paste Time at {}", pos);
                        // TODO: Implement Paste from Clipboard or Duplicate logic
                    }
                    AudioCommand::SetGlobalQuantization(division) => {
                        *global_quantization_t.lock().unwrap() = division;
                    }
                    AudioCommand::SetTempoAutomation(curve) => {
                        *tempo_automation_t.lock().unwrap() = curve;
                    }
                    AudioCommand::SetCompMode(idx, enabled) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(idx) {
                            track.comp_mode_enabled = enabled;
                        }
                    }
                    AudioCommand::SetActiveTake(track_idx, take_idx) => {
                        let mut tracks_lock = tracks_management.lock().unwrap();
                        if let Some(track) = tracks_lock.get_mut(track_idx) {
                            if take_idx < track.takes.len() {
                                // Save current clips as a new take or swap?
                                // Let's swap for simplicity.
                                let current_clips = std::mem::take(&mut track.clips);
                                track.clips = std::mem::replace(&mut track.takes[take_idx], current_clips);
                            }
                        }
                    }
                    AudioCommand::AddTakeFromSelection(idx, _start, _end) => {
                         if let Some(track) = tracks_management.lock().unwrap().get_mut(idx) {
                             // Clone current state as a take
                             track.takes.push(track.clips.clone());
                         }
                    }
                    AudioCommand::AddPlaylist(idx, name) => {
                        if let Some(track) = tracks_management.lock().unwrap().get_mut(idx) {
                            track.playlists.push(super::graph::TrackPlaylist {
                                name,
                                clips: Vec::new(),
                                midi_clips: Vec::new(),
                            });
                        }
                    }
                    AudioCommand::SetActivePlaylist(track_idx, playlist_idx) => {
                        let mut tracks_lock = tracks_management.lock().unwrap();
                        if let Some(track) = tracks_lock.get_mut(track_idx) {
                            if playlist_idx < track.playlists.len() {
                                // Save current state into current playlist
                                let old_idx = track.active_playlist_idx;
                                if old_idx < track.playlists.len() {
                                    track.playlists[old_idx].clips = track.clips.clone();
                                    track.playlists[old_idx].midi_clips = track.midi_clips.clone();
                                }
                                
                                // Load new playlist
                                track.clips = track.playlists[playlist_idx].clips.clone();
                                track.midi_clips = track.playlists[playlist_idx].midi_clips.clone();
                                track.active_playlist_idx = playlist_idx;
                            }
                        }
                    }
                    AudioCommand::DetectTransients(track_idx, clip_id) => {
                        let tracks_t = Arc::clone(&tracks_management);
                        thread::spawn(move || {
                            let path = {
                                let tracks = tracks_t.lock().unwrap();
                                tracks.get(track_idx).and_then(|t| t.clips.iter().find(|c| c.id == clip_id).and_then(|c| c.path.clone()))
                            };

                            if let Some(p) = path {
                                if let Ok(samples) = decode_file_to_vec(&p) {
                                     let detector = TransientDetector::default();
                                     let transients = detector.detect(&samples, 48000.0);
                                     let transient_stamps: Vec<u64> = transients.into_iter().map(|t| t.position_samples).collect();

                                     let mut tracks = tracks_t.lock().unwrap();
                                     if let Some(track) = tracks.get_mut(track_idx) {
                                         if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                             clip.transients = transient_stamps;
                                             println!("VIBE: Detected {} transients for clip {}", clip.transients.len(), clip.id);
                                         }
                                     }
                                }
                            }
                        });
                    }
                    AudioCommand::SetClipEnvelope(track_idx, clip_id, env_type, curve) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                match env_type.as_str() {
                                    "gain" => clip.gain_envelope = Some(curve),
                                    "pitch" => clip.pitch_envelope = Some(curve),
                                    "pan" => clip.pan_envelope = Some(curve),
                                    _ => {}
                                }
                            }
                        }
                    }
                    AudioCommand::AddVcaGroup(name) => {
                        let group = crate::engine::vca_group::VcaGroup::new(name);
                        vca_groups_management.lock().unwrap().push(group.clone());
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::AddVcaGroup(group));
                    }
                    AudioCommand::RemoveVcaGroup(id) => {
                        vca_groups_management.lock().unwrap().retain(|g| g.id != id);
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::RemoveVcaGroup(id));
                    }
                    AudioCommand::SetVcaGain(id, val) => {
                        if let Some(group) = vca_groups_management.lock().unwrap().iter_mut().find(|g| g.id == id) {
                            group.gain.set_value(val);
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetVcaGain(id, val));
                    }
                    AudioCommand::SetVcaMute(id, muted) => {
                        if let Some(group) = vca_groups_management.lock().unwrap().iter_mut().find(|g| g.id == id) {
                            group.is_muted = muted;
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetVcaMute(id, muted));
                    }
                    AudioCommand::SetVcaSolo(id, solo) => {
                        if let Some(group) = vca_groups_management.lock().unwrap().iter_mut().find(|g| g.id == id) {
                            group.is_solo = solo;
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetVcaSolo(id, solo));
                    }
                    AudioCommand::AddTrackToVca(vca_id, track_id) => {
                        if let Some(group) = vca_groups_management.lock().unwrap().iter_mut().find(|g| g.id == vca_id) {
                            group.add_track(track_id);
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::AddTrackToVca(vca_id, track_id));
                    }
                    AudioCommand::RemoveTrackFromVca(vca_id, track_id) => {
                        if let Some(group) = vca_groups_management.lock().unwrap().iter_mut().find(|g| g.id == vca_id) {
                            group.remove_track(track_id);
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::RemoveTrackFromVca(vca_id, track_id));
                    }
                    AudioCommand::SetMonitoringMode(idx, mode) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(idx) {
                            track.monitoring_mode = mode;
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetMonitoringMode(idx, mode));
                    }
                    AudioCommand::AddTrackSend(t_idx, target_id, gain, is_post) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        let send = super::graph::TrackSend {
                            id: Uuid::new_v4(),
                            target_id,
                            gain: super::graph::Parameter::new("Send Gain", gain, -60.0, 12.0),
                            is_post_fader: is_post,
                            is_muted: false,
                        };
                        if let Some(track) = tracks.get_mut(t_idx) {
                            track.sends.push(send.clone());
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::AddTrackSend(t_idx, send));
                    }
                    AudioCommand::RemoveTrackSend(t_idx, target_id) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(t_idx) {
                            track.sends.retain(|s| s.target_id != target_id);
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::RemoveTrackSend(t_idx, target_id));
                    }
                    AudioCommand::SetTrackSendGain(t_idx, target_id, gain) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(t_idx) {
                            if let Some(send) = track.sends.iter_mut().find(|s| s.target_id == target_id) {
                                send.gain.set_value(gain);
                            }
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetTrackSendGain(t_idx, target_id, gain));
                    }
                    AudioCommand::SetTrackSendMute(t_idx, target_id, muted) => {
                        let mut tracks = tracks_management.lock().unwrap();
                        if let Some(track) = tracks.get_mut(t_idx) {
                            if let Some(send) = track.sends.iter_mut().find(|s| s.target_id == target_id) {
                                send.is_muted = muted;
                            }
                        }
                        let _ = graph_prod_management.lock().unwrap().push(GraphCommand::SetTrackSendMute(t_idx, target_id, muted));
                    }
                    AudioCommand::SaveMixSnapshot(name) => {
                        println!("VIBE: Saving Mix Snapshot '{}'", name);
                    }
                    AudioCommand::LoadMixSnapshot(id) => {
                        println!("VIBE: Loading Mix Snapshot {}", id);
                    }
                }
                
                Self::commit_history(
                    &history_management,
                    &tracks_management,
                    &bpm_management,
                );
            }
        }).unwrap(); // thread::Builder::new().spawn(...)


        Self {
            command_tx: tx,
            tracks,
            busses,
            library,
            plugins,
            playhead,
            is_playing,
            is_recording,
            metronome_enabled,
            bpm,
            bpm_atomic,
            global_swing,
            _midi_conn: midi_conn,
            midi_map,
            midi_cc_lsb_cache,
            visualizer_prod: viz_prod_shared,
            cpu_load_micros,
            history,
            fades,
            velocity_engine,
            recorded_samples,
            autosave_path,
            plugin_path,
            engine_fx,
            master_limiter,
            summing_engine,
            neural_mapper,
            spectrum_analyzer,
            gpu_meter,
            midi_prod,
            param_prod,
            graph_prod,
            io_manager,
            audio_graph,
            buffer_pool,
            cached_execution_order,
            graph_dirty,
            hyper_streamer,
            hyper_pool,
            initialization_error,
            current_config,
            markers,
            loop_enabled,
            loop_start,
            loop_end,
            scene_manager,
            tempo_automation,
            global_quantization,
            kropelka_brain,
            groove_pool,
            humanization_engine,
            spectral_audio_tx,
            spectral_worker,
            mel_frame_rx,
            video_manager,
            vca_groups,
            hardware_input_prods: hw_prods_management,
            disk_writer,
        }
    }

    pub fn get_sample_rate(&self) -> f32 {
        48000.0 // Default for the engine
    }

    /// Audio stream initialization with all critical fixes integrated
    /// Issues fixed: #1 (Neural Mapper), #2 (Playhead sync), #3 (Buffer safety),
    /// #4 (Deadlock prevention), #5 (Preview cleanup), #6 (Pre-allocation),
    /// #7 (MIDI overflow), #8 (Error handling), #9 (CPU load), #10 (FX sync)
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)] // Called from audio thread setup
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
        visualizer_prod: Arc<Mutex<rtrb::Producer<f32>>>,
        gpu_meter: Arc<super::metering::GpuMeter>,
        cpu_load: Arc<AtomicU64>,
        neural_mapper: Arc<super::midi_mapping::NeuralMapper>,
        loop_enabled: Arc<AtomicBool>,
        loop_start: Arc<AtomicU64>,
        loop_end: Arc<AtomicU64>,
        hyper_pool: Arc<crate::engine::streamer::GlobalBufferPool>,
        hyper_streamer: Arc<crate::engine::streamer::WindowsAsyncStreamer>,
        spectral_audio_tx: crossbeam_channel::Sender<Vec<f32>>,
        hardware_input_prods: Arc<Mutex<Vec<rtrb::Producer<f32>>>>,
    ) -> Result<(cpal::Stream, Option<cpal::Stream>), String> {
        use cpal::traits::{DeviceTrait, StreamTrait};
        
        let output_channels = config.channels as usize;
        let sample_rate = config.sample_rate.0 as f64;
        let input_channels = input_config.channels as usize;
        
        // Issue #8 Fix: Build input stream with proper error handling
        let input_stream = if let Some(in_dev) = input_device {
            let _io_mgr = io_manager.clone();
            let rec_prod_clone = rec_prod.clone();
            let is_rec_clone = is_recording.clone();
            let _configured_input_channels = input_channels;
            let spectral_audio_tx_input = spectral_audio_tx.clone();
            
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
                    
                    
                    // Phase 5: Push to hardware input ring buffers for monitoring
                    if let Ok(mut prods) = hardware_input_prods.lock() {
                        for chunk in data.chunks_exact(input_channels) {
                            for (ch, &sample) in chunk.iter().enumerate() {
                                if ch < prods.len() {
                                    let _ = prods[ch].push(sample);
                                }
                            }
                        }
                    }

                    // Anti-Gravity: Send input to Spectral Engine (for Beatbox -> MIDI)
                    let _ = spectral_audio_tx_input.try_send(data.to_vec());
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
        let output_stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let callback_start = std::time::Instant::now();
                let frames_in_block = data.len() / output_channels;
                
                // Issue #3 Fix: Assert block size doesn't exceed pre-allocated buffer
                if frames_in_block > 4096 {
                    eprintln!("VIBE: Block size {} exceeds buffer limit 4096!", frames_in_block);
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
                let mut block_midi_scratch = [MidiEvent { sample_offset: 0, status: 0, data1: 0, data2: 0 }; MAX_MIDI_PER_BLOCK];
                let mut block_midi_len = 0;
                let mut graph_commands = Vec::with_capacity(4);
                let mut param_changes = Vec::with_capacity(8);
                
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
                    while let Ok(p) = cons.param_cons.pop() {
                        param_changes.push(p);
                    }
                    
                    // Process graph commands
                    while let Ok(cmd) = cons.graph_cons.pop() {
                        graph_commands.push(cmd);
                    }
                } // consumers lock dropped here
                
                let block_midi = &block_midi_scratch[..block_midi_len];

                let (mut internal_tracks, internal_busses, internal_vca_groups, mut preview_voice_opt, mut fx_opt, mut limiter_opt, mut local_master_buffer, hw_inputs) = {
                    let mut dsp = dsp_state.lock().unwrap();
                    
                    // Phase 5: Pull hardware inputs from ring buffers
                    {
                        let mut cons = consumers.lock().unwrap();
                        for (ch, hw_cons) in cons.hardware_input_cons.iter_mut().enumerate() {
                            if ch < dsp.hardware_inputs.len() {
                                let mut samples_read = 0;
                                while samples_read < frames_in_block {
                                    if let Ok(s) = hw_cons.pop() {
                                        dsp.hardware_inputs[ch][samples_read] = s;
                                        samples_read += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    
                    // Apply parameter changes
                    for p in param_changes {
                        dsp.set_parameter(p.id, p.value);
                    }
                    
                    // Apply graph commands BEFORE extracting tracks
                    for cmd in graph_commands {
                        use crate::engine::audio_commands::GraphCommand;
                        match cmd {
                            GraphCommand::AddTrack(track) => dsp.internal_tracks.push(track),
                            GraphCommand::RemoveTrack(index) => if index < dsp.internal_tracks.len() { dsp.internal_tracks.remove(index); },
                            GraphCommand::MoveTrack(from, to) => {
                                let len = dsp.internal_tracks.len();
                                if from < len && to < len && from != to {
                                    let track = dsp.internal_tracks.remove(from);
                                    dsp.internal_tracks.insert(to, track);
                                }
                            },
                            GraphCommand::SetTrackMute(idx, muted) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_muted = muted; },
                            GraphCommand::SetTrackSolo(idx, solo) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_solo = solo; },
                            GraphCommand::SetTrackPan(idx, pan) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.pan.set_value(pan); },
                            GraphCommand::SetTrackWidth(idx, width) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.width.set_value(width); },
                            GraphCommand::SetTrackDrive(idx, drive) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.input_drive.set_value(drive); },
                            GraphCommand::SetTrackPhaseInvert(idx, invert) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.phase_inverted = invert; },
                            GraphCommand::SetTrackArm(idx, armed) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_armed = armed; },
                            GraphCommand::SetTrackType(idx, t_type) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.track_type = t_type; },
                            GraphCommand::SetTrackParent(idx, parent_id) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.parent_id = parent_id; },
                            GraphCommand::SetTrackCollapsed(idx, collapsed) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_collapsed = collapsed; },
                            GraphCommand::SetTrackHeight(idx, height) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.height = height; },
                            GraphCommand::SetTrackFrozen(idx, frozen) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_frozen = frozen; },
                            GraphCommand::SetTrackDisabled(idx, disabled) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_disabled = disabled; },
                            GraphCommand::SetTrackAutomationArmed(idx, armed) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.is_automation_armed = armed; },
                            GraphCommand::SetTrackAutomationMode(idx, mode) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.automation_mode = mode; },
                            GraphCommand::SetTrackColor(idx, color) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.color = color; },
                            GraphCommand::AddProcessor(t_idx, proc) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { t.processors.push(proc); dsp.pdc_dirty = true; },
                            GraphCommand::SetEffectBypass(t_idx, p_idx, bypassed) => if let Some(track) = dsp.internal_tracks.get_mut(t_idx) { if let Some(proc) = track.processors.get_mut(p_idx) { proc.set_bypass(bypassed); } },
                            GraphCommand::MoveEffect(t_idx, from, to) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if from < t.processors.len() && to <= t.processors.len() { let proc = t.processors.remove(from); let target = to.min(t.processors.len()); t.processors.insert(target, proc); dsp.pdc_dirty = true; } },
                            GraphCommand::RemoveEffect(t_idx, idx) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if idx < t.processors.len() { t.processors.remove(idx); dsp.pdc_dirty = true; } },
                            GraphCommand::SetTrackInput(idx, alias_id, channels) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.input_alias_id = alias_id; if let Some(ch) = channels { t.input_channels = Some(ch); } },
                            GraphCommand::SetTrackSidechain(idx, source_id) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.sidechain_source_id = source_id; },
                            GraphCommand::InsertSilence(pos, len) => { for t in dsp.internal_tracks.iter_mut() { for c in t.clips.iter_mut() { if c.start_sample >= pos { c.start_sample += len; } } for c in t.midi_clips.iter_mut() { if c.start_sample >= pos { c.start_sample += len; } } } },
                            GraphCommand::DeleteTime(pos, len) => { for t in dsp.internal_tracks.iter_mut() { t.clips.retain(|c| c.start_sample + c.length_in_samples <= pos || c.start_sample >= pos + len); for c in t.clips.iter_mut() { if c.start_sample >= pos + len { c.start_sample -= len; } } t.midi_clips.retain(|c| c.start_sample + c.length_samples <= pos || c.start_sample >= pos + len); for c in t.midi_clips.iter_mut() { if c.start_sample >= pos + len { c.start_sample -= len; } } } },
                            GraphCommand::DuplicateTime(pos, len) => { for t in dsp.internal_tracks.iter_mut() { let mut a_clones = Vec::new(); for c in &t.clips { if c.start_sample >= pos && c.start_sample < pos + len { let mut cl = c.clone(); cl.id = Uuid::new_v4(); cl.start_sample += len; a_clones.push(cl); } } let mut m_clones = Vec::new(); for c in &t.midi_clips { if c.start_sample >= pos && c.start_sample < pos + len { let mut cl = c.clone(); cl.id = Uuid::new_v4(); cl.start_sample += len; m_clones.push(cl); } } for c in t.clips.iter_mut() { if c.start_sample >= pos + len { c.start_sample += len; } } for c in t.midi_clips.iter_mut() { if c.start_sample >= pos + len { c.start_sample += len; } } t.clips.extend(a_clones); t.midi_clips.extend(m_clones); } },
                            GraphCommand::TransposeMidiClip(t_idx, c_id, semitones) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(clip) = t.midi_clips.iter_mut().find(|c| c.id == c_id) { for n in &mut clip.notes { n.note = (n.note as i32 + semitones).clamp(0, 127) as u16; } } },
                            GraphCommand::DuplicateMidiNotes(t_idx, c_id, note_indices) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(clip) = t.midi_clips.iter_mut().find(|c| c.id == c_id) { let mut new_notes = Vec::new(); for &idx in &note_indices { if let Some(note) = clip.notes.get(idx) { new_notes.push(note.clone()); } } clip.notes.extend(new_notes); } },
                            GraphCommand::SetCrossfade(t_idx, c_a, c_b, duration) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { 
                                if let Some(clip) = t.clips.iter_mut().find(|c| c.id == c_a) { clip.fade_out_len = duration; } 
                                if let Some(clip) = t.clips.iter_mut().find(|c| c.id == c_b) { clip.fade_in_len = duration; } 
                            },
                            GraphCommand::LoadPreset(t_idx, p_idx, preset) => {
                                if let Some(track) = dsp.internal_tracks.get_mut(t_idx) {
                                    if let Some(proc) = track.processors.get_mut(p_idx) {
                                        if let Some(synth) = proc.as_any().downcast_mut::<crate::engine::synth::VOneSynth>() {
                                            synth.set_patch(&preset);
                                        }
                                    }
                                }
                            },
                            GraphCommand::UpdateModMatrix(t_idx, p_idx, slots) => {
                                if let Some(track) = dsp.internal_tracks.get_mut(t_idx) {
                                    if let Some(proc) = track.processors.get_mut(p_idx) {
                                        if let Some(synth) = proc.as_any().downcast_mut::<crate::engine::synth::VOneSynth>() {
                                            for (i, slot) in slots.iter().enumerate() {
                                                if i < 8 {
                                                    synth.mod_matrix[i] = *slot;
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                             GraphCommand::SetAudioClipPitch(t_idx, c_id, pitch) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(clip) = t.clips.iter_mut().find(|c| c.id == c_id) { clip.pitch_semitones = pitch; } },
                             GraphCommand::SetAudioClipWarpMode(t_idx, c_id, mode) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(clip) = t.clips.iter_mut().find(|c| c.id == c_id) { clip.warp_mode = mode; } },
                             GraphCommand::SetAudioClipWarp(t_idx, c_id, warped, speed) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(clip) = t.clips.iter_mut().find(|c| c.id == c_id) { clip.is_warped = warped; clip.playback_speed = speed; } },
                             GraphCommand::SetAudioClipGain(t_idx, c_id, gain) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(clip) = t.clips.iter_mut().find(|c| c.id == c_id) { clip.gain = gain; } },
                             GraphCommand::AddVcaGroup(group) => dsp.internal_vca_groups.push(group),
                             GraphCommand::RemoveVcaGroup(id) => dsp.internal_vca_groups.retain(|g| g.id != id),
                             GraphCommand::SetVcaGain(id, val) => if let Some(g) = dsp.internal_vca_groups.iter_mut().find(|g| g.id == id) { g.gain.set_value(val); },
                             GraphCommand::SetVcaMute(id, muted) => if let Some(g) = dsp.internal_vca_groups.iter_mut().find(|g| g.id == id) { g.is_muted = muted; },
                             GraphCommand::SetVcaSolo(id, solo) => if let Some(g) = dsp.internal_vca_groups.iter_mut().find(|g| g.id == id) { g.is_solo = solo; },
                             GraphCommand::AddTrackToVca(vca_id, track_id) => if let Some(g) = dsp.internal_vca_groups.iter_mut().find(|g| g.id == vca_id) { g.add_track(track_id); },
                             GraphCommand::RemoveTrackFromVca(vca_id, track_id) => if let Some(g) = dsp.internal_vca_groups.iter_mut().find(|g| g.id == vca_id) { g.remove_track(track_id); },
                             GraphCommand::SetMonitoringMode(idx, mode) => if let Some(t) = dsp.internal_tracks.get_mut(idx) { t.monitoring_mode = mode; },
                             GraphCommand::AddTrackSend(t_idx, send) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { t.sends.push(send); },
                             GraphCommand::RemoveTrackSend(t_idx, target_id) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { t.sends.retain(|s| s.target_id != target_id); },
                             GraphCommand::SetTrackSendGain(t_idx, target_id, gain) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(s) = t.sends.iter_mut().find(|s| s.target_id == target_id) { s.gain.set_value(gain); } },
                             GraphCommand::SetTrackSendMute(t_idx, target_id, muted) => if let Some(t) = dsp.internal_tracks.get_mut(t_idx) { if let Some(s) = t.sends.iter_mut().find(|s| s.target_id == target_id) { s.is_muted = muted; } },
                             _ => {}
                        }
                    }

                    // Issue #6 Fix: Clear pre-allocated hardware input buffers (no allocation!)
                    for buf in &mut dsp.hardware_inputs {
                        buf[..frames_in_block].fill(0.0);
                    }

                    // Phase 4: Integrated Interaction Loop (MIDI handling)
                    for event in block_midi {
                        let mpe_events = dsp.mpe_handler.process_event(event);
                        for mpe_ev in mpe_events {
                            for track in dsp.internal_tracks.iter_mut() {
                                for proc in track.processors.iter_mut() {
                                    proc.on_mpe_event(mpe_ev.clone()); 
                                }
                            }
                        }

                        // Neural Mapping (MIDI 1.0 CC)
                        if event.status & 0xF0 == 0xB0 {
                            let channel = event.status & 0x0F;
                            let cc = event.data1 as u8;
                            let value = (event.data2 >> 25) as u8;
                            
                            let mapping_res = neural_mapper.process_cc(0, channel, cc, value, |p_id| {
                                for track in dsp.internal_tracks.iter() {
                                    if track.volume.id == p_id { return track.volume.get_current_value(); }
                                    if track.pan.id == p_id { return track.pan.get_current_value(); }
                                }
                                0.5
                            });

                            if let crate::engine::midi_mapping::MappingResult::ParameterUpdates(updates) = mapping_res {
                                for (p_id, val) in updates {
                                    dsp.set_parameter(p_id, val);
                                }
                            }
                        }

                        for track in dsp.internal_tracks.iter_mut() {
                            for proc in track.processors.iter_mut() {
                                proc.on_midi_event(event.status, event.data1, event.data2);
                            }
                        }
                    }
                
                    // Macros / Launcher
                    let dsp_ref = &mut *dsp;
                    let macro_engine = &dsp_ref.macro_engine;
                    for track in dsp_ref.internal_tracks.iter_mut() {
                        let mut params = track.get_all_parameters();
                        macro_engine.apply_to_params(&mut params);
                    }
                    let _ = dsp.clip_launcher.process(current_playhead, frames_in_block);
                    
                    // --- PDC (Plugin Delay Compensation) Loop Synchronization ---
                    // Detect if any plugin requested a latency update (dynamic PDC) 
                    // or if the graph was modified (Add/Remove Processor).
                    let mut needs_recalc = dsp.pdc_dirty;
                    for track in dsp.internal_tracks.iter_mut() {
                        for proc in track.processors.iter_mut() {
                            if proc.needs_pdc_recalc() {
                                needs_recalc = true;
                                proc.reset_pdc_recalc();
                            }
                        }
                    }
                    if needs_recalc {
                        crate::engine::pdc::PdcManager::recalculate_project_pdc(&mut dsp.internal_tracks);
                        dsp.pdc_dirty = false;
                    }

                    (
                        std::mem::take(&mut dsp.internal_tracks),
                        std::mem::take(&mut dsp.internal_busses),
                        std::mem::take(&mut dsp.internal_vca_groups),
                        dsp.preview_voice.take(),
                        dsp.internal_engine_fx.take(),
                        dsp.internal_master_limiter.take(),
                        std::mem::replace(&mut dsp.master_buffer, AudioBuffer::new()),
                        std::mem::take(&mut dsp.hardware_inputs),
                    )
                }; // dsp_state lock dropped here
                
                let mut master_chans_vec = vec![vec![0.0; frames_in_block]; output_channels];
                let mut master_chans: Vec<&mut [f64]> = master_chans_vec.iter_mut().map(|v| v.as_mut_slice()).collect();
                
                let mut next_playhead = current_playhead + frames_in_block as u64;

                if is_play {
                    let loop_en = loop_enabled.load(Ordering::Acquire);
                    let loop_st = loop_start.load(Ordering::Acquire);
                    let loop_ed = loop_end.load(Ordering::Acquire);

                    if loop_en && current_playhead < loop_ed && current_playhead + frames_in_block as u64 > loop_ed {
                        let frames_before = (loop_ed - current_playhead) as usize;
                        let mut first_chans: Vec<&mut [f64]> = master_chans.iter_mut().map(|c| &mut c[..frames_before]).collect();
                        summing.process_parallel(&mut internal_tracks, &mut first_chans, &internal_vca_groups, sample_rate, current_bpm as f64, current_playhead, &fades, block_midi, &hyper_pool, &hyper_streamer, false, &hw_inputs, is_play);
                        
                        let mut second_chans: Vec<&mut [f64]> = master_chans.iter_mut().map(|c| &mut c[frames_before..]).collect();
                        summing.process_parallel(&mut internal_tracks, &mut second_chans, &internal_vca_groups, sample_rate, current_bpm as f64, loop_st, &fades, block_midi, &hyper_pool, &hyper_streamer, false, &hw_inputs, is_play);
                        next_playhead = loop_st + (frames_in_block - frames_before) as u64;
                    } else {
                        let jumped_start = if loop_en && current_playhead >= loop_ed {
                             loop_st + (current_playhead - loop_ed)
                        } else {
                             current_playhead
                        };
                        summing.process_parallel(&mut internal_tracks, &mut master_chans, &internal_vca_groups, sample_rate, current_bpm as f64, jumped_start, &fades, block_midi, &hyper_pool, &hyper_streamer, false, &hw_inputs, is_play);
                        next_playhead = jumped_start + frames_in_block as u64;
                    }
                }
                
                // Metronome
                if is_play && metro_on {
                    let samples_per_beat = (sample_rate * 60.0 / current_bpm as f64) as u64;
                    for i in 0..frames_in_block {
                        let sample_pos = current_playhead + i as u64;
                        if sample_pos % samples_per_beat < 1000 {
                            let click = (1.0 - (sample_pos % samples_per_beat) as f32 / 1000.0) * 0.4;
                            for c in 0..output_channels.min(2) { master_chans[c][i] += click as f64; }
                        }
                    }
                }
                
                // Preview Voice Cleanup
                if let Some(ref mut voice) = preview_voice_opt {
                    if voice.is_playing && current_playhead >= voice.start_sample {
                        for i in 0..frames_in_block {
                            if voice.position + 1 < voice.data.len() {
                                master_chans[0][i] += voice.data[voice.position] as f64 * voice.volume as f64;
                                if output_channels > 1 { master_chans[1][i] += voice.data[voice.position + 1] as f64 * voice.volume as f64; }
                                voice.position += 2;
                            } else {
                                voice.is_playing = false;
                                break;
                            }
                        }
                    } else if !voice.is_playing {
                        preview_voice_opt = None;
                    }
                }
                
                // Master FX (Lock-Free)
                local_master_buffer.frames = frames_in_block;
                local_master_buffer.num_channels = output_channels.min(MAX_CHANNELS);
                for c in 0..local_master_buffer.num_channels {
                    local_master_buffer.channels_data[c][..frames_in_block].copy_from_slice(master_chans[c]);
                }

                let context = ProcessingContext { sample_rate, playhead: current_playhead, sidechain: None };
                if let Some(ref mut fx) = fx_opt { fx.process(&mut local_master_buffer, &context); }
                if let Some(ref mut limiter) = limiter_opt { limiter.process(&mut local_master_buffer, &context); }
                
                for c in 0..local_master_buffer.num_channels {
                    master_chans[c].copy_from_slice(&local_master_buffer.channels_data[c][..frames_in_block]);
                }
                
                // PHASE 5: Metering & Copy
                gpu_meter.update_multichannel(&master_chans.iter().map(|c| c.as_ref()).collect::<Vec<_>>());
                if let Ok(mut viz) = visualizer_prod.try_lock() {
                    for &s in master_chans[0].iter() { let _ = viz.push(s as f32); }
                }
                let _ = spectral_audio_tx.try_send(master_chans[0].iter().map(|&s| s as f32).collect());

                for (i, frame) in data.chunks_mut(output_channels).enumerate().take(frames_in_block) {
                    for c in 0..output_channels {
                        frame[c] = master_chans[c][i].clamp(-1.0, 1.0) as f32;
                    }
                }
                
                // PHASE 6: Finalize
                if is_playing.load(Ordering::Acquire) {
                    let _ = playhead.compare_exchange(current_playhead, next_playhead, Ordering::Release, Ordering::Relaxed);
                }
                
                // Return state
                {
                    let mut dsp = dsp_state.lock().unwrap();
                    dsp.internal_tracks = internal_tracks;
                    dsp.internal_busses = internal_busses;
                    dsp.internal_vca_groups = internal_vca_groups;
                    dsp.preview_voice = preview_voice_opt;
                    dsp.internal_engine_fx = fx_opt;
                    dsp.internal_master_limiter = limiter_opt;
                    dsp.master_buffer = local_master_buffer;
                    dsp.hardware_inputs = hw_inputs;
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
        ).map_err(|e| format!("Failed to build output stream: {}", e))?;
        
        // Issue #8 Fix: Start stream with error handling
        output_stream.play().map_err(|e| format!("Failed to play output stream: {}", e))?;
        
        Ok((output_stream, input_stream))
    }

    fn commit_history(
        history: &Arc<Mutex<HistoryManager>>,
        tracks: &Arc<Mutex<Vec<Track>>>,
        bpm: &Arc<Mutex<f32>>,
    ) {
        let mut t_list = tracks.lock().unwrap();
        let track_snapshots: Vec<super::persistence::TrackSnapshot> = t_list
            .iter_mut()
            .map(|t| super::persistence::TrackSnapshot {
                id: t.id.to_string(),
                name: t.name.clone(),
                volume: super::persistence::ParameterSnapshot {
                    id: t.volume.id.to_string(),
                    name: t.volume.name.clone(),
                    value: t.volume.get_current_value(),
                    automation: t.volume.curve.load().knots.clone(),
                },
                pan: super::persistence::ParameterSnapshot {
                    id: t.pan.id.to_string(),
                    name: t.pan.name.clone(),
                    value: t.pan.get_current_value(),
                    automation: t.pan.curve.load().knots.clone(),
                },
                width: super::persistence::ParameterSnapshot {
                    id: t.width.id.to_string(),
                    name: t.width.name.clone(),
                    value: t.width.get_current_value(),
                    automation: t.width.curve.load().knots.clone(),
                },
                input_drive: super::persistence::ParameterSnapshot {
                    id: t.input_drive.id.to_string(),
                    name: t.input_drive.name.clone(),
                    value: t.input_drive.get_current_value(),
                    automation: t.input_drive.curve.load().knots.clone(),
                },
                muted: t.is_muted,
                solo: t.is_solo,
                is_armed: t.is_armed,
                phase_inverted: t.phase_inverted,
                color: t.color.clone(),
                clips: t
                    .clips
                    .iter()
                    .map(|c| super::persistence::ClipSnapshot {
                        id: c.id.to_string(),
                        audio_path: c.name.clone(), // Use name as path for now
                        start_sample: c.start_sample,
                        duration_samples: c.length_in_samples,
                        offset_in_data: c.offset_in_data,
                        fade_in_len: c.fade_in_len,
                        fade_out_len: c.fade_out_len,
                        fade_in_type: c.fade_in_type,
                        fade_out_type: c.fade_out_type,
                    })
                    .collect(),
                plugins: t
                    .processors
                    .iter_mut()
                    .map(|p| super::persistence::PluginSnapshot {
                        id: p.id().to_string(),
                        plugin_path: p.name(),
                        state_blob: p.get_state(),
                        parameters: p
                            .get_parameters()
                            .iter()
                            .map(|param| super::persistence::ParameterSnapshot {
                                id: param.id.to_string(),
                                name: param.name.clone(),
                                value: param.get_current_value(),
                                automation: param.curve.load().knots.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
                input_alias_id: t.input_alias_id.map(|id| id.to_string()),
                midi_clips: t
                    .midi_clips
                    .iter()
                    .map(|mc| super::persistence::MidiClipSnapshot {
                        id: mc.id.to_string(),
                        name: mc.name.clone(),
                        start_sample: mc.start_sample,
                        length_samples: mc.length_samples,
                        color: mc.color.clone(),
                        is_muted: mc.is_muted,
                        is_looped: mc.is_looped,
                        scale: mc.scale.clone(),
                        chord_markers: mc.chord_markers.clone(),
                        groove_template: mc.groove_template.clone(),
                        pattern_id: mc.pattern_id.clone(),
                        tuning_steps: mc.tuning_steps,
                        time_signature_num: mc.time_signature_num,
                        time_signature_den: mc.time_signature_den,
                        notes: mc
                            .notes
                            .iter()
                            .map(|n| super::persistence::MidiNoteSnapshot {
                                start_sample: n.start_sample,
                                length_samples: n.length_samples,
                                note: n.note,
                                velocity: n.velocity,
                                channel: n.channel,
                                pitch_bend: n.pitch_bend,
                                pressure: n.pressure,
                                timbre: n.timbre,
                                probability: n.probability,
                                velocity_random: n.velocity_random,
                                timing_random: n.timing_random,
                            })
                            .collect(),
                        cc_events: mc
                            .cc_events
                            .iter()
                            .map(|cc| super::persistence::MidiCCSnapshot {
                                sample: cc.sample,
                                cc_number: cc.cc_number,
                                value: cc.value,
                                channel: cc.channel,
                            })
                            .collect(),
                    })
                    .collect(),
                quantize_division: t.quantize_division,
            })
            .collect();

        let snapshot = DagSnapshot {
            id: Uuid::new_v4(),
            timestamp: get_micros(),
            action_name: "Manual Snapshot".to_string(),
            tracks: track_snapshots,
            bpm: *bpm.lock().unwrap(),
            parent_id: None,
        };

        history.lock().unwrap().commit(snapshot);
    }

    pub fn get_cpu_load(&self) -> f32 {
        // Returns percentage (micros / 21333us for 48kHz buffer of 1024)
        // rough estimate assuming ~21ms buffer budget
        let micros = self.cpu_load_micros.load(Ordering::Relaxed);
        let budget_us = 21333.0;
        (micros as f32 / budget_us) * 100.0
    }

    pub fn get_memory_usage(&self) -> f32 {
        // Dummy implementation for now - return a realistic %
        // In a real version, we'd use sysinfo or windows-sys
        let mut sum = 0.0;
        if let Ok(tracks) = self.tracks.lock() {
            sum += tracks.len() as f32 * 5.0; // 5% per track
        }
        sum %= 100.0;
        if sum < 10.0 { sum = 12.5; }
        sum
    }

    pub fn get_history_graph(&self) -> Vec<(String, Option<String>, String)> {
        self.history
            .lock()
            .unwrap()
            .get_history_graph()
            .into_iter()
            .map(|(id, parent, name)| (id.to_string(), parent.map(|p| p.to_string()), name))
            .collect()
    }

    pub fn get_midi_clips_for_track(&self, track_idx: usize) -> Vec<super::graph::MidiClip> {
        let tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get(track_idx) {
            return track.midi_clips.clone();
        }
        Vec::new()
    }

    pub fn get_current_node(&self) -> String {
        self.history.lock().unwrap().current_node.to_string()
    }

    pub fn get_branches(&self) -> HashMap<String, String> {
        self.history
            .lock()
            .unwrap()
            .branches
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect()
    }

    pub fn play(&self) -> Result<(), String> {
        self.is_playing.store(true, Ordering::Release);
        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.is_playing.store(false, Ordering::Release);
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        println!("VIBE: AudioEngine::stop() called");
        self.is_playing.store(false, Ordering::Release);
        self.playhead.store(0, Ordering::Release);
        Ok(())
    }

    pub fn toggle_record(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::ToggleRecord)
            .map_err(|e| e.to_string())
    }

    pub fn set_automation_interpolation(
        &self,
        param_id: String,
        interp_type: crate::engine::automation::InterpolationType,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::SetAutomationInterpolation(uuid, interp_type))
            .map_err(|e| e.to_string())
    }

    pub fn set_automation_layer(
        &self,
        param_id: String,
        layer: crate::engine::automation::AutomationLayer,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::SetAutomationLayer(uuid, layer))
            .map_err(|e| e.to_string())
    }

    pub fn start_midi_learn(&self, param_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::StartMidiLearn(uuid))
            .map_err(|e| e.to_string())
    }

    pub fn get_midi_bindings(&self) -> Vec<super::midi_mapping::MidiBinding> {
        self.neural_mapper.get_bindings()
    }

    pub fn remove_midi_binding(&self, binding_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&binding_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::RemoveBinding(uuid))
            .map_err(|e| e.to_string())
    }

    pub fn get_scope_data(&self) -> (Vec<f32>, Vec<f32>) {
        if let Ok(spectrum) = self.spectrum_analyzer.lock() {
            spectrum.get_scope_data()
        } else {
            (vec![], vec![])
        }
    }

    pub fn get_playhead(&self) -> u64 {
        self.playhead.load(Ordering::Acquire)
    }

    pub fn is_loop_enabled(&self) -> bool {
        self.loop_enabled.load(Ordering::Acquire)
    }

    pub fn get_loop_range(&self) -> (u64, u64) {
        (
            self.loop_start.load(Ordering::Acquire),
            self.loop_end.load(Ordering::Acquire),
        )
    }

    pub fn set_loop_enabled(&self, enabled: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetLoopEnabled(enabled))
            .map_err(|e| e.to_string())
    }

    pub fn set_loop_range(&self, start: u64, end: u64) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetLoopRange(start, end))
            .map_err(|e| e.to_string())
    }

    pub fn set_playhead(&self, pos: u64) -> Result<(), String> {
        println!("VIBE: AudioEngine::set_playhead({}) called", pos);
        self.playhead.store(pos, Ordering::Release);
        Ok(())
    }

    pub fn get_waveform_data(&self, clip_id: String, lod_level: u8) -> Result<Vec<u8>, String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        let lib = self.library.lock().unwrap();
        let clip = lib.iter().find(|c| c.id == uuid).ok_or("Clip not found")?;

        if let Some(cache) = &clip.waveform_cache {
            // Map LOD level: 1 -> 0, 2 -> 1, 3 -> 2 in array
            if lod_level == 0 || lod_level > 3 {
                return Err("Invalid LOD level (1-3)".to_string());
            }
            let index = (lod_level - 1) as usize;
            if index >= cache.lods.len() {
                return Err("LOD not available".to_string());
            }

            let lod = &cache.lods[index];

            // Serialize raw bytes
            let ptr = lod.data.as_ptr() as *const u8;
            let len =
                lod.data.len() * std::mem::size_of::<crate::engine::waveform::WaveformPoint>();
            // Safety: WaveformPoint is repr(C) and contains only f16 (transparent/pod-like)
            let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
            Ok(bytes.to_vec())
        } else {
            Err("Cache not ready (Generating...)".to_string())
        }
    }

    /// Get raw spectrum data for a track (Binary Blob for UI)
    pub fn get_analyzer_data(&self, track_idx: usize) -> Vec<u8> {
        let tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get(track_idx) {
            let data = track.spectrum_analyzer.get_data(); // Vec<f32>
                                                           // Convert to Vec<u8> (Safety: Logic matches prompt requirement for zero-copy-ish transfer)
            unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4).to_vec()
            }
        } else {
            // Return empty if track not found
            vec![]
        }
    }

    pub fn export_project(
        &self,
        config: crate::engine::render_engine::RenderConfig,
    ) -> Result<crossbeam_channel::Receiver<crate::engine::render_engine::RenderStatus>, String>
    {
        // 1. Pause Playback to ensure no file handle conflicts (though RenderEngine opens new handles)
        self.stop()?;

        // 2. Clone Snapshot of State
        // We need a deep clone of tracks and graph.
        // Graph is currently empty/test, real logic is in tracks map for Phase 4.1
        let tracks_snapshot: Vec<Track> = {
            let tracks = self.tracks.lock().unwrap();
            tracks.iter().map(|t| t.clone_as_dummy()).collect() // clone_as_dummy actually deep clones clips/params
        };

        let graph_snapshot = {
            let g = self.audio_graph.lock().unwrap();
            g.clone()
        };

        // 3. Setup Progress Channel
        let (tx, rx) = crossbeam_channel::unbounded();

        // 4. Instantiate Render Engine
        let mut render_engine = crate::engine::render_engine::RenderEngine::new(
            graph_snapshot,
            tracks_snapshot,
            Arc::clone(&self.hyper_pool),
            Arc::clone(&self.hyper_streamer),
            Arc::clone(&self.fades),
            config,
            tx,
        );

        // 5. Run in dedicated thread
        std::thread::spawn(move || {
            render_engine.render();
        });

        Ok(rx)
    }

    pub fn get_track_levels(&self) -> Vec<super::graph::TrackLevel> {
        let tracks = self.tracks.lock().unwrap();
        tracks
            .iter()
            .map(|t| {
                let lufs = t.meter.get_lufs_full();
                let mut peaks = Vec::new();
                let mut rms = Vec::new();
                for c in 0..t.output_buffer.num_channels {
                    peaks.push(t.meter.get_peak_db(c) as f32);
                    rms.push(t.meter.get_rms_db(c) as f32);
                }
                
                super::graph::TrackLevel {
                    id: t.id.to_string(),
                    peaks,
                    rms,
                    true_peaks: vec![lufs.true_peak_l as f32, lufs.true_peak_r as f32],
                    lufs_momentary: lufs.momentary as f32,
                }
            })
            .collect()
    }

    pub fn set_bpm(&self, bpm: f32) -> Result<(), String> {
        if (1.0..=999.0).contains(&bpm) {
            *self.bpm.lock().unwrap() = bpm;
            self.bpm_atomic
                .store(bpm.to_bits() as u64, Ordering::Release);
            Ok(())
        } else {
            Err(format!("Invalid BPM requested: {}", bpm))
        }
    }

    pub fn get_bpm(&self) -> f32 {
        *self.bpm.lock().unwrap()
    }

    pub fn get_global_swing(&self) -> f32 {
        f32::from_bits(self.global_swing.load(Ordering::Relaxed) as u32)
    }

    /// Generic command sender for MIDI and other commands
    pub fn send_command(&self, cmd: AudioCommand) -> Result<(), String> {
        self.command_tx.send(cmd).map_err(|e| e.to_string())
    }

    pub fn check_initialization(&self) -> Result<(), String> {
        let err_guard = self.initialization_error.lock().unwrap();
        if let Some(e) = &*err_guard {
            return Err(e.clone());
        }
        Ok(())
    }

    pub fn add_track(&self, name: String) -> Result<(), String> {
        self.check_initialization()?;
        let track = Track::new(name);
        self.command_tx
            .send(AudioCommand::AddTrack(track))
            .map_err(|e| e.to_string())
    }

    pub fn init_default_project(&self) -> Result<(), String> {
        let mut tracks = self.tracks.lock().unwrap();
        tracks.clear();
        
        // Add 4 default tracks for a fresh start
        for i in 1..=4 {
            tracks.push(Track::new(format!("Track {}", i)));
        }

        self.playhead.store(0, Ordering::SeqCst);
        *self.bpm.lock().unwrap() = 120.0;
        self.bpm_atomic.store(120.0f32.to_bits() as u64, Ordering::SeqCst);
        
        // Clear graph
        if let Ok(mut graph) = self.audio_graph.lock() {
            *graph = crate::engine::audio_graph::AudioGraph::new();
        }
        self.graph_dirty.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub fn new_project(&self) -> Result<(), String> {
        self.init_default_project()?;
        // Clear history too
        if let Ok(mut hist) = self.history.lock() {
            *hist = super::history::HistoryManager::new(DagSnapshot {
                id: Uuid::new_v4(),
                timestamp: get_micros(),
                action_name: "New Project Initial State".to_string(),
                tracks: Vec::new(),
                bpm: 120.0,
                parent_id: None,
            });
        }
        self.send_command(AudioCommand::NewProject)
    }
    pub fn add_clip_to_library(&self, clip: AudioClip) -> Result<super::graph::AudioClipInfo, String> {
        let info = super::graph::AudioClipInfo {
            id: clip.id.to_string(),
            name: clip.name.clone(),
            path: clip.path.clone(),
            start_sample: 0,
            duration_samples: clip.length_in_samples,
            peaks: clip.peaks.clone(),
            offset_in_data: 0,
            fade_in_len: clip.fade_in_len,
            fade_out_len: clip.fade_out_len,
            fade_in_type: format!("{:?}", clip.fade_in_type),
            fade_out_type: format!("{:?}", clip.fade_out_type),
            gain: clip.gain,
            pitch_semitones: clip.pitch_semitones,
            playback_speed: clip.playback_speed,
            is_warped: clip.is_warped,
            has_gain_envelope: clip.gain_envelope.is_some(),
            has_pitch_envelope: clip.pitch_envelope.is_some(),
            transient_count: clip.transients.len(),
            color: clip.color.clone(),
        };
        self.library.lock().unwrap().push(clip);
        Ok(info)
    }
    pub fn set_volume(&self, index: usize, volume: f64) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackVolume(index, volume))
            .map_err(|e| e.to_string())
    }

    pub fn set_mute(&self, index: usize, muted: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackMute(index, muted))
            .map_err(|e| e.to_string())
    }

    pub fn set_solo(&self, index: usize, solo: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackSolo(index, solo))
            .map_err(|e| e.to_string())
    }

    pub fn set_pan(&self, index: usize, val: f64) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackPan(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn set_track_drive(&self, index: usize, val: f64) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackDrive(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn set_width(&self, index: usize, val: f64) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackWidth(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn set_phase_invert(&self, index: usize, val: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackPhaseInvert(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn set_arm(&self, index: usize, val: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackArm(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn set_track_input(&self, index: usize, val: Option<String>) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackInput(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn set_track_sidechain(&self, index: usize, source_id: Option<String>) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackSidechain(index, source_id))
            .map_err(|e| e.to_string())
    }

    pub fn set_track_output(&self, index: usize, val: Option<String>) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetTrackOutput(index, val))
            .map_err(|e| e.to_string())
    }

    pub fn scan_plugins(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::ScanPlugins)
            .map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn get_plugins(&self) -> Vec<super::scanner::PluginMetadata> {
        self.plugins.lock().unwrap().clone()
    }

    pub fn add_plugin_to_track(&self, index: usize, path: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::AddPluginToTrack(index, PathBuf::from(path)))
            .map_err(|e| e.to_string())
    }

    pub fn import_plugin(&self, path: String) -> Result<(), String> {
        let src_path = PathBuf::from(&path);
        if !src_path.exists() {
            return Err("Plugin file not found".to_string());
        }

        let file_name = src_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_string_lossy()
            .to_string();

        let dest_path = self.plugin_path.join(file_name);

        // Create plugins dir if it doesn't exist (it should though)
        std::fs::create_dir_all(&self.plugin_path).map_err(|e| e.to_string())?;

        // Copy file
        std::fs::copy(&src_path, &dest_path).map_err(|e| e.to_string())?;

        // Trigger scan
        self.scan_plugins()
    }

    // POINT 10: Get spectrum data for UI

    // POINT 9: Get master meters for UI
    // POINT 9: Get master meters for UI
    pub fn get_master_meters_db(&self) -> (f64, f64, f64, f64, super::metering::LufsResults) {
        let (peak_l, peak_r) = (self.gpu_meter.get_peak_db(0), self.gpu_meter.get_peak_db(1));
        let (rms_l, rms_r) = (self.gpu_meter.get_rms_db(0), self.gpu_meter.get_rms_db(1));
        let lufs = self.gpu_meter.get_lufs_full();
        (peak_l, peak_r, rms_l, rms_r, lufs)
    }

    pub fn rename_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        new_name: String,
        is_midi: bool,
    ) -> Result<(), String> {
        let mut tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get_mut(track_idx) {
            if is_midi {
                if let Some(clip) = track
                    .midi_clips
                    .iter_mut()
                    .find(|c| c.id.to_string() == clip_id)
                {
                    clip.name = new_name;
                    return Ok(());
                }
            } else if let Some(clip) = track.clips.iter_mut().find(|c| c.id.to_string() == clip_id)
            {
                clip.name = new_name;
                return Ok(());
            }
        }
        Err("Clip not found".to_string())
    }

    pub fn set_clip_gain(&self, track_idx: usize, clip_id: String, gain: f32) -> Result<(), String> {
        let mut tracks = self.tracks.lock().unwrap();
        let track = tracks.get_mut(track_idx).ok_or("Track not found")?;
        let clip = track
            .clips
            .iter_mut()
            .find(|c| c.id.to_string() == clip_id)
            .ok_or("Clip not found")?;
        clip.gain = gain;
        Ok(())
    }

    pub fn normalize_clip(&self, track_idx: usize, clip_id: String) -> Result<(), String> {
        let mut tracks = self.tracks.lock().unwrap();
        let track = tracks.get_mut(track_idx).ok_or("Track not found")?;
        let clip = track
            .clips
            .iter_mut()
            .find(|c| c.id.to_string() == clip_id)
            .ok_or("Clip not found")?;

        let mut max_peak = 0.0f32;
        // Search all peak levels (usually 0 is lowest zoom, last is highest zoom)
        for level in &clip.peaks {
            for &p in level {
                if p > max_peak {
                    max_peak = p;
                }
            }
        }

        if max_peak > 0.0 {
            clip.gain = 1.0 / max_peak;
        }
        Ok(())
    }

    pub fn duplicate_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        is_midi: bool,
    ) -> Result<(), String> {
        let mut tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get_mut(track_idx) {
            if is_midi {
                if let Some(pos) = track
                    .midi_clips
                    .iter()
                    .position(|c| c.id.to_string() == clip_id)
                {
                    let mut new_clip = track.midi_clips[pos].clone();
                    new_clip.id = Uuid::new_v4();
                    new_clip.start_sample += new_clip.length_samples;
                    track.midi_clips.push(new_clip);
                    return Ok(());
                }
            } else if let Some(pos) = track.clips.iter().position(|c| c.id.to_string() == clip_id) {
                let mut new_clip = track.clips[pos].clone();
                new_clip.id = Uuid::new_v4();
                new_clip.start_sample += new_clip.length_in_samples;
                track.clips.push(new_clip);
                return Ok(());
            }
        }
        Err("Clip not found".to_string())
    }

    pub fn reverse_audio_clip(&self, track_idx: usize, clip_id: String) -> Result<(), String> {
        // 1. Find Clip and its Path
        let (path, _is_midi) = {
            let tracks = self.tracks.lock().unwrap();
            let track = tracks.get(track_idx).ok_or("Track not found")?;
            let clip = track
                .clips
                .iter()
                .find(|c| c.id.to_string() == clip_id)
                .ok_or("Clip not found")?;
            (clip.path.clone().ok_or("Clip has no file path")?, false)
        };

        // 2. Read FULL Audio (ignoring head limit)
        let (samples, channels, sample_rate) = read_full_samples(path)?;

        // 3. Reverse Logic (Stereo Aware)
        let channels = channels as usize;
        if channels == 0 {
            return Err("No channels found".to_string());
        }

        // We need to reverse chunks of 'channels' size
        // e.g. [L1, R1, L2, R2] -> [L2, R2, L1, R1]
        // This effectively reverses time but keeps L/R phase alignment (L matches L)

        let frame_count = samples.len() / channels;
        let mut reversed_samples = vec![0.0; samples.len()];

        for i in 0..frame_count {
            let src_idx = i * channels;
            let dest_idx = (frame_count - 1 - i) * channels;
            for c in 0..channels {
                reversed_samples[dest_idx + c] = samples[src_idx + c];
            }
        }

        // 4. Write to New File
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let new_path_str = format!("C:\\Users\\brigh\\Desktop\\VIBE_Reversed_{}.wav", timestamp);
        let spec = hound::WavSpec {
            channels: channels as u16,
            sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer =
            hound::WavWriter::create(&new_path_str, spec).map_err(|e| e.to_string())?;

        // Write as i24
        for s in reversed_samples {
            let scaled = (s.clamp(-1.0, 1.0) * 8_388_607.0) as i32;
            writer.write_sample(scaled).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;

        // 5. Import and Replace
        let new_clip_info = self.import_file_internal(PathBuf::from(new_path_str))?;
        let new_template_uuid = Uuid::parse_str(&new_clip_info.id).map_err(|e| e.to_string())?;

        let template = {
            let lib = self.library.lock().unwrap();
            lib.iter()
                .find(|c| c.id == new_template_uuid)
                .cloned()
                .ok_or("Imported clip not found in library")?
        };

        {
            let mut tracks = self.tracks.lock().unwrap();
            let track = tracks.get_mut(track_idx).ok_or("Track not found")?;
            let old_clip_idx = track
                .clips
                .iter()
                .position(|c| c.id.to_string() == clip_id)
                .ok_or("Clip not found")?;
            let old_start = track.clips[old_clip_idx].start_sample;
            // Replace with new
            let mut new_clip = template.clone();
            new_clip.id = Uuid::new_v4();
            new_clip.start_sample = old_start;
            track.clips[old_clip_idx] = new_clip;
        }

        // 6. Notify Render Thread
        self.send_command(AudioCommand::ReverseClip(track_idx, Uuid::parse_str(&clip_id).unwrap_or(Uuid::nil())))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn reverse_audio_clip_glue(&self, track_idx: usize, clip_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.send_command(AudioCommand::ReverseClip(track_idx, uuid))
    }

    pub fn insert_silence(&self, pos: u64, len: u64) -> Result<(), String> {
        self.send_command(AudioCommand::InsertSilence(pos, len))
    }

    pub fn paste_time(&self, pos: u64) -> Result<(), String> {
        self.send_command(AudioCommand::PasteTime(pos))
    }

    pub fn delete_time(&self, pos: u64, len: u64) -> Result<(), String> {
        self.send_command(AudioCommand::DeleteTime(pos, len))
    }

    pub fn duplicate_time(&self, pos: u64, len: u64) -> Result<(), String> {
        self.send_command(AudioCommand::DuplicateTime(pos, len))
    }

    pub fn add_marker(&self, label: String, pos: u64, color: String) -> Result<(), String> {
        self.send_command(AudioCommand::AddMarker(label, pos, color))
    }

    pub fn remove_marker(&self, id: Uuid) -> Result<(), String> {
        self.send_command(AudioCommand::RemoveMarker(id))
    }

    pub fn get_markers(&self) -> Vec<super::graph::Marker> {
        self.markers.lock().unwrap().clone()
    }

    pub fn rename_track(&self, idx: usize, name: String) -> Result<(), String> {
        self.send_command(AudioCommand::RenameTrack(idx, name))
    }

    pub fn duplicate_track(&self, idx: usize) -> Result<(), String> {
        self.send_command(AudioCommand::DuplicateTrack(idx))
    }

    pub fn remove_track(&self, idx: usize) -> Result<(), String> {
        self.send_command(AudioCommand::RemoveTrack(idx))
    }

    pub fn transpose_midi_clip(&self, track_idx: usize, clip_id: String, semitones: i32) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.send_command(AudioCommand::TransposeMidiClip(track_idx, uuid, semitones))
    }

    pub fn duplicate_midi_notes(&self, track_idx: usize, clip_id: String, note_indices: Vec<usize>) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.send_command(AudioCommand::DuplicateMidiNotes(track_idx, uuid, note_indices))
    }





    pub fn export_audio_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        path: String,
    ) -> Result<(), String> {
        let (clip, track_snapshot) = {
            let tracks = self.tracks.lock().unwrap();
            let track = tracks.get(track_idx).ok_or("Track not found")?;
            // Use to_string() comparison as established
            let clip = track
                .clips
                .iter()
                .find(|c| c.id.to_string() == clip_id)
                .ok_or("Clip not found")?
                .clone();

            // Create a dummy track with ONLY this clip, shifted to 0
            let mut dummy_track = track.clone_as_dummy();
            // We need to clear existing clips and add our target clip
            dummy_track.clips = vec![clip.clone()];
            dummy_track.clips[0].start_sample = 0;

            // Clear MIDI clips too just in case
            dummy_track.midi_clips.clear();

            (clip, vec![dummy_track])
        };

        let range_end = clip.length_in_samples;

        let config = crate::engine::render_engine::RenderConfig {
            output_path: PathBuf::from(path.clone()),
            format: crate::engine::render_engine::ExportFormat::Wav,
            sample_rate: 48000,
            bit_depth: crate::engine::render_engine::BitDepth::Integer24,
            dithering: crate::engine::render_engine::DitherMode::None,
            normalize_lufs: None,
            range: crate::engine::render_engine::RenderRange::Selection(0, range_end),
            stem_export: vec![],
            dry_run: false,
            mp3_bitrate: 320,
        };

        let (tx, _rx) = crossbeam_channel::unbounded();

        // Clone AudioGraph (Arc<Mutex> -> lock -> clone inner)
        let graph_snapshot = { self.audio_graph.lock().unwrap().clone() };

        let mut render_engine = crate::engine::render_engine::RenderEngine::new(
            graph_snapshot,
            track_snapshot,
            Arc::clone(&self.hyper_pool),
            Arc::clone(&self.hyper_streamer),
            Arc::clone(&self.fades),
            config,
            tx,
        );

        // Run synchronously
        render_engine.render();

        Ok(())
    }

    pub fn consolidate_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        is_midi: bool,
    ) -> Result<(), String> {
        if is_midi {
            return Err("MIDI consolidation not implemented".to_string());
        }

        // 1. Generate path
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        // Use a safe location for consolidated files. For now, Desktop to be visible, or temp.
        // Let's use the same logic as save_file_dialog default: Desktop.
        let path_str = format!(
            "C:\\Users\\brigh\\Desktop\\VIBE_Consolidate_{}.wav",
            timestamp
        );

        // 2. Export
        self.export_audio_clip(track_idx, clip_id.clone(), path_str.clone())?;

        // 3. Import
        let new_clip_info = self.import_file_internal(PathBuf::from(path_str))?;
        let new_template_uuid = Uuid::parse_str(&new_clip_info.id).map_err(|e| e.to_string())?;

        // 4. Replace in Track
        // Need to lock Library to get the template
        let template = {
            let lib = self.library.lock().unwrap();
            lib.iter()
                .find(|c| c.id == new_template_uuid)
                .cloned()
                .ok_or("Imported clip not found in library")?
        };

        {
            let mut tracks = self.tracks.lock().unwrap();
            let track = tracks.get_mut(track_idx).ok_or("Track not found")?;

            let old_clip_idx = track
                .clips
                .iter()
                .position(|c| c.id.to_string() == clip_id)
                .ok_or("Clip not found")?;
            let old_start = track.clips[old_clip_idx].start_sample;

            // Remove old
            track.clips.remove(old_clip_idx);

            // Add new
            let mut new_instance = template; // template is cloned
            new_instance.id = Uuid::new_v4();
            new_instance.start_sample = old_start;

            track.clips.push(new_instance);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn create_audio_track(&self, name: String) -> Result<String, String> {
        let track = Track::new(name);
        let id = track.id.to_string();
        self.command_tx
            .send(AudioCommand::AddTrack(track))
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn set_global_swing(&self, swing: f32) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetGlobalSwing(swing))
            .map_err(|e| e.to_string())
    }

    pub fn create_track_group(&self, name: String) -> Result<String, String> {
        use crate::engine::graph::TrackType;
        let mut track = Track::new(name);
        track.track_type = TrackType::Group;
        track.color = "#ffd700".to_string();
        let id = track.id;
        self.command_tx
            .send(AudioCommand::AddTrack(track))
            .map_err(|e| e.to_string())?;
        Ok(id.to_string())
    }

    #[allow(dead_code)]
    pub fn preview_seek(&self, position: f32) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::PreviewSeek(position))
            .map_err(|e| e.to_string())
    }

    pub fn preview_sample_synced(
        &self,
        path: String,
        quantize: Option<String>,
        stretch: bool,
        strength: f32,
        swing: f32,
    ) -> Result<(), String> {
        let q = match quantize.as_deref() {
            Some("1/4") => Some(QuantizeDivision::Quarter),
            Some("1/8") => Some(QuantizeDivision::Eighth),
            Some("1/16") => Some(QuantizeDivision::Sixteenth),
            Some("1Bar") | Some("Bar") => Some(QuantizeDivision::Quarter), // Map Bar to Quarter for now or fix enum
            _ => None,
        };

        // LOAD FILE HERE (Management Thread)
        let clip_info = self.import_file_internal(PathBuf::from(path))?;
        // For now we re-read or use the library cache?
        // import_file_internal pushed to library.
        // Let's get the data from library.
        let lib = self.library.lock().unwrap();
        let clip = lib
            .iter()
            .find(|c| c.id.to_string() == clip_info.id)
            .ok_or("Failed to load clip for preview")?;
        // Use head_data (RAM)
        let data = clip.head_data.to_vec(); // Convert Arc<Vec> to Vec (Clone)
                                            // Send COPY to audio thread. (It's Vec<f32>, cloning is O(N) but safe).
                                            // Optimize: Arc? AudioCommand takes Vec.

        self.command_tx
            .send(AudioCommand::PreviewSampleSynced(data, q, stretch, strength, swing))
            .map_err(|e| e.to_string())
    }

    pub fn stop_preview(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::StopPreview)
            .map_err(|e| e.to_string())
    }

    pub fn create_input_alias(
        &self,
        name: String,
        is_stereo: bool,
        channels: Vec<usize>,
        color: String,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::CreateInputAlias(
                name, is_stereo, channels, color,
            ))
            .map_err(|e| e.to_string())
    }

    pub fn delete_input_alias(&self, id: Uuid) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::DeleteInputAlias(id))
            .map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn import_file(&self, path: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::ImportToLibrary(PathBuf::from(path)))
            .map_err(|e| e.to_string())
    }

    pub fn import_file_internal(&self, path: PathBuf) -> Result<AudioClipInfo, String> {
        let clip = load_audio_file(path, 48000.0)?;
        let info = AudioClipInfo {
            id: clip.id.to_string(),
            name: clip.name.clone(),
            path: clip.path.clone(),
            start_sample: 0,
            duration_samples: clip.length_in_samples,
            peaks: clip.peaks.clone(),
            offset_in_data: 0,
            fade_in_len: clip.fade_in_len,
            fade_out_len: clip.fade_out_len,
            fade_in_type: format!("{:?}", clip.fade_in_type),
            fade_out_type: format!("{:?}", clip.fade_out_type),
            gain: clip.gain,
            pitch_semitones: clip.pitch_semitones,
            playback_speed: clip.playback_speed,
            is_warped: clip.is_warped,
            has_gain_envelope: clip.gain_envelope.is_some(),
            has_pitch_envelope: clip.pitch_envelope.is_some(),
            transient_count: clip.transients.len(),
            color: clip.color.clone(),
        };
        self.library.lock().unwrap().push(clip);
        Ok(info)
    }
    pub fn add_clip_to_track(
        &self,
        track_index: usize,
        clip_id: String,
        start_pos: u64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::AddClipToTrack(track_index, uuid, start_pos))
            .map_err(|e| e.to_string())
    }
    pub fn delete_clip(&self, track_index: usize, clip_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::DeleteClip(track_index, uuid))
            .map_err(|e| e.to_string())
    }
    pub fn slice_clip(
        &self,
        track_index: usize,
        clip_id: String,
        sample_pos: u64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::SliceClip(track_index, uuid, sample_pos))
            .map_err(|e| e.to_string())
    }
    #[allow(dead_code)]
    pub fn set_clip_fades(
        &self,
        track_idx: usize,
        clip_id: String,
        in_len: u64,
        out_len: u64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::SetClipFades(track_idx, uuid, in_len, out_len))
            .map_err(|e| e.to_string())
    }
    pub fn move_clip(
        &self,
        src_idx: usize,
        clip_id: String,
        dest_idx: usize,
        new_pos: u64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::MoveClip(src_idx, uuid, dest_idx, new_pos))
            .map_err(|e| e.to_string())
    }
    pub fn resize_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        new_start: u64,
        new_offset: u64,
        new_len: u64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::ResizeClip(
                track_idx, uuid, new_start, new_offset, new_len,
            ))
            .map_err(|e| e.to_string())
    }
    pub fn add_effect(&self, track_index: usize, effect_type: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::AddEffect(track_index, effect_type))
            .map_err(|e| e.to_string())
    }
    pub fn add_bus(&self, name: String, color: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::AddBus(name, color))
            .map_err(|e| e.to_string())
    }
    #[allow(dead_code)]
    pub fn route_track_to_bus(&self, track_index: usize, bus_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&bus_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::RouteTrackToBus(track_index, uuid))
            .map_err(|e| e.to_string())
    }

    pub fn set_parameter(&self, param_id: String, value: f64) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::SetParameter(uuid, value))
            .map_err(|e| e.to_string())
    }

    pub fn add_automation_point(
        &self,
        param_id: String,
        pos: u64,
        value: f64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::AddAutomationPoint(uuid, pos, value))
            .map_err(|e| e.to_string())
    }

    pub fn set_automation_tension(
        &self,
        param_id: String,
        pos: u64,
        tension: f64,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::SetAutomationTension(uuid, pos, tension))
            .map_err(|e| e.to_string())
    }

    pub fn clear_automation(&self, param_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::ClearAutomation(uuid))
            .map_err(|e| e.to_string())
    }

    pub fn note_on(&self, note: u8, velocity: u8) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::MidiNoteOn(note, velocity))
            .map_err(|e| e.to_string())
    }
    pub fn note_off(&self, note: u8) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::MidiNoteOff(note))
            .map_err(|e| e.to_string())
    }

    pub fn map_midi(&self, cc: u8, param_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&param_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::MapMidi(cc, uuid))
            .map_err(|e| e.to_string())
    }

    pub fn undo(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Undo)
            .map_err(|e| e.to_string())
    }

    pub fn redo(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Redo)
            .map_err(|e| e.to_string())
    }

    pub fn checkout(&self, node_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&node_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::Checkout(uuid))
            .map_err(|e| e.to_string())
    }

    pub fn create_branch(&self, name: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::CreateBranch(name))
            .map_err(|e| e.to_string())
    }

    pub fn set_metronome(&self, enabled: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetMetronome(enabled))
            .map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn is_metronome_enabled(&self) -> bool {
        self.metronome_enabled.load(Ordering::Relaxed)
    }

    pub fn get_eq_bands(
        &self,
        track_idx: usize,
        processor_id: String,
    ) -> Result<Vec<crate::engine::eq_module::EqBand>, String> {
        let mut track_list = self.tracks.lock().unwrap();
        if track_idx >= track_list.len() {
            return Err("Track index out of bounds".to_string());
        }

        let track = &mut track_list[track_idx];
        let uuid = Uuid::parse_str(&processor_id).map_err(|e| e.to_string())?;

        // Check if it's the console EQ
        if track.equalizer.id() == uuid {
            return Ok(track.equalizer.get_bands());
        }

        // Check inserts
        for processor in &mut track.processors {
            if processor.id() == uuid {
                if let Some(eq) = processor
                    .as_any()
                    .downcast_mut::<crate::engine::eq_module::dsp::equalizer::Equalizer>(
                ) {
                    return Ok(eq.get_bands());
                }
            }
        }

        Err("EQ processor not found".to_string())
    }

    pub fn set_eq_bands(
        &self,
        track_idx: usize,
        processor_id: String,
        bands: Vec<crate::engine::eq_module::EqBand>,
    ) -> Result<(), String> {
        let mut track_list = self.tracks.lock().unwrap();
        if track_idx >= track_list.len() {
            return Err("Track index out of bounds".to_string());
        }

        let track = &mut track_list[track_idx];
        let uuid = Uuid::parse_str(&processor_id).map_err(|e| e.to_string())?;

        // Check if it's the console EQ
        if track.equalizer.id() == uuid {
            track.equalizer.set_bands(bands);
            return Ok(());
        }

        // Check inserts
        for processor in &mut track.processors {
            if processor.id() == uuid {
                if let Some(eq) = processor
                    .as_any()
                    .downcast_mut::<crate::engine::eq_module::dsp::equalizer::Equalizer>(
                ) {
                    eq.set_bands(bands);
                    return Ok(());
                }
            }
        }

        Err("EQ processor not found".to_string())
    }

    #[allow(dead_code)]
    pub fn update_mod_matrix(
        &self,
        track_idx: usize,
        proc_idx: usize,
        slots: Vec<crate::engine::synth::ModSlot>,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::UpdateModMatrix(
                track_idx,
                proc_idx,
                slots.clone(),
            ))
            .map_err(|e| e.to_string())?;

        // Mirror state for persistence/UI
        let mut tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get_mut(track_idx) {
            if let Some(proc) = track.processors.get_mut(proc_idx) {
                if let Some(synth) = proc
                    .as_any()
                    .downcast_mut::<crate::engine::synth::VOneSynth>()
                {
                    for (i, slot) in slots.iter().enumerate() {
                        if i < 8 {
                            synth.mod_matrix[i] = *slot;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn load_synth_preset(
        &self,
        track_idx: usize,
        proc_idx: usize,
        path: String,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::LoadSynthPreset(track_idx, proc_idx, path))
            .map_err(|e| e.to_string())
    }

    pub fn save_synth_preset(
        &self,
        track_idx: usize,
        proc_idx: usize,
        path: String,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SaveSynthPreset(track_idx, proc_idx, path))
            .map_err(|e| e.to_string())
    }

    pub fn get_tracks(&self) -> Vec<TrackInfo> {
        // Need mutable access to call get_parameters()
        let mut track_list = self.tracks.lock().unwrap();
        track_list
            .iter_mut()
            .map(|t| TrackInfo {
                id: t.id.to_string(),
                name: t.name.clone(),
                volume: super::graph::ParameterInfo {
                    id: t.volume.id.to_string(),
                    name: t.volume.name.clone(),
                    value: t.volume.value,
                    min_value: t.volume.min_value,
                    max_value: t.volume.max_value,
                    automation: t.volume.curve.load().knots.clone(),
                },
                pan: super::graph::ParameterInfo {
                    id: t.pan.id.to_string(),
                    name: t.pan.name.clone(),
                    value: t.pan.value,
                    min_value: t.pan.min_value,
                    max_value: t.pan.max_value,
                    automation: t.pan.curve.load().knots.clone(),
                },
                width: super::graph::ParameterInfo {
                    id: t.width.id.to_string(),
                    name: t.width.name.clone(),
                    value: t.width.value,
                    min_value: t.width.min_value,
                    max_value: t.width.max_value,
                    automation: t.width.curve.load().knots.clone(),
                },
                input_drive: super::graph::ParameterInfo {
                    id: t.input_drive.id.to_string(),
                    name: t.input_drive.name.clone(),
                    value: t.input_drive.value,
                    min_value: t.input_drive.min_value,
                    max_value: t.input_drive.max_value,
                    automation: t.input_drive.curve.load().knots.clone(),
                },
                is_muted: t.is_muted,
                is_solo: t.is_solo,
                is_armed: t.is_armed,
                phase_inverted: t.phase_inverted,
                input_source: t.input_source.clone(),
                output_target: t.output_target.clone(),
                sidechain_source_id: t.sidechain_source_id.map(|id| id.to_string()),
                color: t.color.clone(),
                is_frozen: t.is_frozen,
                is_disabled: t.is_disabled,
                is_automation_armed: t.is_automation_armed,
                bus_id: t.bus_id.map(|b| b.to_string()),
                clips: t
                    .clips
                    .iter()
                    .map(|c| AudioClipInfo {
                        id: c.id.to_string(),
                        name: c.name.clone(),
                        path: c.path.clone(),
                        start_sample: c.start_sample,
                        duration_samples: c.length_in_samples,
                        peaks: c.peaks.clone(),
                        offset_in_data: c.offset_in_data,
                        fade_in_len: c.fade_in_len,
                        fade_out_len: c.fade_out_len,
                        fade_in_type: format!("{:?}", c.fade_in_type),
                        fade_out_type: format!("{:?}", c.fade_out_type),
                        gain: c.gain,
                        pitch_semitones: c.pitch_semitones,
                        playback_speed: c.playback_speed,
                        is_warped: c.is_warped,
                        has_gain_envelope: c.gain_envelope.is_some(),
                        has_pitch_envelope: c.pitch_envelope.is_some(),
                        transient_count: c.transients.len(),
                        color: c.color.clone(),
                    })
                    .collect(),
                midi_clips: t
                    .midi_clips
                    .iter()
                    .map(|mc| super::graph::MidiClipInfo {
                        id: mc.id.to_string(),
                        name: mc.name.clone(),
                        start_sample: mc.start_sample,
                        length_samples: mc.length_samples,
                        note_count: mc.notes.len(),
                        color: mc.color.clone(),
                        is_muted: mc.is_muted,
                        is_looped: mc.is_looped,
                        preview_notes: mc
                            .notes
                            .iter()
                            .take(100)
                            .map(|n| (n.start_sample, n.note, n.velocity))
                            .collect(),
                        pattern_id: mc.pattern_id.clone(),
                        tuning_steps: mc.tuning_steps,
                        time_signature_num: mc.time_signature_num,
                        time_signature_den: mc.time_signature_den,
                        gain_offset: 0.0,
                        has_envelope: false,
                    })
                    .collect(),
                effects: t
                    .processors
                    .iter_mut()
                    .map(|p| {
                        let mut info = super::graph::EffectInfo {
                            id: p.id().to_string(),
                            name: p.name(),
                            is_bypassed: p.is_bypassed(),
                            parameters: p
                                .get_parameters()
                                .iter()
                                .map(|param| super::graph::ParameterInfo {
                                    id: param.id.to_string(),
                                    name: param.name.clone(),
                                    value: param.value,
                                    min_value: param.min_value,
                                    max_value: param.max_value,
                                    automation: param.curve.load().knots.clone(),
                                })
                                .collect(),
                            mod_matrix: None,
                        };

                        if let Some(synth) =
                            p.as_any().downcast_mut::<crate::engine::synth::VOneSynth>()
                        {
                            info.mod_matrix = Some(synth.mod_matrix.to_vec());
                        }

                        info
                    })
                    .collect(),
                console_eq: super::graph::EffectInfo {
                    id: t.equalizer.id().to_string(),
                    name: t.equalizer.name(),
                    is_bypassed: false,
                    parameters: t
                        .equalizer
                        .get_parameters()
                        .iter()
                        .map(|param| super::graph::ParameterInfo {
                            id: param.id.to_string(),
                            name: param.name.clone(),
                            value: param.value,
                            min_value: param.min_value,
                            max_value: param.max_value,
                            automation: param.curve.load().knots.clone(),
                        })
                        .collect(),
                    mod_matrix: None,
                },
                console_comp: super::graph::EffectInfo {
                    id: t.compressor.id().to_string(),
                    name: t.compressor.name(),
                    is_bypassed: false,
                    parameters: t
                        .compressor
                        .get_parameters()
                        .iter()
                        .map(|param| super::graph::ParameterInfo {
                            id: param.id.to_string(),
                            name: param.name.clone(),
                            value: param.value,
                            min_value: param.min_value,
                            max_value: param.max_value,
                            automation: param.curve.load().knots.clone(),
                        })
                        .collect(),
                    mod_matrix: None,
                },
                eq_pre_dynamics: super::graph::ParameterInfo {
                    id: t.eq_pre_dynamics.id.to_string(),
                    name: t.eq_pre_dynamics.name.clone(),
                    value: t.eq_pre_dynamics.value,
                    min_value: t.eq_pre_dynamics.min_value,
                    max_value: t.eq_pre_dynamics.max_value,
                    automation: t.eq_pre_dynamics.curve.load().knots.clone(),
                },
                track_type: t.track_type,
                monitoring_mode: t.monitoring_mode,
                parent_id: t.parent_id.map(|id| id.to_string()),
                is_collapsed: t.is_collapsed,
                height: t.height,
                peak_l: t.meter.get_peak_db(0) as f32,
                peak_r: t.meter.get_peak_db(1) as f32,
                rms_l: t.meter.get_rms_db(0) as f32,
                rms_r: t.meter.get_rms_db(1) as f32,
                lufs_l: t.meter.get_lufs_full().momentary as f32,
                lufs_r: t.meter.get_lufs_full().momentary as f32,
                playlist_count: t.playlists.len(),
                active_playlist_name: if t.active_playlist_idx < t.playlists.len() {
                    t.playlists[t.active_playlist_idx].name.clone()
                } else {
                    "Main".to_string()
                },
                take_count: t.takes.len(),
                comp_mode_enabled: t.comp_mode_enabled,
                comp_lanes: t.takes.iter().map(|take_lane| {
                     take_lane.iter().map(|c| super::graph::AudioClipInfo {
                        id: c.id.to_string(),
                        name: c.name.clone(),
                        path: c.path.clone(),
                        start_sample: c.start_sample,
                        duration_samples: c.length_in_samples,
                        peaks: vec![],
                        offset_in_data: c.offset_in_data,
                        fade_in_len: c.fade_in_len,
                        fade_out_len: c.fade_out_len,
                        fade_in_type: format!("{:?}", c.fade_in_type),
                        fade_out_type: format!("{:?}", c.fade_out_type),
                        gain: c.gain,
                        pitch_semitones: c.pitch_semitones,
                        playback_speed: c.playback_speed,
                        is_warped: c.is_warped,
                        has_gain_envelope: c.gain_envelope.is_some(),
                        has_pitch_envelope: c.pitch_envelope.is_some(),
                        transient_count: c.transients.len(),
                        color: c.color.clone(),
                    }).collect()
                }).collect()
            })
            .collect()
    }

    pub fn get_master_info(&self) -> Vec<super::graph::EffectInfo> {
        let mut engine_fx = self.engine_fx.lock().unwrap();
        let mut master_limiter = self.master_limiter.lock().unwrap();

        vec![
            super::graph::EffectInfo {
                id: engine_fx.id().to_string(),
                name: engine_fx.name(),
                is_bypassed: engine_fx.is_bypassed(),
                parameters: engine_fx
                    .get_parameters()
                    .iter()
                    .map(|p| super::graph::ParameterInfo {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                        value: p.value,
                        min_value: p.min_value,
                        max_value: p.max_value,
                        automation: p.curve.load().knots.clone(),
                    })
                    .collect(),
                mod_matrix: None,
            },
            super::graph::EffectInfo {
                id: master_limiter.id().to_string(),
                name: master_limiter.name(),
                is_bypassed: master_limiter.is_bypassed(),
                parameters: master_limiter
                    .get_parameters()
                    .iter()
                    .map(|p| super::graph::ParameterInfo {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                        value: p.value,
                        min_value: p.min_value,
                        max_value: p.max_value,
                        automation: p.curve.load().knots.clone(),
                    })
                    .collect(),
                mod_matrix: None,
            },
        ]
    }

    pub fn get_midi_clip_data(&self, track_idx: usize, clip_id: String) -> Result<MidiClip, String> {
        let tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get(track_idx) {
            if let Some(clip) = track
                .midi_clips
                .iter()
                .find(|c| c.id.to_string() == clip_id)
            {
                return Ok(clip.clone());
            }
        }
        Err("Clip not found".to_string())
    }

    #[allow(dead_code)]
    pub fn get_track_meters(&self) -> Vec<(f64, f64, f64, f64)> {
        let tracks = self.tracks.lock().unwrap();
        tracks
            .iter()
            .map(|t| {
                let (p_l, p_r) = (t.meter.get_peak_db(0), t.meter.get_peak_db(1));
                let (r_l, r_r) = (t.meter.get_rms_db(0), t.meter.get_rms_db(1));
                (p_l, p_r, r_l, r_r)
            })
            .collect()
    }

    pub fn get_library(&self) -> Vec<AudioClipInfo> {
        let lib = self.library.lock().unwrap();
        lib.iter()
            .map(|c| AudioClipInfo {
                id: c.id.to_string(),
                name: c.name.clone(),
                path: c.path.clone(),
                start_sample: 0,
                duration_samples: c.length_in_samples,
                peaks: c.peaks.clone(),
                offset_in_data: 0,
                fade_in_len: c.fade_in_len,
                fade_out_len: c.fade_out_len,
                fade_in_type: format!("{:?}", c.fade_in_type),
                fade_out_type: format!("{:?}", c.fade_out_type),
                gain: c.gain,
                pitch_semitones: c.pitch_semitones,
                playback_speed: c.playback_speed,
                is_warped: c.is_warped,
                has_gain_envelope: c.gain_envelope.is_some(),
                has_pitch_envelope: c.pitch_envelope.is_some(),
                transient_count: c.transients.len(),
                color: c.color.clone(),
            })
            .collect()
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Acquire)
    }
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Acquire)
    }

    #[allow(dead_code)]
    pub fn get_recorded_samples(&self) -> Vec<f32> {
        let mut rec = self.recorded_samples.lock().unwrap();
        std::mem::take(&mut *rec)
    }

    pub fn get_clip_data(&self, clip_id: Uuid) -> Option<Arc<Vec<f32>>> {
        let lib = self.library.lock().unwrap();
        lib.iter()
            .find(|c| c.id == clip_id)
            .map(|c| Arc::clone(&c.head_data))
    }

    pub fn get_samples_range(
        &self,
        clip_id: Uuid,
        start: u64,
        end: u64,
    ) -> Result<Vec<f32>, String> {
        let lib = self.library.lock().unwrap();
        let clip = lib
            .iter()
            .find(|c| c.id == clip_id)
            .ok_or("Clip not found")?;

        let actual_end = end.min(clip.length_in_samples);
        if start >= actual_end {
            return Ok(vec![]);
        }

        let len = (actual_end - start) as usize;

        // If it's in head_data, return directly
        if actual_end <= clip.head_data.len() as u64 {
            let s = start as usize;
            let e = actual_end as usize;
            return Ok(clip.head_data[s..e].to_vec());
        }

        // Otherwise read from file
        if let Some(path_str) = &clip.path {
            use symphonia::core::audio::Signal;
            use symphonia::core::formats::FormatOptions;
            use symphonia::core::io::MediaSourceStream;

            let path = PathBuf::from(path_str);
            let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = symphonia::core::probe::Hint::new();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                hint.with_extension(ext);
            }

            let probed = symphonia::default::get_probe()
                .format(&hint, mss, &FormatOptions::default(), &Default::default())
                .map_err(|e| e.to_string())?;

            let mut format = probed.format;
            let stream = format.default_track().ok_or("No default track")?;
            let mut decoder = symphonia::default::get_codecs()
                .make(&stream.codec_params, &Default::default())
                .map_err(|e| e.to_string())?;

            // Seek
            format
                .seek(
                    symphonia::core::formats::SeekMode::Accurate,
                    symphonia::core::formats::SeekTo::Time {
                        time: symphonia::core::units::Time::from(
                            start as f64 / clip.sample_rate as f64,
                        ),
                        track_id: None,
                    },
                )
                .map_err(|e| e.to_string())?;

            let mut samples = Vec::with_capacity(len);
            let mut decoded_count = 0;

            while decoded_count < len {
                let packet = match format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break,
                };
                let decoded = decoder.decode(&packet).map_err(|e| e.to_string())?;
                let mut buffer = symphonia::core::audio::AudioBuffer::<f32>::new(
                    decoded.capacity() as u64,
                    *decoded.spec(),
                );
                decoded.convert(&mut buffer);

                let frames = buffer.frames();
                let channels = buffer.spec().channels.count();

                for i in 0..frames {
                    let mut sum = 0.0;
                    for c in 0..channels {
                        sum += buffer.chan(c)[i];
                    }
                    samples.push(sum / channels as f32);
                    decoded_count += 1;
                    if decoded_count >= len {
                        break;
                    }
                }
            }
            Ok(samples)
        } else {
            Err("No file path available for streaming".to_string())
        }
    }

    pub fn add_midi_clip(&self, track_idx: usize, clip: MidiClip) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::AddMidiClip(track_idx, clip))
            .map_err(|e| e.to_string())
    }

    /// Searches for the nearest zero-crossing within `window_samples` of `position`.
    /// Uses the best available audio data (head cache, then None).
    /// Returns None if no audio data is available.
    pub fn find_zero_crossing_near(&self, position: u64, window_samples: u64) -> Option<u64> {
        use crate::engine::waveform::PyramidCache;
        let lib = self.library.lock().ok()?;
        // Find any clip whose head_data contains the position
        for clip in lib.iter() {
            let head_len = clip.head_data.len() as u64;
            if position < head_len {
                let start = position.saturating_sub(window_samples) as usize;
                let end = (position + window_samples).min(head_len - 1) as usize;
                let samples = &clip.head_data[start..=end];
                let relative_pos = (position - start as u64) as usize;
                let snapped_rel = PyramidCache::find_zero_crossing(
                    samples,
                    relative_pos,
                    window_samples as usize,
                );
                return Some(start as u64 + snapped_rel as u64);
            }
        }
        None
    }


    pub fn delete_midi_clip(&self, track_idx: usize, clip_id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::DeleteMidiClip(track_idx, uuid))
            .map_err(|e| e.to_string())
    }

    pub fn update_midi_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        clip: MidiClip,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(&clip_id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::UpdateMidiClip(track_idx, uuid, clip))
            .map_err(|e| e.to_string())
    }

    pub fn add_midi_note(
        &self,
        track_idx: usize,
        clip_id: String,
        note: MidiNote,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::AddMidiNote(track_idx, clip_id, note))
            .map_err(|e| e.to_string())
    }

    pub fn delete_midi_note(
        &self,
        track_idx: usize,
        clip_id: String,
        note_idx: usize,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::DeleteMidiNote(track_idx, clip_id, note_idx))
            .map_err(|e| e.to_string())
    }

    pub fn get_compressor_metrics(&self, track_idx: usize, processor_id: String) -> (f32, f32) {
        let mut tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get_mut(track_idx) {
            let proc_uuid = Uuid::parse_str(&processor_id).unwrap_or(Uuid::nil());
            if let Some(proc) = track.processors.iter_mut().find(|p| p.id() == proc_uuid) {
                if let Some(comp) = proc
                    .as_any()
                    .downcast_mut::<crate::engine::dynamics_module::Compressor>()
                {
                    return comp.get_metrics();
                }
            }
        }
        (0.0, 0.0)
    }

    // --- Phase 5.2: Video Sync ---

    pub fn load_video(&self, path: PathBuf) -> Result<super::video_manager::VideoState, String> {
        self.video_manager.load_video(path)
    }

    pub fn unload_video(&self) -> Result<(), String> {
        self.video_manager.unload_video()
    }

    pub fn set_video_offset(&self, offset_samples: i64) -> Result<(), String> {
        self.video_manager.set_offset(offset_samples)
    }

    pub fn set_video_framerate(&self, fps: f64) -> Result<(), String> {
        self.video_manager.set_framerate(fps)
    }

    pub fn get_video_state(&self) -> super::video_manager::VideoState {
        self.video_manager.get_state()
    }

    pub fn update_midi_note(
        &self,
        track_idx: usize,
        clip_id: String,
        note_idx: usize,
        note: MidiNote,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::UpdateMidiNote(
                track_idx, clip_id, note_idx, note,
            ))
            .map_err(|e| e.to_string())
    }

    pub fn set_clip_scale(
        &self,
        track_idx: usize,
        clip_id: String,
        scale: Option<Scale>,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetClipScale(track_idx, clip_id, scale))
            .map_err(|e| e.to_string())
    }

    pub fn detect_chords(&self, track_idx: usize, clip_id: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::DetectChords(track_idx, clip_id))
            .map_err(|e| e.to_string())
    }

    pub fn quantize_notes(
        &self,
        track_idx: usize,
        clip_id: String,
        division: QuantizeDivision,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::QuantizeNotes(track_idx, clip_id, division))
            .map_err(|e| e.to_string())
    }

    pub fn generate_stress_notes(
        &self,
        track_idx: usize,
        clip_id: String,
        count: usize,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::GenerateStressNotes(track_idx, clip_id, count))
            .map_err(|e| e.to_string())
    }

    // --- Phase 3.10: Advanced Routing Matrix API ---

    pub fn graph_add_node(
        &self,
        node: crate::engine::audio_graph::GraphNode,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::GraphAddNode(node))
            .map_err(|e| e.to_string())
    }

    pub fn graph_remove_node(&self, id: String) -> Result<(), String> {
        let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
        self.command_tx
            .send(AudioCommand::GraphRemoveNode(uuid))
            .map_err(|e| e.to_string())
    }

    pub fn graph_connect(
        &self,
        from_node: String,
        to_node: String,
        from_port: u32,
        to_port: u32,
        gain_db: f64,
    ) -> Result<(), String> {
        let from_uuid = Uuid::parse_str(&from_node).map_err(|e| e.to_string())?;
        let to_uuid = Uuid::parse_str(&to_node).map_err(|e| e.to_string())?;

        self.command_tx
            .send(AudioCommand::GraphConnect {
                from_node: from_uuid,
                to_node: to_uuid,
                from_port,
                to_port,
                gain_db,
            })
            .map_err(|e| e.to_string())
    }

    pub fn graph_disconnect(&self, from_node: String, to_node: String) -> Result<(), String> {
        let from_uuid = Uuid::parse_str(&from_node).map_err(|e| e.to_string())?;
        let to_uuid = Uuid::parse_str(&to_node).map_err(|e| e.to_string())?;

        self.command_tx
            .send(AudioCommand::GraphDisconnect {
                from_node: from_uuid,
                to_node: to_uuid,
            })
            .map_err(|e| e.to_string())
    }
}

pub fn load_audio_file(path: PathBuf, target_sample_rate: f64) -> Result<AudioClip, String> {
    use symphonia::core::audio::Signal;
    use symphonia::core::formats::FormatOptions;
    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();
    
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            hint.with_extension(ext_str);
        }
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &Default::default())
        .map_err(|e| e.to_string())?;

    let mut format = probed.format;
    let stream = format.default_track().ok_or("No default track")?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&stream.codec_params, &Default::default())
        .map_err(|e| e.to_string())?;

    let mut raw_samples_interleaved = Vec::new();
    let source_sample_rate = stream.codec_params.sample_rate.unwrap_or(48000) as f64;
    // We load EVERYTHING into RAM first for V1 (Streaming support is partial)
    // For VIBE 4.5 Resampling, we need the whole or chunks.
    // Since 'head_samples' logic was previously loading everything if small...

    // We simply decode until end for now (assuming fits in RAM or is head)
    // The original logic had a limit. Let's respect the limit for HEAD, but wait,
    // if we resample, we need continuity.
    // If disk file is 44.1k and project is 48k, streaming logic needs a real-time resampler.
    // THAT is the Linear Interpolation task (Task 6).
    // HERE we are handling the "RAM Head" or "Small Clip".

    let _head_limit_samples_source = (source_sample_rate * 0.75) as usize;
    let mut _total_source_samples = 0u64;
    let mut _is_streaming = false;

    // Decode loop
    while let Ok(packet) = format.next_packet() {
        // Track duration even if we don't decode
        _total_source_samples += packet.dur();

        // For VIBE 1.0 Stabilization: Force load EVERYTHING into RAM. 
        // Streaming logic is too fragile for "Application hung again" reports.
        // We need rock solid reliability first.
        let decoded = decoder.decode(&packet).map_err(|e| e.to_string())?;
        let mut buffer = symphonia::core::audio::AudioBuffer::<f32>::new(
            decoded.capacity() as u64,
            *decoded.spec(),
        );
        decoded.convert(&mut buffer);

        // Interleave
        for i in 0..buffer.frames() {
            // ALWAYS push to RAM for now (Disable Streaming optimization)
            // if total_source_samples < head_limit_samples_source as u64 {
                for c in 0..buffer.spec().channels.count() {
                    raw_samples_interleaved.push(buffer.chan(c)[i]);
                }
            // } else {
            //     is_streaming = true;
            // }
            // total_source_samples += 1; // Handled by packet.dur() above approximately? No, packet.dur is frames.
        }
    }
    
    // Recalculate duration based on actual decoded frames
    _total_source_samples = (raw_samples_interleaved.len() / 2) as u64; // Assuming stereo
    _is_streaming = false; // Forced RAM mode

    // RESAMPLING STEP
    let (final_data, final_rate) = if (source_sample_rate - target_sample_rate).abs() > 0.1 {
        // We need to resample the HEAD data
        // For streaming files, this creates a mismatch: Head is 48k, Disk is 44.1k.
        // The Streamer MUST handle resampling if is_streaming is true.
        // For now, we only resample the head data here.

        let resampler = super::resampler::VibeResampler::new(
            super::resampler::ResamplingQuality::High,
            1024,
            2, // Assuming Stereo
        );

        // Convert f32 -> f64 for resampler
        let input_f64: Vec<f64> = raw_samples_interleaved.iter().map(|&s| s as f64).collect();
        let resampled_f64 = resampler
            .resample(&input_f64, source_sample_rate, target_sample_rate)
            .map_err(|e| format!("Resampling failed: {}", e))?;

        // Convert back to f32 for storage (AudioClip struct limitation)
        let final_f32: Vec<f32> = resampled_f64.iter().map(|&s| s as f32).collect();
        (final_f32, target_sample_rate)
    } else {
        (raw_samples_interleaved, source_sample_rate)
    };

    // Update length_in_samples to match TARGET rate
    let ratio = target_sample_rate / source_sample_rate;
    let final_length = (_total_source_samples as f64 * ratio).ceil() as u64;

    // Use PyramidCache for multi-resolution visualization
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    let cache_dir = std::env::temp_dir().join("vibe_peaks");
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache_path = cache_dir.join(format!("{:x}.vpeak", hasher.finish()));

    let cache = if let Ok(loaded) = crate::engine::waveform::PyramidCache::load_cache(&cache_path) {
        loaded
    } else {
        let generated = crate::engine::waveform::PyramidCache::generate(&final_data, final_rate as u32);
        let _ = generated.save_cache(&cache_path);
        generated
    };

    Ok(AudioClip {
        id: Uuid::new_v4(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        head_data: Arc::new(final_data),
        peaks: Vec::new(), // Deprecated in favor of waveform_cache
        start_sample: 0,
        offset_in_data: 0,
        length_in_samples: final_length,
        sample_rate: final_rate as u32,
        color: String::new(),
        fade_in_len: 0,
        fade_out_len: 0,
        fade_in_type: super::fades::FadeType::Linear,
        fade_out_type: super::fades::FadeType::Linear,
        gain: 1.0,
        pitch_semitones: 0.0,
        playback_speed: 1.0,
        is_warped: false,
        is_reversed: false,
        warp_mode: super::graph::WarpMode::Beats,
        path: Some(path.to_string_lossy().to_string()),
        waveform_cache: Some(Arc::new(cache)),
        is_streaming: _is_streaming,
        #[cfg(target_os = "windows")]
        file: if _is_streaming {
            std::fs::File::open(&path).ok().map(Arc::new)
        } else {
            None
        },
        gain_envelope: None,
        pitch_envelope: None,
        pan_envelope: None,
        transients: Vec::new(),
        warp_markers: vec![],
        base_bpm: 120.0,
        reference_clip_id: None,
    })
}

impl AudioEngine {
    #[allow(dead_code)]
    pub fn open_editor(
        &self,
        track_idx: usize,
        plugin_id: String,
        window_handle: usize,
    ) -> Result<(), String> {
        let mut tracks = self.tracks.lock().unwrap();
        if let Some(track) = tracks.get_mut(track_idx) {
            for processor in &mut track.processors {
                if processor.id().to_string() == plugin_id {
                    let handle_ptr = window_handle as *mut std::ffi::c_void;
                    processor.open_editor(handle_ptr);
                    return Ok(());
                }
            }
        }
        Err("Plugin not found".to_string())
    }

    /// Save current project state to binary .vibe file
    pub fn save_project(&self, path: &std::path::Path) -> Result<(), String> {
        use super::persistence::{
            save_project, ClipSnapshot, MidiClipSnapshot, MidiNoteSnapshot, ProjectSnapshot,
            TrackSnapshot,
        };

        let mut tracks_lock = self.tracks.lock().unwrap();
        let vca_lock = self.vca_groups.lock().unwrap();
        let bpm_lock = self.bpm.lock().unwrap();
        let config_lock = self.current_config.lock().unwrap();
        let io_lock = self.io_manager.lock().unwrap();

        // Convert AudioEngine state to ProjectSnapshot
        let snapshot = ProjectSnapshot {
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            bpm: *bpm_lock as f64,
            sample_rate: config_lock.as_ref().map(|c| c.sample_rate as f64).unwrap_or(48000.0),
            master_volume: 1.0,    // TODO: Get from master bus
            master_pan: 0.0,       // TODO: Get from master bus
            input_aliases: io_lock.get_all_input_aliases(),
            midi_bindings: self.neural_mapper.get_bindings(),
            loop_enabled: self.loop_enabled.load(Ordering::Relaxed),
            loop_start: self.loop_start.load(Ordering::Relaxed),
            loop_end: self.loop_end.load(Ordering::Relaxed),
            vca_groups: vca_lock
                .iter()
                .map(|vca| super::persistence::VcaGroupSnapshot {
                    id: vca.id.to_string(),
                    name: vca.name.clone(),
                    member_tracks: vca.member_tracks.iter().map(|id| id.to_string()).collect(),
                    gain: super::persistence::ParameterSnapshot {
                        id: vca.gain.id.to_string(),
                        name: vca.gain.name.clone(),
                        value: vca.gain.get_current_value(),
                        automation: vca.gain.curve.load().knots.clone(),
                    },
                    is_muted: vca.is_muted,
                    is_solo: vca.is_solo,
                })
                .collect(),
            tracks: tracks_lock
                .iter_mut()
                .map(|track| TrackSnapshot {
                    id: track.id.to_string(),
                    name: track.name.clone(),
                    volume: super::persistence::ParameterSnapshot {
                        id: track.volume.id.to_string(),
                        name: track.volume.name.clone(),
                        value: track.volume.get_current_value(),
                        automation: track.volume.curve.load().knots.clone(),
                    },
                    pan: super::persistence::ParameterSnapshot {
                        id: track.pan.id.to_string(),
                        name: track.pan.name.clone(),
                        value: track.pan.get_current_value(),
                        automation: track.pan.curve.load().knots.clone(),
                    },
                    width: super::persistence::ParameterSnapshot {
                        id: track.width.id.to_string(),
                        name: track.width.name.clone(),
                        value: track.width.get_current_value(),
                        automation: track.width.curve.load().knots.clone(),
                    },
                    input_drive: super::persistence::ParameterSnapshot {
                        id: track.input_drive.id.to_string(),
                        name: track.input_drive.name.clone(),
                        value: track.input_drive.get_current_value(),
                        automation: track.input_drive.curve.load().knots.clone(),
                    },
                    muted: track.is_muted,
                    solo: track.is_solo,
                    is_armed: track.is_armed,
                    phase_inverted: track.phase_inverted,
                    color: track.color.clone(),
                    clips: track
                        .clips
                        .iter()
                        .map(|clip| ClipSnapshot {
                            id: clip.id.to_string(),
                            audio_path: clip.path.clone().unwrap_or_default(),
                            start_sample: clip.start_sample,
                            duration_samples: clip.length_in_samples,
                            offset_in_data: clip.offset_in_data,
                            fade_in_len: clip.fade_in_len,
                            fade_out_len: clip.fade_out_len,
                            fade_in_type: clip.fade_in_type,
                            fade_out_type: clip.fade_out_type,
                        })
                        .collect(),
                    plugins: track
                        .processors
                        .iter_mut()
                        .map(|p| super::persistence::PluginSnapshot {
                            id: p.id().to_string(),
                            plugin_path: p.name(),
                            state_blob: p.get_state(),
                            parameters: p
                                .get_parameters()
                                .iter()
                                .map(|param| super::persistence::ParameterSnapshot {
                                    id: param.id.to_string(),
                                    name: param.name.clone(),
                                    value: param.get_current_value(),
                                    automation: param.curve.load().knots.clone(),
                                })
                                .collect(),
                        })
                        .collect(),
                    input_alias_id: track.input_alias_id.map(|id| id.to_string()),
                    // MIDI Sequencer
                    midi_clips: track
                        .midi_clips
                        .iter()
                        .map(|midi_clip| MidiClipSnapshot {
                            id: midi_clip.id.to_string(),
                            name: midi_clip.name.clone(),
                            start_sample: midi_clip.start_sample,
                            length_samples: midi_clip.length_samples,
                            color: midi_clip.color.clone(),
                            is_muted: midi_clip.is_muted,
                            is_looped: midi_clip.is_looped,
                            scale: midi_clip.scale.clone(),
                            chord_markers: midi_clip.chord_markers.clone(),
                            groove_template: midi_clip.groove_template.clone(),
                            pattern_id: midi_clip.pattern_id.clone(),
                            tuning_steps: midi_clip.tuning_steps,
                            time_signature_num: midi_clip.time_signature_num,
                            time_signature_den: midi_clip.time_signature_den,
                            cc_events: midi_clip
                                .cc_events
                                .iter()
                                .map(|cc| super::persistence::MidiCCSnapshot {
                                    sample: cc.sample,
                                    cc_number: cc.cc_number,
                                    value: cc.value,
                                    channel: cc.channel,
                                })
                                .collect(),
                            notes: midi_clip
                                .notes
                                .iter()
                                .map(|note| MidiNoteSnapshot {
                                    start_sample: note.start_sample,
                                    length_samples: note.length_samples,
                                    note: note.note,
                                    velocity: note.velocity,
                                    channel: note.channel,
                                    pitch_bend: note.pitch_bend,
                                    pressure: note.pressure,
                                    timbre: note.timbre,
                                    probability: note.probability,
                                    velocity_random: note.velocity_random,
                                    timing_random: note.timing_random,
                                })
                                .collect(),
                        })
                        .collect(),
                    quantize_division: track.quantize_division,
                })
                .collect(),
        };

        save_project(&snapshot, path)
    }


    pub fn set_audio_config(
        &self,
        config: crate::engine::audio_device::AudioDeviceConfig,
    ) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetAudioConfig(config))
            .map_err(|e| e.to_string())
    }

    /// Load project from binary .vibe file
    pub fn load_project(
        &mut self,
        path: &std::path::Path,
        plugin_manager: &std::sync::Mutex<super::plugin_manager::PluginManager>,
    ) -> Result<(), String> {
        use super::persistence::load_project;
        use std::sync::atomic::Ordering as AtomicOrdering;

        let snapshot = load_project(path)?;

        // Apply loaded state to AudioEngine
        let project_name = snapshot.name.clone();
        let track_count = snapshot.tracks.len();

        *self.bpm.lock().unwrap() = snapshot.bpm as f32;
        self.bpm_atomic
            .store(snapshot.bpm.to_bits(), AtomicOrdering::Relaxed);

        self.loop_enabled.store(snapshot.loop_enabled, AtomicOrdering::Relaxed);
        self.loop_start.store(snapshot.loop_start, AtomicOrdering::Relaxed);
        self.loop_end.store(snapshot.loop_end, AtomicOrdering::Relaxed);

        // Clear existing tracks
        self.tracks.lock().unwrap().clear();

        // Recreate tracks from snapshot (consume snapshot.tracks)
        for track_snap in snapshot.tracks {
            let mut track = super::graph::Track::new(track_snap.name);
            track.id = Uuid::parse_str(&track_snap.id).unwrap_or_else(|_| Uuid::new_v4());
            track.volume.set_value(track_snap.volume.value);
            let mut v_curve =
                crate::engine::automation::AutomationCurve::new(track_snap.volume.value);
            v_curve.knots = track_snap.volume.automation;
            track.volume.curve.store(Arc::new(v_curve));

            track.pan.set_value(track_snap.pan.value);
            track.width.set_value(track_snap.width.value);
            track.input_drive.set_value(track_snap.input_drive.value);

            track.is_muted = track_snap.muted;
            track.is_solo = track_snap.solo;
            track.is_armed = track_snap.is_armed;
            track.phase_inverted = track_snap.phase_inverted;
            track.color = track_snap.color;
            track.input_alias_id = track_snap
                .input_alias_id
                .and_then(|id| Uuid::parse_str(&id).ok());

            // Restore plugins
            for p_snap in track_snap.plugins {
               // Load plugin via PluginManager
               if let Ok(mut plugin) = plugin_manager.lock().unwrap().load_plugin(&p_snap.plugin_path) {
                    // Restore state
                    if !p_snap.state_blob.is_empty() {
                         let blob: &[u8] = &p_snap.state_blob;
                         plugin.set_state(blob);
                    }
                    track.processors.push(plugin);
               } else {
                   eprintln!("Failed to load plugin: {}", p_snap.plugin_path);
               }
            }

            // Restore audio clips
            for ci in track_snap.clips {
                if let Some(template) = self
                    .library
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|c| c.name == ci.audio_path)
                {
                    let mut clip = template.clone();
                    clip.id = Uuid::parse_str(&ci.id).unwrap_or(Uuid::new_v4());
                    clip.start_sample = ci.start_sample;
                    clip.offset_in_data = ci.offset_in_data;
                    clip.length_in_samples = ci.duration_samples;
                    clip.fade_in_len = ci.fade_in_len;
                    clip.fade_out_len = ci.fade_out_len;
                    clip.fade_in_type = ci.fade_in_type;
                    clip.fade_out_type = ci.fade_out_type;
                    track.clips.push(clip);
                }
            }

            // Restore MIDI clips
            for mc_snap in track_snap.midi_clips {
                let midi_clip = super::graph::MidiClip {
                    id: Uuid::parse_str(&mc_snap.id).unwrap_or_else(|_| Uuid::new_v4()),
                    name: mc_snap.name,
                    start_sample: mc_snap.start_sample,
                    length_samples: mc_snap.length_samples,
                    notes: mc_snap
                        .notes
                        .into_iter()
                        .map(|n| super::graph::MidiNote {
                            start_sample: n.start_sample,
                            length_samples: n.length_samples,
                            note: n.note,
                            velocity: n.velocity,
                            channel: n.channel,
                            pitch_bend: n.pitch_bend,
                            pressure: n.pressure,
                            timbre: n.timbre,
                            probability: n.probability,
                            velocity_random: n.velocity_random,
                            timing_random: n.timing_random,
                        })
                        .collect(),
                    cc_events: mc_snap
                        .cc_events
                        .into_iter()
                        .map(|cc| super::graph::MidiCCEvent {
                            sample: cc.sample,
                            cc_number: cc.cc_number,
                            value: cc.value,
                            channel: cc.channel,
                        })
                        .collect(),
                    color: mc_snap.color,
                    is_muted: mc_snap.is_muted,
                    is_looped: mc_snap.is_looped,
                    scale: mc_snap.scale,
                    chord_markers: mc_snap.chord_markers,
                    groove_template: mc_snap.groove_template,
                    pattern_id: mc_snap.pattern_id,
                    tuning_steps: mc_snap.tuning_steps,
                    time_signature_num: mc_snap.time_signature_num,
                    time_signature_den: mc_snap.time_signature_den,
                    reference_clip_id: None,
                };
                track.midi_clips.push(midi_clip);
            }

            self.tracks.lock().unwrap().push(track);
        }

        // Restore VCA groups
        let mut vca_groups_lock = self.vca_groups.lock().unwrap();
        vca_groups_lock.clear();
        for vca_snap in snapshot.vca_groups {
            let mut vca = crate::engine::vca_group::VcaGroup::new(vca_snap.name);
            vca.id = Uuid::parse_str(&vca_snap.id).unwrap_or_else(|_| Uuid::new_v4());
            vca.member_tracks = vca_snap.member_tracks.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect();
            vca.gain.set_value(vca_snap.gain.value);
            let mut g_curve = crate::engine::automation::AutomationCurve::new(vca_snap.gain.value);
            g_curve.knots = vca_snap.gain.automation;
            vca.gain.curve.store(Arc::new(g_curve));
            vca.is_muted = vca_snap.is_muted;
            vca.is_solo = vca_snap.is_solo;
            
            let _ = self.graph_prod.lock().unwrap().push(GraphCommand::AddVcaGroup(vca.clone()));
            vca_groups_lock.push(vca);
        }

        println!(
            "VIBE: Loaded project '{}' with {} tracks",
            project_name, track_count
        );
        Ok(())
    }
    pub fn export_midi_clip(
        &self,
        track_idx: usize,
        clip_id: String,
        path: String,
    ) -> Result<(), String> {
        let (notes, bpm) = {
            let tracks = self.tracks.lock().unwrap();
            let track = tracks.get(track_idx).ok_or("Track not found")?;
            let clip = track
                .midi_clips
                .iter()
                .find(|c| c.id.to_string() == clip_id)
                .ok_or("Midi Clip not found")?;
            (clip.notes.clone(), *self.bpm.lock().unwrap())
        };

        // Constants
        const PPQ: f64 = 960.0;
        let sample_rate = 48000.0;

        let samples_to_ticks =
            |s: u64| -> u32 { ((s as f64 * bpm as f64 * PPQ) / (sample_rate * 60.0)) as u32 };

        let mut events: Vec<AbsEvent> = Vec::new();

        for note in notes {
            let start = samples_to_ticks(note.start_sample);
            let end = samples_to_ticks(note.start_sample + note.length_samples);

            events.push(AbsEvent {
                time: start,
                kind: SmfEventKind::NoteOn(note.note as u8, note.velocity.min(127) as u8),
                channel: note.channel,
            });
            events.push(AbsEvent {
                time: end,
                kind: SmfEventKind::NoteOff(note.note as u8),
                channel: note.channel,
            });
        }

        events.sort_by(|a, b| a.time.cmp(&b.time));

        // Add EndOfTrack
        let last_time = events.last().map(|e| e.time).unwrap_or(0);
        events.push(AbsEvent {
            time: last_time + 960,
            kind: SmfEventKind::EndOfTrack,
            channel: 0,
        });

        // Write File
        let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        use std::io::Write;

        // MThd
        file.write_all(b"MThd").map_err(|e| e.to_string())?;
        file.write_all(&6u32.to_be_bytes())
            .map_err(|e| e.to_string())?; // Chunk Len
        file.write_all(&0u16.to_be_bytes())
            .map_err(|e| e.to_string())?; // Format 0
        file.write_all(&1u16.to_be_bytes())
            .map_err(|e| e.to_string())?; // 1 Track
        file.write_all(&(PPQ as u16).to_be_bytes())
            .map_err(|e| e.to_string())?; // Division

        // MTrk
        let mut track_data = Vec::new();
        let mut current_time = 0;
        let mut running_status = 0u8;

        for e in events {
            let delta = e.time - current_time;
            current_time = e.time;

            write_vlq(&mut track_data, delta);

            match e.kind {
                SmfEventKind::NoteOn(note, vel) => {
                    let status = 0x90 | (e.channel & 0x0F);
                    if status != running_status {
                        track_data.push(status);
                        running_status = status;
                    }
                    track_data.push(note);
                    track_data.push(vel);
                }
                SmfEventKind::NoteOff(note) => {
                    let status = 0x80 | (e.channel & 0x0F);
                    if status != running_status {
                        track_data.push(status);
                        running_status = status;
                    }
                    track_data.push(note);
                    track_data.push(0);
                }
                SmfEventKind::EndOfTrack => {
                    track_data.push(0xFF);
                    track_data.push(0x2F);
                    track_data.push(0x00);
                }
            }
        }

        file.write_all(b"MTrk").map_err(|e| e.to_string())?;
        file.write_all(&(track_data.len() as u32).to_be_bytes())
            .map_err(|e| e.to_string())?;
        file.write_all(&track_data).map_err(|e| e.to_string())?;

        Ok(())
    }
}

// --- MIDI Export Helpers ---
struct AbsEvent {
    time: u32,
    kind: SmfEventKind,
    channel: u8,
}
enum SmfEventKind {
    NoteOn(u8, u8),
    NoteOff(u8),
    EndOfTrack,
}
fn write_vlq(vec: &mut Vec<u8>, value: u32) {
    let mut buffer = Vec::new();
    let mut val = value;

    buffer.push((val & 0x7F) as u8);
    val >>= 7;

    while val > 0 {
        buffer.push(((val & 0x7F) | 0x80) as u8);
        val >>= 7;
    }

    for b in buffer.iter().rev() {
        vec.push(*b);
    }
}
pub fn read_full_samples(path: String) -> Result<(Vec<f32>, u32, u32), String> {
    use symphonia::core::audio::Signal;
    use symphonia::core::formats::FormatOptions;
    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let hint = symphonia::core::probe::Hint::new();
    // hint.with_extension("wav"); // Don't restrict hint?

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &Default::default())
        .map_err(|e| e.to_string())?;

    let mut format = probed.format;
    let stream = format.default_track().ok_or("No default track")?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&stream.codec_params, &Default::default())
        .map_err(|e| e.to_string())?;

    let mut raw_samples_interleaved = Vec::new();
    let sample_rate = stream.codec_params.sample_rate.unwrap_or(48000);
    // Determine channels from spec or codec params
    // We'll get it from buffer spec eventually, but codec_params is a good hint
    let mut channel_count = 2; // Default

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet).map_err(|e| e.to_string())?;
        let spec = *decoded.spec();
        channel_count = spec.channels.count() as u32;

        let mut buffer =
            symphonia::core::audio::AudioBuffer::<f32>::new(decoded.capacity() as u64, spec);
        decoded.convert(&mut buffer);

        // Interleave
        for i in 0..buffer.frames() {
            for c in 0..spec.channels.count() {
                raw_samples_interleaved.push(buffer.chan(c)[i]);
            }
        }
    }

    Ok((raw_samples_interleaved, channel_count, sample_rate))
}
