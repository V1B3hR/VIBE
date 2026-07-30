mod commands;
mod engine;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let audio_engine = engine::AudioEngine::new();
            let library_service = engine::library_service::LibraryService::new();
            let plugin_manager = engine::plugin_manager::PluginManager::new();

            // Initialize NeuralForest Sidecar natively inside VIBE
            let script_path = std::env::current_dir().unwrap().join("service.py");
            let script_path_str = script_path.to_string_lossy().to_string();
            
            // Using system Python. The user should have Python installed or use Ollama directly.
            #[cfg(target_os = "windows")]
            let python_path_str = "python".to_string();
            #[cfg(not(target_os = "windows"))]
            let python_path_str = "python3".to_string();

            let bridge = std::sync::Arc::new(tokio::sync::Mutex::new(
                engine::neural_forest::NeuralForestBridge::new(python_path_str, script_path_str),
            ));

            // Spawn the start process in background
            let bridge_clone = bridge.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = bridge_clone.lock().await.start().await {
                    log::error!("Failed to start NeuralForest: {}", e);
                }
            });

            audio_engine.kropelka_brain.lock().unwrap().attach_brain(bridge);

            app.manage(AppState {
                audio_engine: Mutex::new(audio_engine),
                library_service: Mutex::new(library_service),
                plugin_manager: Mutex::new(plugin_manager),
            });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Transport / Global
            commands::audio::get_audio_hosts,
            commands::audio::get_audio_devices,
            commands::audio::get_buffer_sizes,
            commands::audio::get_sample_rates,
            commands::audio::get_current_audio_config,
            commands::audio::set_audio_config,
            commands::audio::play_audio,
            commands::audio::pause_audio,
            commands::audio::stop_transport,
            commands::audio::toggle_record,
            commands::audio::set_bpm,
            commands::audio::set_global_swing,
            commands::audio::get_bpm,
            commands::audio::set_metronome,
            commands::audio::set_loop_enabled,
            commands::audio::set_loop_range,
            commands::audio::is_loop_enabled,
            commands::audio::get_loop_range,
            commands::audio::get_playhead,
            commands::audio::set_playhead,
            commands::audio::is_playing,
            commands::audio::is_recording,
            commands::audio::get_cpu_load,
            commands::audio::get_memory_usage,
            commands::audio::get_master_meters,
            commands::audio::get_track_levels,
            commands::audio::get_analyzer_data,
            commands::audio::get_scope_data,
            commands::audio::set_time_signature,
            commands::audio::set_global_quantization,
            commands::audio::get_input_aliases,
            commands::audio::create_input_alias,
            commands::audio::delete_input_alias,
            commands::audio::update_input_alias,
            commands::audio::get_channel_meters,
            commands::audio::assign_track_input,
            commands::audio::add_marker,
            commands::audio::remove_marker,
            commands::audio::get_markers,
            // Video Sync
            commands::audio::load_video,
            commands::audio::unload_video,
            commands::audio::get_video_state,
            commands::audio::set_video_offset,
            commands::audio::set_video_framerate,
            // Project
            commands::project::new_project,
            commands::project::save_project,
            commands::project::save_project_file,
            commands::project::load_project_file,
            commands::project::check_autosave_exists,
            commands::project::recover_from_autosave,
            commands::project::delete_autosave,
            commands::project::export_project,
            commands::project::undo,
            commands::project::redo,
            commands::project::get_history_graph,
            commands::project::get_current_node,
            commands::project::get_branches,
            commands::project::checkout_node,
            commands::project::create_branch,
            // System
            commands::system::log_frontend_action,
            commands::system::log_frontend_error,
            commands::system::show_in_explorer,
            commands::system::save_file_dialog,
            commands::system::open_file_dialog,
            // Tracks
            commands::track::add_track,
            commands::track::create_track,
            commands::track::create_track_with_clip,
            commands::library::get_library,
            commands::track::create_track_with_parent,
            commands::track::remove_track,
            commands::track::move_track,
            commands::track::duplicate_track,
            commands::track::rename_track,
            commands::track::set_track_color,
            commands::track::set_track_volume,
            commands::track::set_track_mute,
            commands::track::set_track_solo,
            commands::track::set_track_pan,
            commands::track::set_track_width,
            commands::track::set_track_phase_invert,
            commands::track::set_track_drive,
            commands::track::set_track_arm,
            commands::track::set_track_disabled,
            commands::track::set_track_frozen,
            commands::track::set_track_collapsed,
            commands::track::set_track_type,
            commands::track::set_track_parent,
            commands::track::set_track_automation_mode,
            commands::track::set_track_input,
            commands::track::set_track_output,
            commands::track::set_track_sidechain,
            commands::track::get_tracks,
            commands::track::add_effect,
            commands::track::set_effect_bypass,
            commands::track::move_effect,
            commands::track::remove_effect,
            commands::track::set_parameter,
            commands::track::set_eq_bands,
            commands::track::get_eq_bands,
            commands::track::update_eq_band,
            commands::track::get_compressor_metrics,
            commands::track::load_synth_preset,
            commands::track::save_synth_preset,
            commands::track::list_synth_presets,
            commands::track::update_mod_matrix,
            commands::track::add_plugin_to_track,
            commands::track::get_eq_presets,
            commands::track::open_plugin_editor,
            commands::track::get_plugin_state,
            commands::track::set_plugin_state,
            commands::track::get_plugin_latency,
            commands::track::save_plugin_preset,
            commands::track::load_plugin_preset,
            commands::track::list_plugin_presets,
            commands::track::poll_plugin_param_changes,
            commands::track::get_plugin_programs,
            commands::track::set_plugin_program,
            commands::track::get_plugin_cpu_usage,
            commands::track::poll_plugin_resize,
            commands::track::get_master_info,
            commands::track::add_bus,
            commands::track::create_track_group,
            commands::track::graph_add_node,
            commands::track::graph_remove_node,
            commands::track::graph_connect,
            commands::track::graph_disconnect,
            // Clips
            commands::clip::add_clip_to_track,
            commands::clip::slice_clip,
            commands::clip::move_clip,
            commands::clip::resize_clip,
            commands::clip::delete_clip,
            commands::clip::rename_clip,
            commands::clip::reverse_audio_clip,
            commands::clip::duplicate_clip,
            commands::clip::set_clip_gain,
            commands::clip::normalize_clip,
            commands::clip::consolidate_clip,
            commands::clip::export_audio_clip,
            commands::clip::export_midi_clip,
            commands::clip::set_audio_clip_warp_mode,
            commands::clip::set_clip_color,
            commands::clip::add_midi_note,
            commands::clip::delete_midi_note,
            commands::clip::update_midi_note,
            commands::clip::get_track_midi_clips,
            commands::clip::quantize_notes,
            commands::clip::apply_groove_custom,
            commands::clip::humanize_midi_clip,
            commands::clip::convert_audio_to_midi,
            commands::clip::extract_groove,
            commands::clip::transpose_midi_clip,
            commands::clip::duplicate_midi_notes,
            commands::clip::get_midi_clip_data,
            commands::clip::set_clip_scale,
            commands::clip::detect_chords,
            commands::clip::add_automation_point,
            commands::clip::set_automation_tension,
            commands::clip::clear_automation,
            commands::clip::get_waveform_chunk,
            commands::clip::get_raw_samples,
            commands::clip::get_clip_data,
            commands::clip::snap_loop_to_zero,
            commands::clip::get_clip_statistics,
            commands::clip::set_automation_interpolation,
            commands::clip::set_automation_layer,
            commands::clip::set_clip_envelope,
            commands::clip::add_midi_clip,
            commands::clip::delete_midi_clip,
            commands::clip::update_midi_clip,
            commands::clip::apply_groove_template,
            commands::clip::generate_stress_notes,
            // Comping & Freeze
            commands::comping::add_take_lane,
            commands::comping::select_comp_region,
            commands::comping::freeze_track_cmd,
            commands::comping::bounce_track_in_place_cmd,
            // VCA & Sidechain Spectral Masking
            commands::vca::create_vca_group_cmd,
            commands::vca::add_track_to_vca_cmd,
            commands::vca::remove_track_from_vca_cmd,
            commands::vca::set_vca_gain_cmd,
            commands::sidechain_spectral::get_sidechain_spectrum_comparison,
            // MCU & 3D Spatial Panner
            commands::mcu::connect_mcu_device_cmd,
            commands::mcu::send_mcu_display_text_cmd,
            commands::mcu::process_mcu_input_cmd,
            commands::spatial::calculate_714_spatial_gains_cmd,
            commands::spatial::calculate_binaural_hrtf_cmd,
            // Library / Plugins
            commands::library::get_library,
            commands::library::import_to_library,
            commands::library::import_audio_file,
            commands::library::library_search,
            commands::library::library_add_directory,
            commands::library::plugin_scan_directory,
            commands::library::scan_plugins,
            commands::library::import_plugin,
            commands::library::plugin_get_all,
            commands::library::plugin_search,
            commands::library::plugin_get_by_type,
            commands::library::plugin_get_by_category,
            commands::library::plugin_toggle_favorite,
            commands::library::plugin_get_favorites,
            commands::library::preview_play,
            commands::library::preview_sample_synced,
            commands::library::preview_stop,
            commands::library::stop_preview,
            commands::library::add_wasm_plugin,
            commands::library::create_audio_track,
            commands::library::plugin_delete_chain,
            commands::library::plugin_add_search_path,
            commands::library::plugin_remove_search_path,
            commands::library::plugin_handle_blacklist,
            commands::library::plugin_set_hidden,
            commands::library::plugin_set_deprecated,
            commands::library::plugin_merge_duplicates,
            commands::library::plugin_set_custom_folder,
            commands::library::plugin_update_last_used,
            // MIDI
            commands::midi::note_on,
            commands::midi::note_off,
            commands::midi::map_midi,
            commands::midi::start_midi_learn,
            commands::midi::remove_midi_binding,
            commands::midi::get_midi_bindings,
            commands::midi::add_cc_event,
            // Arrangement
            commands::arrangement::set_comp_mode,
            commands::arrangement::set_active_take,
            commands::arrangement::add_take_from_selection,
            commands::arrangement::add_playlist,
            commands::arrangement::set_active_playlist,
            commands::arrangement::paste_time,
            commands::arrangement::insert_silence,
            commands::arrangement::delete_time,
            commands::arrangement::duplicate_time,
            // Kropelka
            commands::kropelka::get_kropelka_suggestion,
            commands::kropelka::trigger_vibe_check,
            commands::kropelka::detect_project_key,
            commands::kropelka::apply_kropelka_fix,
            commands::kropelka::get_structure_analysis,
            commands::kropelka::reject_kropelka_suggestion,
            commands::kropelka::get_ai_server_status,
            commands::kropelka::get_kropelka_stats,
            commands::kropelka::reset_kropelka_memory,
            commands::kropelka::get_assistant_knowledge_tip,
            commands::kropelka::query_plugin_database,
            commands::kropelka::generate_drum_pattern,
            commands::kropelka::generate_melody_pattern,
            commands::kropelka::generate_chord_pattern,
            commands::kropelka::generate_intelligent_arrangement,
            commands::kropelka::trigger_zosia_activity,
            commands::kropelka::trigger_zosia_audit,
            // Groove
            commands::groove::get_groove_templates,
            commands::groove::get_shadow_grid,
            commands::groove::extract_groove_from_track,
            // Audio to MIDI
            commands::audio_to_midi::convert_audio_clip_to_midi,
            // Spectral
            commands::spectral::analyze_spectral,
            commands::spectral::convert_clip_to_drums,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
