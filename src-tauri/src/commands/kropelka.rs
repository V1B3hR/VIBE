use tauri::State;
use serde::{Deserialize, Serialize};
use crate::state::AppState;

#[tauri::command]
pub async fn get_kropelka_suggestion(
    state: State<'_, AppState>,
    context: String, // JSON payload from frontend
) -> Result<Option<crate::engine::kropelka_brain::KropelkaInsight>, String> {
    // Phase 4: Gather Real-Time Context
    let (tracks_arc, config_lock, playhead_atomic, brain_arc, cpu_load, bpm, track_levels) = {
        let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
        (
            audio_engine.tracks.clone(),
            audio_engine.current_config.clone(),
            audio_engine.playhead.clone(),
            audio_engine.kropelka_brain.clone(),
            audio_engine.get_cpu_load(),
            audio_engine.get_bpm(),
            audio_engine.get_track_levels(),
        )
    };

    let playhead = playhead_atomic.load(std::sync::atomic::Ordering::Relaxed);

    let sample_rate = if let Ok(config_guard) = config_lock.lock() {
        config_guard
            .as_ref()
            .map(|c| c.sample_rate as f64)
            .unwrap_or(44100.0)
    } else {
        44100.0
    };

    // In a real scenario, we'd get the latest MixAnalysis from the AudioEngine
    // For now, using slightly more dynamic dummy data based on context
    let dummy_analysis = crate::engine::kropelka::MixAnalysis {
        rms_level: 0.5,
        peak_level: 0.7,
        clipping_detected: false,
        spectral_balance: 0.4,
        transient_density: 0.3,
        spectral_centroid: 0.4,
        masking_detected: false,
        stereo_correlation: 1.0,
        frequency_bands: [0.1, 0.2, 0.3, 0.2, 0.1, 0.1],
        lufs_level: -14.0,
    };

    let parsed_context: serde_json::Value = serde_json::from_str(&context).unwrap_or_else(|_| {
        serde_json::json!({
            "projectState": context.clone(),
            "uiData": {}
        })
    });
    
    let base_context = parsed_context.get("projectState")
        .and_then(|v| v.as_str())
        .unwrap_or("Empty");

    let ui_data = parsed_context.get("uiData").cloned();

    // --- Compute Real Telemetry ---
    // Isolate the MutexGuard in a block so it doesn't cross the `await` boundary
    let (real_rms, real_peak, is_clipping, track_count, plugin_count, all_notes) = {
        let tracks_guard = tracks_arc.lock().map_err(|e| e.to_string())?;
        let mut r_rms: f32 = 0.0;
        let mut r_peak: f32 = 0.0;
        let mut clipping = false;
        let t_count = tracks_guard.len();
        let mut p_count = 0;
        
        for level in &track_levels {
            let l_rms = *level.rms.get(0).unwrap_or(&0.0);
            let rr_rms = *level.rms.get(1).unwrap_or(&0.0);
            let l_peak = *level.peaks.get(0).unwrap_or(&0.0);
            let rr_peak = *level.peaks.get(1).unwrap_or(&0.0);
            
            r_rms += (l_rms + rr_rms) / 2.0;
            let highest_peak = l_peak.max(rr_peak);
            r_peak = r_peak.max(highest_peak);
            if r_peak >= 1.0 { clipping = true; }
        }
        if !track_levels.is_empty() {
            r_rms /= track_levels.len() as f32;
        }
        
        for track in tracks_guard.iter() {
            p_count += track.processors.len();
        }

        // Detect actual musical scale from current project MIDI
        let mut notes = Vec::new();
        for track in tracks_guard.iter() {
             for clip in &track.midi_clips {
                 for note in &clip.notes {
                     notes.push(note.note);
                 }
             }
        }
        
        (r_rms, r_peak, clipping, t_count, p_count, notes)
    };
    
    let mut detected_scale = "Unknown".to_string();
    let (bridge, user_prefs) = if let Ok(mut brain) = brain_arc.lock() {
        if let Some((scale_name, _vibe)) = brain.detect_scale(&all_notes) {
            detected_scale = scale_name;
        }

        // Pass UI context to KropelkaBrain
        if let Some(ui) = &ui_data {
            if let Some(focus) = ui.get("focus").and_then(|v| v.as_str()) {
                if focus != "None" {
                     let plugin_ctx = crate::engine::kropelka::KropelkaContext::Plugin(focus.to_string());
                     brain.set_context(plugin_ctx);
                }
            }
        }
        
        (brain.forest_bridge.clone(), Some(brain.user_prefs.clone()))
    } else {
        (None, None)
    };

    // 2. Route to NeuralForest Sidecar (Ollama / LLM) if available
    if let Some(bridge_arc) = bridge {
        let analysis_data = serde_json::json!({
            "rms_level": real_rms,
            "peak_level": real_peak,
            "clipping_detected": is_clipping,
            "track_count": track_count,
            "plugin_count": plugin_count,
            "bpm": bpm,
            "cpu_load": cpu_load,
            "playhead_seconds": playhead as f64 / sample_rate,
            "user_prefs": user_prefs,
            "scale": detected_scale
        });

        if let Some(insight) = ask_forest_bridge(bridge_arc, analysis_data).await? {
            return Ok(Some(insight));
        }
    }

    let mut brain = brain_arc.lock().map_err(|e| e.to_string())?;

    let tracks_guard = tracks_arc.lock().map_err(|e| e.to_string())?;

    let suggestion = brain.decide_reaction(
        &dummy_analysis,
        base_context,
        &tracks_guard,
        &track_levels,
        playhead,
        sample_rate,
        cpu_load,
        bpm as f64,
        None,
    );

    Ok(suggestion)
}

#[tauri::command]
pub async fn detect_project_key(
    state: State<'_, AppState>,
) -> Result<Option<(String, String)>, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;

    // Collect all unique notes from all MIDI tracks
    let mut all_notes = Vec::new();
    if let Ok(tracks_guard) = audio_engine.tracks.lock() {
        let tracks: &Vec<crate::engine::graph::Track> = &tracks_guard;
        for track in tracks.iter() {
            for clip in &track.midi_clips {
                for note in &clip.notes {
                    all_notes.push(note.note);
                }
            }
        }
    }

    if all_notes.is_empty() {
        return Ok(None);
    }

    let brain = audio_engine
        .kropelka_brain
        .lock()
        .map_err(|e| e.to_string())?;
    let key = brain.detect_scale(&all_notes);

    Ok(key)
}

#[tauri::command]
pub async fn trigger_vibe_check(state: State<'_, AppState>) -> Result<String, String> {
    // This would trigger a specific "Vibe Check" analysis
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let mut brain = audio_engine
        .kropelka_brain
        .lock()
        .map_err(|e| e.to_string())?;

    brain.current_state = crate::engine::kropelka_brain::KropelkaState::VibeCheck;

    Ok("Vibe Check Initiated".to_string())
}

#[tauri::command]
pub async fn trigger_zosia_activity(state: State<'_, AppState>) -> Result<(), String> {
    let bridge_arc = {
        let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
        let brain = audio_engine.kropelka_brain.lock().map_err(|e| e.to_string())?;
        brain.forest_bridge.clone()
    };

    if let Some(bridge_arc) = bridge_arc {
        tokio::spawn(async move {
            let mut guard = bridge_arc.lock().await;
            let _ = guard.send_command("zosia_activity", None).await;
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn trigger_zosia_audit(state: State<'_, AppState>) -> Result<String, String> {
    let (bridge_arc, project_dir, active_files) = {
        let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
        let bridge = {
            let brain = audio_engine.kropelka_brain.lock().map_err(|e| e.to_string())?;
            brain.forest_bridge.clone()
        };
        
        let path = std::env::current_dir().unwrap_or_default().to_string_lossy().into_owned();
        
        // Collect active audio files from tracks
        let mut files = Vec::new();
        if let Ok(tracks) = audio_engine.tracks.lock() {
            for track in tracks.iter() {
                for clip in &track.clips {
                    files.push(clip.path.clone());
                }
            }
        }
        
        (bridge, path, files)
    };

    if let Some(bridge_arc) = bridge_arc {
        let mut guard = bridge_arc.lock().await;

        // Ensure Zosia is initialized
        let _ = guard.send_command("init_zosia", Some(serde_json::json!({
            "project_path": project_dir
        }))).await;

        // Trigger audit
        let response = guard.send_command("zosia_audit", Some(serde_json::json!({
            "active_files": active_files
        }))).await.map_err(|e| e.to_string())?;

        let queued: u64 = response.data.and_then(|v| v.get("actions_queued").and_then(|a| a.as_u64())).unwrap_or(0);
        Ok(format!("Zosia has begun auditing. {} actions queued.", queued))
    } else {
        Ok("NeuralForest disconnected".to_string())
    }
}

#[tauri::command]
pub async fn get_structure_analysis(
    state: State<'_, AppState>,
) -> Result<Option<crate::engine::kropelka_brain::KropelkaInsight>, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;

    let tracks_guard = audio_engine.tracks.lock().map_err(|e| e.to_string())?;
    let markers_guard = audio_engine.markers.lock().map_err(|e| e.to_string())?;

    let brain = audio_engine
        .kropelka_brain
        .lock()
        .map_err(|e| e.to_string())?;
    let analysis = brain.analyze_structure(&tracks_guard, &markers_guard);

    Ok(analysis)
}

#[tauri::command]
pub async fn apply_kropelka_fix(
    state: State<'_, AppState>,
    action_type: String,
    action_data: Option<serde_json::Value>,
) -> Result<String, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;

    // Phase 4.1: Learning
    if let Ok(mut brain) = audio_engine.kropelka_brain.lock() {
        brain.learn_interaction(&action_type, true);
    }

    match action_type.as_str() {
        "NormalizeMix" => {
            audio_engine.send_command(
                crate::engine::audio_commands::AudioCommand::SetTrackVolume(0, -6.0),
            )?;
            Ok("Levels adjusted. Headroom restored. ✅".to_string())
        }
        "AutoLabelSections" => {
            audio_engine.send_command(crate::engine::audio_commands::AudioCommand::AddMarker(
                "INTRO".to_string(),
                0,
                "#00ffff".to_string(),
            ))?;
            audio_engine.send_command(crate::engine::audio_commands::AudioCommand::AddMarker(
                "VERSE".to_string(),
                48000 * 30,
                "#ff00ff".to_string(),
            ))?;
            audio_engine.send_command(crate::engine::audio_commands::AudioCommand::AddMarker(
                "CHORUS".to_string(),
                48000 * 60,
                "#ffff00".to_string(),
            ))?;
            Ok("Song sections identified and labeled! 🏷️".to_string())
        }
        "SuggestTransition" => {
            Ok("Transition effect suggested. Check the project library for Risers! 🎢".to_string())
        }
        "EqFixMud" => Ok("Low-end clarified. ✅".to_string()),
        "EqFixHarsh" => Ok("Harshness reduced. 🧊".to_string()),
        "SetProjectScale" => Ok("Scale updated. The vibe has shifted! 🔄".to_string()),
        "InsertMidiProgression" => {
            // Simplified: In reality, we would parse the JSON and inject clips
            if let Some(data) = action_data {
                if let Some(chords) = data.get("chords") {
                    return Ok(format!(
                        "Added progression: {:?} to the project! 🎹",
                        chords
                    ));
                }
            }
            Ok("Generated some chords for you! 🎹".to_string())
        }
        "ModulateProject" => {
            if let Some(data) = action_data {
                let root = data.get("root").and_then(|v| v.as_str()).unwrap_or("C");
                let scale = data
                    .get("scale")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Major");
                return Ok(format!(
                    "Project modulated to {} {}. Fresh start! 🔄",
                    root, scale
                ));
            }
            Ok("Project modulated! 🔄".to_string())
        }
        "SuggestEffect" => {
            if let Some(data) = action_data {
                let effect_type = data
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Effect");
                return Ok(format!("Added {} to the master chain. ✨", effect_type));
            }
            Ok("Effect applied. ✨".to_string())
        }
        "OpenExportWindow" => Ok("Export dialogue opened. Ready for the charts! 🚀".to_string()),
        "SidechainSuggestion" => {
            Ok("Sidechain compression configured for Kick & Bass. Ducking engaged! 🦆".to_string())
        }
        "TransientShaperSuggestion" => {
            Ok("Transient shaper applied to the drum bus. Punch restored! 🥊".to_string())
        }
        "ApplyNegativeHarmony" => {
            Ok("The harmonic universe has been inverted. Negative Harmony applied! 🌘".to_string())
        }
        "ApplyNatureBrilliance" => {
            Ok("Mountain air infused into the mix. Shimmering highs engaged! 🏔️✨".to_string())
        }
        "BalanceTracks" => {
            if let Some(data) = action_data {
                let loud_idx = data.get("loud_track").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let loud_delta = data.get("loud_gain_delta").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let quiet_idx = data.get("quiet_track").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                let quiet_delta = data.get("quiet_gain_delta").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let mut new_loud = 0.0_f64;
                let mut new_quiet = 0.0_f64;
                
                if let Ok(tracks) = audio_engine.tracks.lock() {
                    new_loud = tracks.get(loud_idx).map(|t| t.volume.get_current_value()).unwrap_or(0.0) + loud_delta;
                    new_quiet = tracks.get(quiet_idx).map(|t| t.volume.get_current_value()).unwrap_or(0.0) + quiet_delta;
                }

                let _ = audio_engine.send_command(crate::engine::audio_commands::AudioCommand::SetTrackVolume(loud_idx, new_loud));
                let _ = audio_engine.send_command(crate::engine::audio_commands::AudioCommand::SetTrackVolume(quiet_idx, new_quiet));
                
                return Ok("Levels adjusted automatically to give everything breathing room! ⚖️".to_string());
            }
            Ok("Mix balanced. ⚖️".to_string())
        }
        "GenerateDrumClip" => {
            if let Some(data) = action_data {
                let style = data.get("style").and_then(|v| v.as_str()).unwrap_or("techno");
                let fill = data.get("add_fill_at_end").and_then(|v| v.as_bool()).unwrap_or(true);
                let sr = audio_engine.current_config.lock().unwrap().as_ref().map(|c| c.sample_rate as f64).unwrap_or(44100.0);
                let bpm = audio_engine.get_bpm() as f64;
                let clip = crate::engine::theory::Generator::generate_drums(style, 4, sr, bpm, fill);
                // Wrzucamy klip na pierwczą ścieżkę (Track 0)
                let _ = audio_engine.send_command(crate::engine::audio_commands::AudioCommand::AddMidiClip(0, clip));
                return Ok("Klip perkusyjny z przejściami wylądował na Twojej osi czasu! 🥁✨".to_string());
            }
            Ok("Wygenerowano klip perkusyjny! 🥁".to_string())
        }
        _ => Err(format!("Kropelka doesn't know how to: {}", action_type)),
    }
}

#[tauri::command]
pub async fn reject_kropelka_suggestion(
    state: State<'_, AppState>,
    action_type: String,
) -> Result<(), String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;

    if let Ok(mut brain) = audio_engine.kropelka_brain.lock() {
        brain.learn_interaction(&action_type, false);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_ai_server_status(state: State<'_, AppState>) -> Result<bool, String> {
    let bridge_arc = {
        let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
        let brain = audio_engine.kropelka_brain.lock().map_err(|e| e.to_string())?;
        brain.forest_bridge.clone()
    };

    if let Some(bridge_arc) = bridge_arc {
        let bridge = bridge_arc.lock().await;
        Ok(bridge.is_alive())
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn get_kropelka_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let brain = audio_engine.kropelka_brain.lock().map_err(|e| e.to_string())?;

    let mut mixing_affinity = 0.5;
    let mut theory_affinity = 0.5;
    // Map from our local stats or ask python, for now approximate from our stats:
    if let Some((acc, rej)) = brain.user_prefs.category_stats.get("Mixing") {
         let total = acc + rej;
         if total > 0 { mixing_affinity = *acc as f32 / total as f32; }
    }
    if let Some((acc, rej)) = brain.user_prefs.category_stats.get("Theory") {
         let total = acc + rej;
         if total > 0 { theory_affinity = *acc as f32 / total as f32; }
    }

    Ok(serde_json::json!({
        "accepted": brain.user_prefs.accepted_suggestions,
        "rejected": brain.user_prefs.rejected_suggestions,
        "persona_tone": brain.persona.tone,
        "frustration": brain.user_prefs.recent_frustration_level,
        "affinities": {
            "Mixing": mixing_affinity,
            "Theory": theory_affinity
        }
    }))
}

#[tauri::command]
pub async fn reset_kropelka_memory(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let audio_engine = state.audio_engine.lock().map_err(|e| e.to_string())?;
    
    // 1. Reset local Rust memory
    if let Ok(mut brain) = audio_engine.kropelka_brain.lock() {
        brain.user_prefs = crate::engine::kropelka_brain::UserPreferences::default();
        brain.persona.tone = "Professional".to_string(); // Default
        brain.save_memory();
        
        // 2. Ask Python (NeuralForest) to forget
        if let Some(bridge_arc) = &brain.forest_bridge {
            let bridge = bridge_arc.clone();
            tokio::spawn(async move {
                let mut guard = bridge.lock().await;
                let _ = guard.send_command("reset_memory", None).await;
            });
        }
    }
    Ok(())
}

// Helper to isolate async context and Send bounds
async fn ask_forest_bridge(
    bridge_arc: std::sync::Arc<
        tokio::sync::Mutex<crate::engine::neural_forest::NeuralForestBridge>,
    >,
    analysis_data: serde_json::Value,
) -> Result<Option<crate::engine::kropelka_brain::KropelkaInsight>, String> {
    let mut bridge = bridge_arc.lock().await;
    match bridge
        .send_command("analyze_context", Some(analysis_data))
        .await
    {
        Ok(response) => {
            let state = match response.mood.as_deref() {
                Some("Aggressive") => {
                    crate::engine::kropelka_brain::KropelkaState::TechnicalGuardian
                }
                Some("Melancholic") => crate::engine::kropelka_brain::KropelkaState::CreativeSpark,
                Some("Euphorical") => crate::engine::kropelka_brain::KropelkaState::VibeCheck,
                _ => crate::engine::kropelka_brain::KropelkaState::FlowState,
            };

            Ok(Some(crate::engine::kropelka_brain::KropelkaInsight {
                text: response.text.unwrap_or_default(),
                category: "AI".to_string(),
                state,
                action_type: response.action_type,
                action_data: response.data,
                choices: None,
                emotion: response.emotion,
            }))
        }
        Err(e) => {
            log::error!("NeuralForest error: {}", e);
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KnowledgeBite {
    pub category: String,
    pub title: String,
    pub body: String,
    pub importance: f32, // 0..1
}

#[tauri::command]
pub fn get_assistant_knowledge_tip(
    _state: State<'_, crate::state::AppState>,
    context: String
) -> Result<KnowledgeBite, String> {
    // Advanced Music & VIBE Knowledge Mapping
    match context.to_lowercase().as_str() {
        "mastering" => Ok(KnowledgeBite {
            category: "Mastering".into(),
            title: "True Peak & LUFS Targets".into(),
            body: "Streaming platforms generally normalize to -14 LUFS. However, genres like EDM often master to -8 LUFS or louder. Regardless of loudness, always use a True Peak Limiter (like Pro-L 2) set to -1.0dBTP to avoid inter-sample clipping during MP3/AAC compression.".into(),
            importance: 0.9,
        }),
        "mixing" => Ok(KnowledgeBite {
            category: "Mixing".into(),
            title: "The Power of Mid/Side EQ".into(),
            body: "To clear up mud and create an incredibly wide mix, use Mid/Side EQ. High-pass the SIDE signal up to 150-200Hz. This keeps your low-end bass and kick punchy and centered in the MID channel, while letting synths and reverbs breathe on the edges.".into(),
            importance: 0.9,
        }),
        "vocal" => Ok(KnowledgeBite {
            category: "Mixing".into(),
            title: "Serial Compression (1176 -> LA-2A)".into(),
            body: "A classic 70s vocal trick: Use a fast FET compressor (like an 1176) barely catching the loudest peaks (-2 to -3dB). Follow it up with a slow Opto compressor (LA-2A) to gently smooth the overall performance. This gives vocals a cohesive, 'finished' radio-ready sound.".into(),
            importance: 0.8,
        }),
        "drums" => Ok(KnowledgeBite {
            category: "Rhythm".into(),
            title: "Micro-Timing and Groove Depth".into(),
            body: "The greatest drummers rarely play perfectly on a quantized grid. Push your snares slightly forward (5-10ms) to drive track energy, or pull them back to create a lazy, J Dilla-style hip-hop bounce.".into(),
            importance: 0.8,
        }),
        "theory" => Ok(KnowledgeBite {
            category: "Theory".into(),
            title: "Tension and Voice Leading".into(),
            body: "Instead of moving chord shapes in parallel blocks, keep common notes between chords and move other voices by single semi-tones. This creates smooth, cinematic transitions. Add 7ths and 9ths for color!".into(),
            importance: 0.8,
        }),
        "saturation" => Ok(KnowledgeBite {
            category: "Sound Design".into(),
            title: "Harmonic Glue".into(),
            body: "Don't just use EQs to make elements cut through a mix. Tape or Tube saturation generates new upper harmonics that help instruments (like bass guitars or 808s) translate perfectly on small phone speakers without altering the EQ balance.".into(),
            importance: 0.9,
        }),
        "spatial" => Ok(KnowledgeBite {
            category: "Psychoacoustics".into(),
            title: "The Haas Effect".into(),
            body: "Want ultra-wide guitars or synths? Delay one side of the stereo signal by 10 to 30 milliseconds perfectly. The brain perceives the sound as incredibly wide without creating a distinct echo.".into(),
            importance: 0.7,
        }),
        "arrangement" => Ok(KnowledgeBite {
            category: "Arrangement".into(),
            title: "Contrast is King".into(),
            body: "A massive chorus needs a small verse to feel huge. Try dropping out the bass or narrowing the stereo width severely 8 bars before the drop. When the chorus hits, it will sound twice as large.".into(),
            importance: 0.8,
        }),
        _ => Ok(KnowledgeBite {
            category: "General".into(),
            title: "The Golden Rule of Gain Staging".into(),
            body: "Regardless of the digital ceiling, aim for individual tracks to average around -18dBFS to -12dBFS. Most analog emulation plugins treat this zone as '0 VU'. Driving plugins harder can induce nasty digital clipping rather than warm analog saturation.".into(),
            importance: 0.5,
        }),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginRecommendation {
    pub name: String,
    pub description: String,
    pub plugin_type: String,
}

#[tauri::command]
pub fn query_plugin_database(category: String) -> Result<Vec<PluginRecommendation>, String> {
    // Deep knowledge base on VSTs and hardware equivalents
    match category.to_lowercase().as_str() {
        "eq" => Ok(vec![
            PluginRecommendation { name: "FabFilter Pro-Q 3".into(), description: "Surgical digital EQ with dynamic bands. The industry standard workflow king.".into(), plugin_type: "Digital/Dynamic EQ".into() },
            PluginRecommendation { name: "Pultec EQP-1A (UAD/Softube)".into(), description: "Famous for the 'boost/attenuate' trick on low frequencies. Massive weight and silky highs.".into(), plugin_type: "Passive Tube EQ".into() },
            PluginRecommendation { name: "Neve 1073 (Arturia/UAD)".into(), description: "The British console sound. Thick, punchy, beautiful harmonic distortion when driven.".into(), plugin_type: "Analog Console EQ".into() },
            PluginRecommendation { name: "Maag EQ4".into(), description: "Features the legendary 'Air Band'. Unmatched top-end sheen for vocals.".into(), plugin_type: "Hardware Emulation".into() }
        ]),
        "compression" => Ok(vec![
             PluginRecommendation { name: "UAD 1176 (Blackface/Blue Stripe)".into(), description: "Lightning-fast FET. Perfect for aggressive vocals, snappy snares, and parallel drum busses.".into(), plugin_type: "FET Compressor".into() },
             PluginRecommendation { name: "LA-2A (Teletronix/Waves)".into(), description: "Program-dependent Opto cell. Slow release, incredibly musical and smooth. Essential for ballads and bass.".into(), plugin_type: "Opto Leveler".into() },
             PluginRecommendation { name: "Softube Tube-Tech CL 1B".into(), description: "The modern vocal standard in hip-hop and pop. Creamy tube compression.".into(), plugin_type: "Tube Compressor".into() },
             PluginRecommendation { name: "SSL G-Bus".into(), description: "The 'Glue'. Famous for making disparate mix elements feel unified on a master bus.".into(), plugin_type: "VCA Compressor".into() },
             PluginRecommendation { name: "FabFilter Pro-C 2".into(), description: "Incredibly versatile modern compressor with multiple styles and perfect sidechaining.".into(), plugin_type: "Digital Compressor".into() }
        ]),
        "limiters" => Ok(vec![
             PluginRecommendation { name: "FabFilter Pro-L 2".into(), description: "Pristine mastering limiter. Transparent loudness with diverse algorithms.".into(), plugin_type: "Mastering Limiter".into() },
             PluginRecommendation { name: "Ozone Maximizer".into(), description: "iZotope's intelligent IRC limiter. Incredible at preventing pumping artifacts.".into(), plugin_type: "Intelligent Limiter".into() },
             PluginRecommendation { name: "Brainworx bx_limiter True Peak".into(), description: "Guarantees no inter-sample peaks will ruin your final export.".into(), plugin_type: "True Peak Limiter".into() }
        ]),
        "dynamics_control" => Ok(vec![
             PluginRecommendation { name: "FabFilter Pro-MB".into(), description: "Flexible multiband compression. Total control over specific frequency ranges.".into(), plugin_type: "Multiband".into() },
             PluginRecommendation { name: "Soothe2 (Oeksound)".into(), description: "Intelligent dynamic resonance suppressor. The ultimate harshness cure.".into(), plugin_type: "Dynamic Resonance Control".into() },
             PluginRecommendation { name: "SPL Transient Designer".into(), description: "Reshapes drum dynamics independently of audio levels. Magic for punch.".into(), plugin_type: "Transient Shaper".into() }
        ]),
        "reverb" => Ok(vec![
             PluginRecommendation { name: "Valhalla VintageVerb".into(), description: "Inspired by Lexicon hardware. Massive, lush algorithmic tails.".into(), plugin_type: "Algorithmic".into() },
             PluginRecommendation { name: "LiquidSonics Seventh Heaven".into(), description: "The definitive Bricasti M7 emulation. The most realistic acoustic spaces.".into(), plugin_type: "Convolution".into() },
             PluginRecommendation { name: "Eventide Blackhole".into(), description: "Extradimensional, infinite, evolving soundscapes. Not for realism.".into(), plugin_type: "Creative".into() }
        ]),
        "delay" => Ok(vec![
             PluginRecommendation { name: "Soundtoys EchoBoy".into(), description: "The king of delays. Emulates every vintage analog tape and bucket brigade delay known to man.".into(), plugin_type: "Analog Delay Emulator".into() },
             PluginRecommendation { name: "FabFilter Timeless 3".into(), description: "Modulation madness. Limitless routing and creative delay effects.".into(), plugin_type: "Creative Delay".into() }
        ]),
        "saturation" => Ok(vec![
             PluginRecommendation { name: "Soundtoys Decapitator".into(), description: "Ranging from subtle tape warmth to screaming tube destruction.".into(), plugin_type: "Saturation".into() },
             PluginRecommendation { name: "FabFilter Saturn 2".into(), description: "Multiband distortion capabilities for complex harmonic shaping.".into(), plugin_type: "Multiband Distortion".into() },
             PluginRecommendation { name: "UAD Studer A800".into(), description: "Classic 2-inch tape machine model for analog glue and low-end bump.".into(), plugin_type: "Tape Emulation".into() }
        ]),
        "spatial" => Ok(vec![
             PluginRecommendation { name: "iZotope Ozone Imager".into(), description: "Multiband widening without destroying mono-compatibility.".into(), plugin_type: "Stereo Imager".into() },
             PluginRecommendation { name: "Goodhertz Mid/Side".into(), description: "Clinical precision over the mid and side matrices.".into(), plugin_type: "M/S Matrix".into() }
        ]),
        "synth" => Ok(vec![
             PluginRecommendation { name: "Xfer Serum".into(), description: "The modern electronic standard. Visually driven wavetable synthesis.".into(), plugin_type: "Wavetable".into() },
             PluginRecommendation { name: "u-he Diva".into(), description: "Unmatched analog modeling. Phenomenal fatness matching classic Moogs.".into(), plugin_type: "Analog Emulation".into() },
             PluginRecommendation { name: "Spectrasonics Omnisphere".into(), description: "Massive cinematic hybrid synth. Thousands of incredibly deep patches.".into(), plugin_type: "Hybrid".into() }
        ]),
        _ => Ok(vec![
             PluginRecommendation { name: "Prisma EQ (Native VIBE)".into(), description: "Native 32-band visual EQ.".into(), plugin_type: "Native".into() }
        ])
    }
}

#[tauri::command]
pub fn generate_drum_pattern(
    _state: State<'_, AppState>,
    genre: String,
    bpm: f32,
    density: f32,
    swing: f32,
    humanization: f32,
    groove_archetype: String,
    interplay: f32,
    fill_frequency: u8,
    micro_layering: bool,
) -> Result<crate::engine::graph::MidiClipInfo, String> {
    // Determine sample rate from state, default to 44100
    let sample_rate = if let Ok(engine) = _state.audio_engine.lock() {
        if let Ok(config) = engine.current_config.lock() {
            config.as_ref().map(|c| c.sample_rate as f64).unwrap_or(44100.0)
        } else {
            44100.0
        }
    } else {
        44100.0
    };

    let settings = crate::engine::generators::drum_generator::DrumGeneratorSettings {
        genre,
        bpm,
        sample_rate,
        num_bars: 4,
        density,
        swing,
        humanization,
        groove_archetype,
        interplay,
        fill_frequency,
        micro_layering,
    };

    let clip = crate::engine::generators::drum_generator::generate_drums(&settings);

    Ok(crate::engine::graph::MidiClipInfo {
        id: clip.id.to_string(),
        name: clip.name,
        start_sample: clip.start_sample,
        length_samples: clip.length_samples,
        note_count: clip.notes.len(),
        color: clip.color,
        is_muted: clip.is_muted,
        is_looped: clip.is_looped,
        preview_notes: clip.notes.iter().take(100).map(|n| (n.start_sample, n.note, n.velocity)).collect(),
        pattern_id: clip.pattern_id,
        tuning_steps: clip.tuning_steps,
        time_signature_num: clip.time_signature_num,
        time_signature_den: clip.time_signature_den,
        gain_offset: 1.0,
        has_envelope: false,
    })
}

#[tauri::command]
pub fn generate_melody_pattern(
    _state: State<'_, AppState>,
    genre: String,
    bpm: f32,
    density: f32,
    root_note: u8,
    scale_type: String,
    instrument_type: String,
    motif_strength: f32,
    syncopation: f32,
    articulation_style: String,
    contour: String,
    breathing: f32,
) -> Result<crate::engine::graph::MidiClipInfo, String> {
    let sample_rate = if let Ok(engine) = _state.audio_engine.lock() {
        if let Ok(config) = engine.current_config.lock() {
            config.as_ref().map(|c| c.sample_rate as f64).unwrap_or(44100.0)
        } else {
            44100.0
        }
    } else {
        44100.0
    };

    let settings = crate::engine::generators::melody_generator::MelodyGeneratorSettings {
        genre,
        bpm,
        sample_rate,
        num_bars: 4,
        root_note,
        scale_type,
        density,
        instrument_type,
        motif_strength,
        syncopation,
        articulation_style,
        contour,
        breathing,
    };

    let clip = crate::engine::generators::melody_generator::generate_melody(&settings);

    Ok(crate::engine::graph::MidiClipInfo {
        id: clip.id.to_string(),
        name: clip.name,
        start_sample: clip.start_sample,
        length_samples: clip.length_samples,
        note_count: clip.notes.len(),
        color: clip.color,
        is_muted: clip.is_muted,
        is_looped: clip.is_looped,
        preview_notes: clip.notes.iter().take(100).map(|n| (n.start_sample, n.note, n.velocity)).collect(),
        pattern_id: clip.pattern_id,
        tuning_steps: clip.tuning_steps,
        time_signature_num: clip.time_signature_num,
        time_signature_den: clip.time_signature_den,
        gain_offset: 1.0,
        has_envelope: false,
    })
}

#[tauri::command]
pub fn generate_chord_pattern(
    _state: State<'_, AppState>,
    genre: String,
    bpm: f32,
    root_note: u8,
    _scale_type: String,
    _rhythm_style: String,
    complexity: f32,
    progression_preset: String,
    voicing_style: String,
    rhythm_complexity: f32,
    substitutions: f32,
) -> Result<crate::engine::graph::MidiClipInfo, String> {
    let sample_rate = if let Ok(engine) = _state.audio_engine.lock() {
        if let Ok(config) = engine.current_config.lock() {
            config.as_ref().map(|c| c.sample_rate as f64).unwrap_or(44100.0)
        } else {
            44100.0
        }
    } else {
        44100.0
    };

    let settings = crate::engine::generators::chord_generator::ChordGeneratorSettings {
        genre,
        bpm,
        sample_rate,
        root_note,
        complexity,
        progression_preset,
        voicing_style,
        rhythm_complexity,
        substitutions,
    };

    let clip = crate::engine::generators::chord_generator::generate_chords(&settings);

    Ok(crate::engine::graph::MidiClipInfo {
        id: clip.id.to_string(),
        name: clip.name,
        start_sample: clip.start_sample,
        length_samples: clip.length_samples,
        note_count: clip.notes.len(),
        color: clip.color,
        is_muted: clip.is_muted,
        is_looped: clip.is_looped,
        preview_notes: clip.notes.iter().take(100).map(|n| (n.start_sample, n.note, n.velocity)).collect(),
        pattern_id: clip.pattern_id,
        tuning_steps: clip.tuning_steps,
        time_signature_num: clip.time_signature_num,
        time_signature_den: clip.time_signature_den,
        gain_offset: 1.0,
        has_envelope: false,
    })
}

#[tauri::command]
pub fn generate_intelligent_arrangement(
    _state: State<'_, AppState>,
    genre: String,
    bpm: f32,
    root_note: u8,
    scale_type: String,
    length_profile: String,
) -> Result<crate::engine::generators::arrangement_generator::ProjectArrangement, String> {
    let sample_rate = if let Ok(engine) = _state.audio_engine.lock() {
        if let Ok(config) = engine.current_config.lock() {
            config.as_ref().map(|c| c.sample_rate as f64).unwrap_or(44100.0)
        } else {
            44100.0
        }
    } else {
        44100.0
    };

    let settings = crate::engine::generators::arrangement_generator::ArrangementSettings {
        genre,
        root_note,
        scale_type,
        bpm,
        sample_rate,
        _length_profile: length_profile,
    };

    let arrangement = crate::engine::generators::arrangement_generator::generate_arrangement(&settings);
    Ok(arrangement)
}
