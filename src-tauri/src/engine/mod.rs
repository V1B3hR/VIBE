pub mod ara2_bridge;
pub mod arena;
pub mod audio;
pub mod audio_commands;
pub mod audio_device;
pub mod audio_graph;
pub mod audio_preview;
pub mod audio_types;
pub mod audio_utils;
pub mod automation;
pub mod comping;
pub mod convolution_reverb;
pub mod diagnostics;
pub mod dynamics_module;
pub mod effects;
pub mod eq_module;
pub mod fades;
pub mod freeze;
pub mod graph;
#[cfg(test)]
pub mod graph_tests;
pub mod history;
pub mod io_manager;
pub mod library_service;
pub mod lockfree_params;
pub mod mcu_protocol;
pub mod metering;
pub mod midi;
pub mod midi_mapping;
pub mod multiband_dynamics;
pub mod oversampling;
pub mod parallel;
pub mod pdc;
pub mod persistence;
pub mod plugin_manager;
pub mod processors;
pub mod psycho;
pub mod recovery;
pub mod remote_dsp;
pub mod render_engine;
pub mod resampler;
pub mod routing;
pub mod sandbox;
pub mod sandbox_v2;
pub mod scanner;
pub mod security_utils;
pub mod simd;
pub mod simd_avx512;
pub mod simd_optimized;
pub mod spatial_panner;
pub mod spectral_gate;
pub mod spectrum;
pub mod sound_similarity;
pub mod stereo_imager;
pub mod streamer;
pub mod summing;
pub mod synth;
pub mod velocity;
#[cfg(test)]
pub mod velocity_tests;
pub mod vst3_bridge;
pub mod warp_engine;
pub mod wasm;
pub mod wasm_processor;
pub mod waveform;

pub use audio::AudioEngine;

// Faza 1: Advanced Time & Frequency
pub mod audio_quantize;
pub mod auto_warp;
pub mod formant_preservation;
pub mod pitch_shift;
pub mod time_stretch;
pub mod transient_detection;

// Faza 2: Pro Production Workflow
pub mod bounce_manager;
pub mod comping_engine;
pub mod sidechain_manager;
pub mod take_lanes;
pub mod track_freeze;

// Faza 2a: Sonic Sculpture & Kinetic Energy
pub mod global_lfo;
pub mod macro_host;
pub mod mod_matrix;
pub mod unmod;
pub mod mseg;
pub mod spectral_analysis;
pub mod vca_group;

// Faza 3: Intelligence & Groove
pub mod audio_to_midi;
pub mod generators;
pub mod gosposia;
pub mod groove_pool;
pub mod humanization_engine;
pub mod kropelka;
pub mod kropelka_brain;
pub mod mix_analyzer;
pub mod neural_forest;
pub mod pitch_detection;
pub mod spectral;
pub mod technik;
pub mod theory;

// Faza 4: Interaction & Expression
pub mod clip_launcher;
pub mod daw_importer;
pub mod control_surface_profiles;
pub mod macro_engine;
pub mod midi2_support;
pub mod mpe_handler;
pub mod scene_manager;

// Faza 5: Standards & Precision
pub mod video_manager;
pub mod disk_writer;
pub mod cloud_manager;
pub mod hardware_calibration;
pub mod step_sequencer;
pub mod control_surface_manager;

#[cfg(test)]
pub mod arrangement_tests;
#[cfg(test)]
pub mod audio_tests;
#[cfg(test)]
pub mod engine_tests;
#[cfg(test)]
pub mod integration_tests;
#[cfg(test)]
pub mod stress_tests;
#[cfg(test)]
pub mod ai_tests;
#[cfg(test)]
pub mod vibe_v2_tests;
