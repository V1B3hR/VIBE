#![allow(dead_code)]
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Data Structures for Knowledge Base ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scale {
    pub name: String,
    pub intervals: Vec<i32>,
    pub vibe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChordProgression {
    pub name: String,
    pub degrees: Vec<String>,
    pub vibe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreProfile {
    pub name: String,
    pub bpm_range: (u32, u32),
    pub key_tendencies: Vec<String>,
    pub instrumentation: Vec<String>,
    pub spectral_targets: HashMap<String, String>,
    pub mixing_tips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixingRule {
    pub category: String,
    pub problem: String,
    pub symptoms: Vec<String>,
    pub solution: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoryDatabase {
    pub scales: Vec<Scale>,
    pub chord_progressions: Vec<ChordProgression>,
    pub modulation_tips: Vec<serde_json::Value>,
}

// --- Kropelka State & Persona ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KropelkaState {
    CreativeSpark,     // Writer's block / Starting
    ProducerMode,      // Collaborative suggestions (NEW)
    FlowState,         // Rapid progress
    TechnicalGuardian, // Creating mixing problems
    VibeCheck,         // Mastering / Finalizing
    Custom(String),
}

// --- Scene Analysis & Suggestions (Phase 4) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDensity {
    pub time_beats: f64,
    pub energy_db: f32,         // RMS / peak in window ~250 ms
    pub transient_count: u32,   // detected transients
    pub midi_note_density: f32, // notes per second
    pub spectral_centroid: f32, // brightness
    pub is_drop_zone: bool,     // heuristic drop detection
    pub is_polishing: bool,     // NEW: Intent Awareness
    pub is_experimenting: bool, // NEW: Intent Awareness
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: uuid::Uuid,
    pub description: String,
    pub confidence: f32,   // 0.0-1.0
    pub impact_score: f32, // estimated mix impact
    pub category: String,
    pub auto_apply: bool,
    pub action_type: String,
    pub action_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KropelkaPersona {
    pub name: String,
    pub tone: String, // "Chill", "Hype", "Strict"
    pub slang_enabled: bool,
}

impl Default for KropelkaPersona {
    fn default() -> Self {
        Self {
            name: "Kropelka".to_string(),
            tone: "Chill".to_string(),
            slang_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KropelkaInsight {
    pub text: String,
    pub category: String, // "Theory", "Mixing", "Safety", "Vibe"
    pub state: KropelkaState,
    pub action_type: Option<String>,
    pub action_data: Option<serde_json::Value>,
    // New optional field for Producer Mode choices
    pub choices: Option<Vec<String>>,
    pub emotion: Option<String>, // NEW: Emotional content from Brain/Forest
}

// --- The Brain ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub scale_usage: HashMap<String, u32>,
    pub genre_usage: HashMap<String, u32>,
    pub accepted_suggestions: u32,
    pub rejected_suggestions: u32,
    pub rejection_history: Vec<RejectionRecord>, // Detailed rejection tracking
    pub category_stats: HashMap<String, (u32, u32)>, // Category -> (accepted, rejected)
    pub recent_frustration_level: f32, // 0.0 to 1.0, decreases on accept, increases on reject
    pub tone_override: Option<String>,
    pub language: String, // e.g. "en", "pl", "it"
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            scale_usage: HashMap::new(),
            genre_usage: HashMap::new(),
            accepted_suggestions: 0,
            rejected_suggestions: 0,
            rejection_history: Vec::new(),
            category_stats: HashMap::new(),
            recent_frustration_level: 0.0,
            tone_override: None,
            language: "en".into()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectionRecord {
    pub timestamp: u128,
    pub action_type: String,
    pub context: String,
    pub persona_tone: String,
}

// Phase 7: Long-Horizon Project Awareness
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectEvolution {
    pub initial_track_count: usize,
    pub highest_cpu_load: f32,
    pub average_lufs_history: Vec<f32>,
    pub session_start_time: u128,
}

pub struct KropelkaBrain {
    pub persona: KropelkaPersona,
    pub current_state: KropelkaState,
    pub knowledge_base: KnowledgeBase,
    pub last_interaction: std::time::Instant,
    pub last_insight_time: std::time::Instant,
    // Phase 4: Context Memory
    pub current_scene: Option<SceneDensity>,
    pub suggestion_history: Vec<uuid::Uuid>,
    // Phase 4.1: Learning
    pub user_prefs: UserPreferences,
    // Phase 7: Long-term
    pub project_evolution: ProjectEvolution,
    // Phase 4: UI Insight
    pub ui_context: Option<crate::engine::kropelka::KropelkaContext>,
    // Integration: NeuralForest Bridge
    pub forest_bridge:
        Option<std::sync::Arc<tokio::sync::Mutex<super::neural_forest::NeuralForestBridge>>>,
    
    // Wellbeing & Homeostasis (Phase 8: Debugging Pro)
    pub last_action_type: Option<String>,
    pub interaction_cooldown: HashMap<String, std::time::Instant>,
}

pub struct KnowledgeBase {
    pub theory: TheoryDatabase,
    pub genres: Vec<GenreProfile>,
    pub mixing_rules: Vec<MixingRule>,
    pub locales: serde_json::Value,
}

lazy_static! {
    static ref KNOWLEDGE_BASE_PATH: String = format!("{}/assets/brain", env!("CARGO_MANIFEST_DIR"));
}

impl KropelkaBrain {
    pub fn new() -> Self {
        let kb = KnowledgeBase {
            theory: TheoryDatabase {
                scales: vec![],
                chord_progressions: vec![],
                modulation_tips: vec![],
            },
            genres: vec![],
            mixing_rules: vec![],
            locales: serde_json::Value::Null,
        };

        Self {
            persona: KropelkaPersona::default(),
            current_state: KropelkaState::CreativeSpark,
            knowledge_base: kb,
            last_interaction: std::time::Instant::now(),
            last_insight_time: std::time::Instant::now().checked_sub(std::time::Duration::from_secs(60)).unwrap(),
            current_scene: None,
            suggestion_history: Vec::new(),
            user_prefs: UserPreferences::default(),
            project_evolution: ProjectEvolution {
                session_start_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(),
                ..Default::default()
            },
            ui_context: None,
            forest_bridge: None,
            last_action_type: None,
            interaction_cooldown: HashMap::new(),
        }
    }

    pub fn set_context(&mut self, context: crate::engine::kropelka::KropelkaContext) {
        self.ui_context = Some(context);
    }

    pub fn attach_brain(
        &mut self,
        bridge: std::sync::Arc<tokio::sync::Mutex<super::neural_forest::NeuralForestBridge>>,
    ) {
        self.forest_bridge = Some(bridge);
    }

    pub fn get_category_rejection_rate(&self, category: &str) -> f32 {
        if let Some((accepted, rejected)) = self.user_prefs.category_stats.get(category) {
            let total = accepted + rejected;
            if total > 3 {
                return *rejected as f32 / total as f32;
            }
        }
        0.0 // Default to 0% rejection if not enough data
    }

    /// Records user interaction to adapt future suggestions
    pub fn learn_interaction(&mut self, action_type: &str, accepted: bool) {
        let mapped_category = match action_type {
            "ApplySmartEQ" | "SidechainSuggestion" | "BalanceTracks" => "Mixing",
            "ApplyLimiter" => "Mastering",
            "SetProjectScale" | "InsertMidiProgression" | "ModulateProject" | "ApplyNegativeHarmony" | "ApplyGenreTemplate" => "Theory",
            "ApplyGroove" => "Groove",
            _ => "General"
        };
        
        let stats = self.user_prefs.category_stats.entry(mapped_category.to_string()).or_insert((0, 0));

        if accepted {
            self.user_prefs.accepted_suggestions += 1;
            stats.0 += 1;
            // Decrease frustration slightly when things go well
            self.user_prefs.recent_frustration_level = (self.user_prefs.recent_frustration_level - 0.15).max(0.0);
        } else {
            self.user_prefs.rejected_suggestions += 1;
            stats.1 += 1;
            // Increase frustration when suggestions are rejected
            self.user_prefs.recent_frustration_level = (self.user_prefs.recent_frustration_level + 0.1).min(1.0);

            // Record detailed rejection for future training/diagnostics
            let record = RejectionRecord {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
                action_type: action_type.to_string(),
                context: format!("{:?} - Frustration: {:.2}", self.current_state, self.user_prefs.recent_frustration_level),
                persona_tone: self.persona.tone.clone(),
            };
            self.user_prefs.rejection_history.push(record);
        }

        self.adapt_persona_from_prefs();
        self.last_action_type = Some(action_type.to_string());
        self.interaction_cooldown.insert(action_type.to_string(), std::time::Instant::now());

        // Integration: Pipe feedback to NeuralForest bridge
        if let Some(bridge_mutex) = &self.forest_bridge {
            let action = action_type.to_string();
            let bridge = bridge_mutex.clone();
            tokio::spawn(async move {
                let mut guard = bridge.lock().await;
                let _ = guard
                    .send_command(
                        "record_feedback",
                        Some(serde_json::json!({
                            "action": action,
                            "accepted": accepted
                        })),
                    )
                    .await;
            });
        }

        self.save_memory();
    }

    pub(crate) fn adapt_persona_from_prefs(&mut self) {
        // High frustration -> Gentle / Supportive voice
        if self.user_prefs.recent_frustration_level > 0.6 {
            self.persona.tone = "Supportive".to_string();
            return;
        }

        let total = self.user_prefs.accepted_suggestions + self.user_prefs.rejected_suggestions;
        if total > 5 {
            let rejection_rate = self.user_prefs.rejected_suggestions as f32 / total as f32;
            if rejection_rate > 0.6 {
                self.persona.tone = "Assertive".to_string(); // Stanowcza, do rzeczy
            } else if rejection_rate < 0.2 {
                self.persona.tone = "Supportive".to_string(); // Przyjacielska, mentor
            } else {
                self.persona.tone = "Professional".to_string(); // Wyważona, rzeczowa
            }
        } else {
            self.persona.tone = "Professional".to_string();
        }

        if let Some(override_tone) = &self.user_prefs.tone_override {
            self.persona.tone = override_tone.clone();
        }
    }

    pub fn t(&self, key: &str, args: Option<&[&str]>) -> String {
        let lang = &self.user_prefs.language;
        let mut text = self.knowledge_base.locales[lang]["suggestions"][key]
            .as_str()
            .unwrap_or_else(|| {
                self.knowledge_base.locales["en"]["suggestions"][key]
                    .as_str()
                    .unwrap_or(key)
            })
            .to_string();

        if let Some(args) = args {
            for arg in args {
                text = text.replacen("{}", arg, 1);
            }
        }
        text
    }

    pub fn localized_choice(&self, key: &str) -> String {
        let lang = &self.user_prefs.language;
        self.knowledge_base.locales[lang]["choices"][key]
            .as_str()
            .unwrap_or_else(|| {
                self.knowledge_base.locales["en"]["choices"][key]
                    .as_str()
                    .unwrap_or(key)
            })
            .to_string()
    }

    pub fn localized_category(&self, key: &str) -> String {
        let lang = &self.user_prefs.language;
        let key_lower = key.to_lowercase();
        self.knowledge_base.locales[lang]["categories"][&key_lower]
            .as_str()
            .unwrap_or_else(|| {
                self.knowledge_base.locales["en"]["categories"][&key_lower]
                    .as_str()
                    .unwrap_or(key)
            })
            .to_string()
    }

    pub fn load_knowledge_base(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let theory_path = format!("{}/core/theory.json", *KNOWLEDGE_BASE_PATH);
        if let Ok(content) = std::fs::read_to_string(&theory_path) {
            self.knowledge_base.theory = serde_json::from_str(&content)?;
        }
        let genre_path = format!("{}/core/genre_profiles.json", *KNOWLEDGE_BASE_PATH);
        if let Ok(content) = std::fs::read_to_string(&genre_path) {
            self.knowledge_base.genres = serde_json::from_str(&content)?;
        }
        let rules_path = format!("{}/core/mixing_rules.json", *KNOWLEDGE_BASE_PATH);
        if let Ok(content) = std::fs::read_to_string(&rules_path) {
            self.knowledge_base.mixing_rules = serde_json::from_str(&content)?;
        }
        let locales_path = format!("{}/core/locales.json", *KNOWLEDGE_BASE_PATH);
        if let Ok(content) = std::fs::read_to_string(&locales_path) {
            self.knowledge_base.locales = serde_json::from_str(&content)?;
        }
        let user_genres_path = format!("{}/user/custom_genres.json", *KNOWLEDGE_BASE_PATH);
        if let Ok(content) = std::fs::read_to_string(&user_genres_path) {
            if let Ok(user_genres) = serde_json::from_str::<Vec<GenreProfile>>(&content) {
                self.knowledge_base.genres.extend(user_genres);
            }
        }

        // Load Memory
        let memory_path = format!("{}/user/user_memory.json", *KNOWLEDGE_BASE_PATH);
        if let Ok(content) = std::fs::read_to_string(&memory_path) {
            if let Ok(prefs) = serde_json::from_str::<UserPreferences>(&content) {
                self.user_prefs = prefs;
            }
        }

        Ok(())
    }

    /// Wellbeing: Prunes old memories to keep the brain fresh (forgiveness)
    pub fn prune_old_memory(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        // Remove rejections older than 7 days (simulated)
        let seven_days_ms = 7 * 24 * 60 * 60 * 1000;
        self.user_prefs.rejection_history.retain(|r| (now - r.timestamp) < seven_days_ms);
        
        // Decay frustration level naturally
        self.user_prefs.recent_frustration_level *= 0.95;
    }

    pub fn save_memory(&self) {
        let memory_path = format!("{}/user/user_memory.json", *KNOWLEDGE_BASE_PATH);
        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(&memory_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.user_prefs) {
            let _ = std::fs::write(memory_path, content);
        }
    }

    pub fn detect_scale(&self, notes: &[u16]) -> Option<(String, String)> {
        if notes.is_empty() {
            return None;
        }
        let mut pitch_classes = HashMap::new();
        for &n in notes {
            let pc = (n % 12) as i32;
            *pitch_classes.entry(pc).or_insert(0) += 1;
        }
        let unique_pcs: Vec<i32> = pitch_classes.keys().cloned().collect();
        let mut best_scale = None;
        let mut best_score = -1;
        for scale in &self.knowledge_base.theory.scales {
            for root in 0..12 {
                let mut score = 0;
                let scale_notes: Vec<i32> =
                    scale.intervals.iter().map(|&i| (root + i) % 12).collect();
                for &pc in &unique_pcs {
                    if scale_notes.contains(&pc) {
                        score += 1;
                    }
                }
                if score > best_score {
                    best_score = score;
                    best_scale = Some((
                        format!(
                            "{} {}",
                            ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
                                [root as usize],
                            scale.name
                        ),
                        scale.vibe.clone(),
                    ));
                }
            }
        }
        best_scale
    }

    // New helper to get safe reference to bridge from parent Kropelka struct
    pub fn decide_reaction(
        &mut self,
        mix_analysis: &crate::engine::kropelka::MixAnalysis,
        project_context: &str,
        tracks: &[crate::engine::graph::Track],
        track_levels: &[crate::engine::graph::TrackLevel],
        playhead: u64,
        sample_rate: f64,
        cpu_load: f32, // TOTAL System CPU Load
        bpm: f64,
        bridge: Option<
            std::sync::Arc<tokio::sync::Mutex<crate::engine::neural_forest::NeuralForestBridge>>,
        >,
    ) -> Option<KropelkaInsight> {
        // --- ZOSIA-SAMOSIA (Groove Genetix / Melody Control) ---
        // Catch user prompt strings that command an AI clip generation
        let ctx_lower = project_context.to_lowercase();
        if ctx_lower.contains("dodaj") || ctx_lower.contains("stworz") || ctx_lower.contains("stwórz") || ctx_lower.contains("wygeneruj") {
            if ctx_lower.contains("groove") || ctx_lower.contains("bębn") || ctx_lower.contains("perkusj") || ctx_lower.contains("techno") {
                let add_fill = ctx_lower.contains("przejści") || ctx_lower.contains("fill");
                
                let insight = KropelkaInsight {
                    text: if add_fill {
                        "Zosia-Samosia zrozumiała! Generuję potężny groove z przejściem (fill'em) na końcu pętli, wrzucam Ci prosto układ na oś czasu!".to_string()
                    } else {
                        "Proszę bardzo! Groove jest wygenerowany i osadzony na pierwszej ścieżce.".to_string()
                    },
                    category: self.localized_category("Groove"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("GenerateDrumClip".to_string()),
                    action_data: Some(serde_json::json!({
                        "style": if ctx_lower.contains("techno") { "techno" } else { "rock" },
                        "add_fill_at_end": add_fill
                    })),
                    choices: None,
                    emotion: Some("confident".to_string()),
                };
                self.last_insight_time = std::time::Instant::now();
                return Some(insight);
            }
        }

        // Phase 4: Analyze Scene context locally (still useful for fast updates)
        let scene = self.evaluate_scene(mix_analysis, tracks, playhead, sample_rate);
        self.current_scene = Some(scene.clone());

        let is_emergency = mix_analysis.peak_level > 1.99 || mix_analysis.clipping_detected;

        // Flow State / Non-intrusiveness Check
        if scene.midi_note_density > 15.0 || scene.transient_count > 60 || scene.is_polishing {
            // Senses that the producer is in a creative frenzy or deep focus. Be quiet unless emergency.
            if !is_emergency {
                return None;
            }
        }

        // Plugin Context Awareness (Micro-contextual advice)
        if let Some(crate::engine::kropelka::KropelkaContext::Plugin(plugin_info)) = &self.ui_context {
            let time_since_last = self.last_insight_time.elapsed().as_secs();
            // Allow parameter insights to be more frequent when actively tweaking, but still rate limit
            // Intent Awareness: If experimenting, suggest sweeping or micro-learning.
            if time_since_last > 4 {
                if let Some(mut insight) = self.generate_plugin_insight(plugin_info, mix_analysis) {
                    if scene.is_experimenting {
                         insight.text = format!("Experimenting? {}", insight.text);
                         insight.emotion = Some("thoughtful".to_string());
                    }
                    self.last_insight_time = std::time::Instant::now();
                    self.inject_persona(&mut insight);
                    return Some(insight);
                }
            }
        }

        let time_since_last = self.last_insight_time.elapsed().as_secs();
        if !is_emergency && time_since_last < 25 {
            return None; // Nie bądź nachalna, max 1 sugestia na 25s
        }

        // We are generating an insight, mark the time (we might return None later, but we rate limit attempts too to avoid spam checking)
        

        // Analyze Long-Horizon Trends (Phase 7)
        // Store historical data
        self.project_evolution.average_lufs_history.push(mix_analysis.lufs_level);
        if self.project_evolution.average_lufs_history.len() > 100 {
            self.project_evolution.average_lufs_history.remove(0);
        }
        if cpu_load > self.project_evolution.highest_cpu_load {
             self.project_evolution.highest_cpu_load = cpu_load;
        }

        if let Some(mut insight) = self.analyze_long_horizon(tracks, cpu_load) {
             if rand::random::<f32>() > 0.8 {
                self.last_insight_time = std::time::Instant::now();
                self.inject_persona(&mut insight);
                return Some(insight);
             }
        }

        // LEVEL 4: CO-PILOT (Creative Partner) - Check for creative opportunities
        if let Some(mut insight) = self.analyze_creative_opportunities(tracks, bpm, sample_rate) {
            // 25% chance to show creative insight
            if rand::random::<f32>() > 0.75 {
                self.last_insight_time = std::time::Instant::now();
                self.inject_persona(&mut insight);
                return Some(insight);
            }
        }

        // IF BRIDGE IS CONNECTED -> ASK THE BRAIN
        if let Some(_) = bridge {
            return None; // Commands will handle the async query.
        }

        // Fallback to local heuristics if no brain (legacy mode)
        self.update_state(mix_analysis, project_context);

        // LEVEL 2: HOUSEKEEPER (GOSPOSIA) - Check for organization issues if in ProducerMode or FlowState
        if self.current_state == KropelkaState::ProducerMode
            || self.current_state == KropelkaState::FlowState
        {
            if let Some(mut insight) = self.analyze_organization(tracks) {
                // 30% chance to show organization insight to avoid being annoying
                if rand::random::<f32>() > 0.7 {
                    self.last_insight_time = std::time::Instant::now();
                    self.inject_persona(&mut insight);
                    return Some(insight);
                }
            }
        }

        // LEVEL 3: TECHNIK (System Guardian) - Check for technical issues
        if let Some(mut insight) = self.analyze_technical_health(tracks, sample_rate, cpu_load) {
            // 20% chance to show technical insight to avoid being annoying
            if rand::random::<f32>() > 0.8 {
                self.last_insight_time = std::time::Instant::now();
                self.inject_persona(&mut insight);
                return Some(insight);
            }
        }

        let mut final_insight = match self.current_state {
            KropelkaState::TechnicalGuardian => self.generate_technical_insight(mix_analysis),
            KropelkaState::ProducerMode => {
                if let Some(insight) = self.analyze_cross_track_masking(tracks, track_levels) {
                    Some(insight)
                } else if let Some(insight) = self.analyze_mix_balance(tracks, track_levels) {
                    Some(insight)
                } else if let Some(insight) = self.generate_smart_eq_insight(mix_analysis) {
                    Some(insight)
                } else {
                    self.generate_producer_insight(&scene)
                }
            }
            KropelkaState::CreativeSpark => {
                if let Some(insight) = self.generate_groove_insight(&scene) {
                    Some(insight)
                } else if rand::random::<f32>() > 0.3 {
                    self.generate_creative_insight()
                } else {
                    self.generate_genre_insight()
                }
            }
            KropelkaState::VibeCheck => self.generate_vibe_insight(mix_analysis),
            _ => None,
        };

        if let Some(ref mut insight) = final_insight {
            // Producer Behavior Modeling: Only suggest if they aren't rejecting this entirely
            // Note: We use localized category, but mapping back to internal english strings for stats lookup
            let mapped_category = match insight.category.as_str() {
                c if c == self.localized_category("Mixing") => "Mixing",
                c if c == self.localized_category("Mastering") => "Mastering",
                c if c == self.localized_category("Theory") => "Theory",
                c if c == self.localized_category("Groove") => "Groove",
                _ => "General"
            };

            let rejection_rate = self.get_category_rejection_rate(mapped_category);
            
            // If the user hates tips for this category (e.g., >80% rejected), we don't bother them unless it's a critical safety issue
            if rejection_rate > 0.8 && self.current_state != KropelkaState::TechnicalGuardian {
                 return None;
            }

            self.last_insight_time = std::time::Instant::now();
            self.inject_persona(insight);
        }
        final_insight
    }

    /// Predictive Arrangement: Suggests the next section based on current project density and structure.
    pub fn predictive_arrangement(&self, _tracks: &[crate::engine::graph::Track], bpm: f64) -> Option<KropelkaInsight> {
        let scene = self.current_scene.as_ref()?;
        
        // Simplified Logic:
        // Intro (Low Density) -> Suggest Verse
        // Verse (Medium Density) -> Suggest Build-up
        // Build-up (Rising Centroid/Density) -> Suggest Drop/Chorus
        
        if scene.midi_note_density < 3.0 && scene.energy_db < -18.0 {
            return Some(KropelkaInsight {
                text: "Czuję tutaj początek czegoś wielkiego. Może po tym Intrze wejdziemy w mocny, rytmiczny Verse?".to_string(),
                category: "Arrangement".to_string(),
                state: KropelkaState::CreativeSpark,
                action_type: Some("SuggestSection".to_string()),
                action_data: Some(serde_json::json!({"type": "Verse", "bpm": bpm})),
                choices: Some(vec!["Stwórz Verse".into(), "Nie, czekaj".into()]),
                emotion: Some("inspired".to_string()),
            });
        }
        
        if scene.is_drop_zone && scene.energy_db > -6.0 {
             return Some(KropelkaInsight {
                text: "Ale ogień! Po takim Dropie przydałby się Breakdown, żeby słuchacz mógł złapać oddech przed kolejnym uderzeniem.".to_string(),
                category: "Arrangement".to_string(),
                state: KropelkaState::ProducerMode,
                action_type: Some("SuggestSection".to_string()),
                action_data: Some(serde_json::json!({"type": "Breakdown"})),
                choices: Some(vec!["Zrób Breakdown".into(), "Jeszcze nie".into()]),
                emotion: Some("hype".to_string()),
            });
        }

        None
    }

    /// Voice Commands: Parses natural language strings for DAW control.
    pub fn parse_voice_command(&mut self, text: &str) -> Option<KropelkaInsight> {
        let input = text.to_lowercase();
        
        if !input.contains("kropelka") {
            return None;
        }

        if input.contains("kick") || input.contains("stopa") || input.contains("stopy") || input.contains("kicka") {
            if input.contains("moc") || input.contains("twardo") || input.contains("punch") || input.contains("dół") || input.contains("dol") {
                return Some(KropelkaInsight {
                    text: "Zrozumiałam! Dopieszczam dół i uderzenie stopy. Kick będzie teraz siedział idealnie w miksie.".to_string(),
                    category: "Voice Command".to_string(),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("TweakTrack".to_string()),
                    action_data: Some(serde_json::json!({"target": "Kick", "params": {"compression": 0.8, "attack": 0.15, "eq_low_boost": 2.5}})),
                    choices: None,
                    emotion: Some("confident".to_string()),
                });
            }
        }

        // 2. Arrangement Control
        if input.contains("następn") || input.contains("next") {
             if input.contains("sekcj") || input.contains("part") {
                 return self.predictive_arrangement(&[], 120.0); // Simple fallback
             }
        }

        // 3. Mixing / Utility
        if input.contains("wyczyść") || input.contains("clear") {
            if input.contains("mik") || input.contains("mix") {
                return Some(KropelkaInsight {
                    text: "Resetuję mikser do stanu początkowego. Zaczynamy od nowa?".to_string(),
                    category: "Voice Command".to_string(),
                    state: KropelkaState::TechnicalGuardian,
                    action_type: Some("ResetMixer".to_string()),
                    action_data: None,
                    choices: Some(vec!["Tak, resetuj".into(), "Nie!".into()]),
                    emotion: Some("serious".to_string()),
                });
            }
        }

        Some(KropelkaInsight {
            text: format!("Usłyszałam: \"{}\", ale nie jestem pewna jak to wykonać. Możesz powtórzyć inaczej?", text),
            category: "Voice Command".to_string(),
            state: KropelkaState::FlowState,
            action_type: None,
            action_data: None,
            choices: None,
            emotion: Some("confused".to_string()),
        })
    }

    fn inject_persona(&self, insight: &mut KropelkaInsight) {
        // Polish contextual persona mapping: positive, professional, supportive friend, but assertive when needed.
        let tone = self.persona.tone.as_str();
        let emergency = insight.state == KropelkaState::TechnicalGuardian;

        if emergency {
            if tone == "Assertive" {
                insight.text = format!("⚠️ Muszę interweniować. {}", insight.text);
                insight.emotion = Some("strict".to_string());
            } else if tone == "Supportive" {
                insight.text = format!("Spokojnie, to nic wielkiego, ale uważaj: {}", insight.text);
                insight.emotion = Some("concerned".to_string());
            } else {
                insight.text = format!("Wykryto problem techniczny. {}", insight.text);
                insight.emotion = Some("serious".to_string());
            }
        } else if tone == "Assertive" {
            insight.text = format!("Moja konkretna rada: {}", insight.text);
            insight.emotion = Some("confident".to_string());
        } else if tone == "Supportive" {
            insight.text = format!("Świetnie Ci idzie! 💡 Wpadłam na pomysł: {}", insight.text);
            insight.emotion = Some("happy".to_string());
        } else {
            insight.text = format!("Z mojej profesjonalnej perspektywy: {}", insight.text);
            insight.emotion = Some("friendly".to_string());
        }
    }

    // Phase 4: Scene Analysis
    pub fn evaluate_scene(
        &self,
        mix_analysis: &crate::engine::kropelka::MixAnalysis,
        tracks: &[crate::engine::graph::Track],
        playhead_pos: u64,
        sample_rate: f64,
    ) -> SceneDensity {
        // 1. Calculate MIDI Density in current window (2 seconds)
        let window_samples = (sample_rate * 2.0) as u64;
        let start = playhead_pos.saturating_sub(window_samples);
        let end = playhead_pos + window_samples;

        let mut note_count = 0;
        let mut transient_count = 0;
        let mut total_clips = 0;

        for track in tracks {
            total_clips += track.clips.len() + track.midi_clips.len();
            for clip in &track.midi_clips {
                if clip.start_sample < end && (clip.start_sample + clip.length_samples) > start {
                    note_count += clip.notes.len();
                }
            }
            for clip in &track.clips {
                if clip.start_sample < end && (clip.start_sample + clip.length_in_samples) > start {
                    transient_count += clip.transients.len().max(1);
                }
            }
        }

        let density = note_count as f32 / 4.0; // Notes per second (approx)
                                               // Heuristic: If density is high and RMS is high -> Drop Zone?
        let is_drop =
            density > 8.0 && (mix_analysis.rms_level > 0.6 || mix_analysis.transient_density > 0.7);

        // Phase 5: Intent Awareness (Simplistic)
        // If track count grew fast, or clip count is changing rapidly, might be 'experimenting'.
        // If everything is static and we are just looping, might be 'polishing'.
        let mut is_polishing = false;
        let mut is_experimenting = false;
        if tracks.len() > self.project_evolution.initial_track_count + 3 {
             is_experimenting = true;
        } else if total_clips > 50 && transient_count < 10 {
             is_polishing = true;
        }

        SceneDensity {
            time_beats: (playhead_pos as f64 / sample_rate) * (120.0 / 60.0), // Simplified BPM assumption
            energy_db: (mix_analysis.rms_level + 1e-6).log10() * 20.0,
            transient_count: transient_count as u32,
            midi_note_density: density,
            spectral_centroid: mix_analysis.spectral_centroid * 10000.0,
            is_drop_zone: is_drop,
            is_polishing,
            is_experimenting,
        }
    }

    fn generate_producer_insight(&self, scene: &SceneDensity) -> Option<KropelkaInsight> {
        // High Energy Drop logic
        if scene.is_drop_zone
            && scene.spectral_centroid < 2000.0 {
                return Some(KropelkaInsight {
                    text: self.t("dark_drop", None),
                    category: self.localized_category("Mixing"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("SuggestEffect".to_string()),
                    action_data: Some(serde_json::json!({"type": "HighShelf", "gain": 3.0})),
                    choices: Some(vec![
                        self.localized_choice("apply"),
                        self.localized_choice("explain"),
                        self.localized_choice("nah"),
                    ]),
                    emotion: None,
                });
            }

        // Low Energy but busy MIDI
        if scene.midi_note_density > 10.0 && scene.energy_db < -12.0 {
            return Some(KropelkaInsight {
                text: self.t("low_energy_midi", None),
                category: self.localized_category("Dynamics"),
                state: KropelkaState::ProducerMode,
                action_type: Some("SuggestPlugin".to_string()),
                action_data: Some(serde_json::json!({"plugin": "Compressor"})),
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        None
    }

    fn generate_plugin_insight(&self, plugin_info: &str, mix_analysis: &crate::engine::kropelka::MixAnalysis) -> Option<KropelkaInsight> {
        // Parse the focus context, which is typically "PluginName:ParameterName"
        let parts: Vec<&str> = plugin_info.split(':').collect();
        let plugin_name = parts[0];
        let param_name = if parts.len() > 1 { parts[1] } else { "" };

        if plugin_name == "NanoEQ" {
            // Depending on what band the user is hovering/tweaking, we offer micro-advice.
            if param_name == "Low Band" && mix_analysis.spectral_balance > 0.6 {
                return Some(KropelkaInsight {
                    text: "You are adjusting the Low Band, but the mix is already quite bottom-heavy. Try cutting rather than boosting.".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::ProducerMode,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("friendly".to_string()),
                });
            } else if param_name == "Mid Band" && mix_analysis.spectral_balance < 0.3 {
                return Some(KropelkaInsight {
                    text: "The mids are feeling a bit hollow right now. A gentle wide boost here might add some body to the mix.".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::ProducerMode,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("friendly".to_string()),
                });
            } else if param_name == "High Band" && mix_analysis.spectral_centroid > 5000.0 {
                return Some(KropelkaInsight {
                    text: "Be careful with the high shelf! The mix is already very bright, we want to avoid harshness.".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::TechnicalGuardian,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("concerned".to_string()),
                });
            } else if param_name == "High Band" && mix_analysis.spectral_centroid < 1000.0 {
                 return Some(KropelkaInsight {
                    text: "Adding some air and presence here could really open up the sound!".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::ProducerMode,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("happy".to_string()),
                });
            }
        } else if plugin_name == "Compressor" {
            if param_name == "Release" && mix_analysis.transient_density > 0.8 {
                return Some(KropelkaInsight {
                    text: "I notice a lot of fast transients here. If your release time is too long, the compressor won't reset in time and you'll lose punch. Try a faster release!".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::ProducerMode,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("friendly".to_string()),
                });
            }
        } else if plugin_name == "Reverb" {
            if param_name == "PreDelay" {
                return Some(KropelkaInsight {
                    text: "Tweaking the Pre-Delay? A good trick is to calculate it based on the BPM. Try dividing 60,000 by your BPM to get a musical delay time (like an 1/8th or 1/16th note).".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::ProducerMode,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("friendly".to_string()),
                });
            } else if param_name == "Decay" && mix_analysis.spectral_balance < 0.3 {
                 return Some(KropelkaInsight {
                    text: "The mix is already pretty dense and dark. A very long reverb decay might muddy things up. Consider shortening the decay or adding a high-pass filter to the reverb tail.".to_string(),
                    category: self.localized_category("Plugin"),
                    state: KropelkaState::TechnicalGuardian,
                    action_type: None,
                    action_data: None,
                    choices: None,
                    emotion: Some("concerned".to_string()),
                });
            }
        }
        
        None
    }

    fn generate_technical_insight(
        &self,
        analysis: &crate::engine::kropelka::MixAnalysis,
    ) -> Option<KropelkaInsight> {
        // 1. Check for Clipping (Highest Priority)
        // CRITICAL: Panic Guard (Auto-Engage)
        if analysis.peak_level > 1.99 {
            // Extremely high spike
            return Some(KropelkaInsight {
                text: self.t("panic_guard_active", None),
                category: self.localized_category("Safety"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("ApplyPanicGuard".to_string()),
                action_data: Some(serde_json::json!({"action": "MasterLimiterEngaged"})),
                choices: None, // Engaged automatically in critical state
                emotion: None,
            });
        }

        if analysis.clipping_detected {
            return Some(KropelkaInsight {
                text: self.t("clipping", None),
                category: self.localized_category("Safety"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("NormalizeMix".to_string()),
                action_data: Some(serde_json::json!({"target": -3.0})),
                choices: Some(vec![
                    self.localized_choice("normalize"),
                    self.localized_choice("add_limiter"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // 2. Dynamic Rule Matching from Knowledge Base
        for rule in &self.knowledge_base.mixing_rules {
            let mut symptom_match = false;

            // Heuristic symptom mapping
            if rule.problem == "Muddy Mix"
                && analysis.spectral_balance < 0.25
                && analysis.rms_level > 0.3
            {
                symptom_match = true;
            } else if rule.problem == "Harsh Highs"
                && analysis.spectral_balance > 0.8
                && analysis.spectral_centroid > 0.7
            {
                symptom_match = true;
            } else if rule.problem == "Low Dynamics"
                && analysis.peak_level - analysis.rms_level < 0.1
                && analysis.rms_level > 0.5
            {
                // Crest factor too low
                symptom_match = true;
            }

            if symptom_match {
                return Some(KropelkaInsight {
                    text: format!(
                        "Analysis complete: {} detected. {}. Recommend: {}",
                        rule.problem, rule.context, rule.solution
                    ),
                    category: rule.category.clone(),
                    state: KropelkaState::TechnicalGuardian,
                    action_type: Some("ApplyMixAdvice".to_string()),
                    action_data: Some(serde_json::json!({
                        "problem": rule.problem,
                        "solution": rule.solution
                    })),
                    choices: Some(vec![
                        self.localized_choice("apply"),
                        self.localized_choice("explain"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // 3. Fallback Heuristics
        if analysis.spectral_balance < 0.2 {
            return Some(KropelkaInsight {
                text: self.t("muddy_mix", None),
                category: self.localized_category("Mixing"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("EqFixMud".to_string()),
                action_data: Some(serde_json::json!({"freq": 300.0, "gain": -4.0})),
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        None
    }

    fn analyze_long_horizon(
        &self,
        tracks: &[crate::engine::graph::Track],
        cpu_load: f32,
    ) -> Option<KropelkaInsight> {
        // CPU Trend
        if cpu_load > 0.8 && self.project_evolution.highest_cpu_load > 0.85 {
            return Some(KropelkaInsight {
                text: "Your project is getting quite heavy on the CPU over time. Consider freezing tracks with heavy synthesizers like Wavetables or Granulars.".to_string(),
                category: self.localized_category("Safety"),
                state: KropelkaState::TechnicalGuardian,
                action_type: None,
                action_data: None,
                choices: None,
                emotion: Some("concerned".to_string()),
            });
        }

        // Track count growth
        if tracks.len() > self.project_evolution.initial_track_count + 15 {
             return Some(KropelkaInsight {
                text: "You've added a lot of layers recently! The arrangement is getting very dense. Might be a good time to check your mid-range for masking or try some subtractive orchestration.".to_string(),
                category: self.localized_category("Mixing"),
                state: KropelkaState::ProducerMode,
                action_type: None,
                action_data: None,
                choices: None,
                emotion: Some("thoughtful".to_string()),
            });
        }

        None
    }

    fn generate_creative_insight(&self) -> Option<KropelkaInsight> {
        let mut rng = rand::thread_rng();

        // 1. Pick a random operation: Suggest Chords (60%), Suggest Modulation (25%), Suggest Theory (15%)
        let choice = rand::random::<f32>();

        if choice < 0.6 {
            // Suggest Chords
            if let Some(prog) = self
                .knowledge_base
                .theory
                .chord_progressions
                .choose(&mut rng)
            {
                // Determine Key (Mock C Major for now, ideally get from project)
                let key = crate::engine::theory::Key {
                    root: "C".to_string(),
                    scale_type: "Major".to_string(),
                };

                // Convert knowledge base progression to template
                let template = crate::engine::theory::generator::ProgressionTemplate {
                    name: prog.name.clone(),
                    degrees: prog.degrees.clone(),
                    vibe: prog.vibe.clone(),
                };

                let chords =
                    crate::engine::theory::Generator::generate_progression(&key, &template);
                let chord_names: Vec<String> = chords
                    .iter()
                    .map(|c| {
                        format!(
                            "{}{}",
                            ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
                                [c.root as usize],
                            c.quality
                        )
                    })
                    .collect();

                return Some(KropelkaInsight {
                    text: format!(
                        "Stuck? Try this '{}' progression: {}",
                        prog.name,
                        chord_names.join(" -> ")
                    ),
                    category: self.localized_category("Theory"),
                    state: KropelkaState::CreativeSpark,
                    action_type: Some("InsertMidiProgression".to_string()),
                    action_data: Some(serde_json::json!({
                        "chords": chord_names,
                        "vibe": prog.vibe
                    })),
                    choices: Some(vec![
                        self.localized_choice("apply"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        } else if choice < 0.85 {
            // Suggest Modulation (Circle of Fifths)
            let key = crate::engine::theory::Key {
                root: "C".to_string(),
                scale_type: "Major".to_string(),
            };
            let neighbors = key.neighbors();
            if let Some(target) = neighbors.choose(&mut rng) {
                return Some(KropelkaInsight {
                    text: format!(
                        "Feeling repetitive? Modulate to {} {} (Circle of Fifths neighbor).",
                        target.root, target.scale_type
                    ),
                    category: self.localized_category("Theory"),
                    state: KropelkaState::CreativeSpark,
                    action_type: Some("ModulateProject".to_string()),
                    action_data: Some(
                        serde_json::json!({ "root": target.root, "scale": target.scale_type }),
                    ),
                    choices: Some(vec![
                        self.localized_choice("apply"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        } else {
            // Suggest Negative Harmony (NEW)
            return Some(KropelkaInsight {
                text: "Want to try Negative Harmony? It can flip your melody into an exotic territory.".to_string(),
                category: self.localized_category("Theory"),
                state: KropelkaState::CreativeSpark,
                action_type: Some("ApplyNegativeHarmony".to_string()),
                action_data: Some(serde_json::json!({"axis": "C-G"})),
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // Fallback to existing theory insight
        self.generate_theory_insight()
    }

    fn generate_theory_insight(&self) -> Option<KropelkaInsight> {
        let mut rng = rand::thread_rng();
        if let Some(scale) = self.knowledge_base.theory.scales.choose(&mut rng) {
            return Some(KropelkaInsight {
                text: format!(
                    "Feeling stuck? Let's try {} vibe. It's often used for '{}' tracks.",
                    scale.name, scale.vibe
                ),
                category: "Theory".to_string(),
                state: KropelkaState::CreativeSpark,
                action_type: Some("SetProjectScale".to_string()),
                action_data: Some(serde_json::json!({"scale": scale.name})),
                choices: None, // No choices for basic scale suggestion
                emotion: None,
            });
        }
        None
    }

    fn generate_genre_insight(&self) -> Option<KropelkaInsight> {
        let mut rng = rand::thread_rng();
        if let Some(genre) = self.knowledge_base.genres.choose(&mut rng) {
            return Some(KropelkaInsight {
                text: format!(
                    "Why not try a {} style? Target BPM: {}-{}.",
                    genre.name, genre.bpm_range.0, genre.bpm_range.1
                ),
                category: "Theory".to_string(),
                state: KropelkaState::CreativeSpark,
                action_type: Some("ApplyGenreTemplate".to_string()),
                action_data: Some(
                    serde_json::json!({"genre": genre.name, "bpm": genre.bpm_range.0}),
                ),
                choices: None,
                emotion: None,
            });
        }
        None
    }

    fn generate_vibe_insight(
        &self,
        analysis: &crate::engine::kropelka::MixAnalysis,
    ) -> Option<KropelkaInsight> {
        // LUFS Check
        if analysis.lufs_level < -18.0 && analysis.rms_level > 0.3 {
            return Some(KropelkaInsight {
                text: format!("The mix is a bit quiet ({:.1} LUFS). Need to squeeze it for more competitive loudness?", analysis.lufs_level),
                category: self.localized_category("Mastering"),
                state: KropelkaState::VibeCheck,
                action_type: Some("ApplyLimiter".to_string()),
                action_data: Some(serde_json::json!({"gain": 3.0})),
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        if analysis.rms_level > 0.4 && !analysis.clipping_detected {
            return Some(KropelkaInsight {
                text: self.t("vibe_masterpiece", None),
                category: self.localized_category("Vibe"),
                state: KropelkaState::VibeCheck,
                action_type: Some("OpenExportWindow".to_string()),
                action_data: None,
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }
        None
    }

    fn analyze_mix_balance(
        &self,
        tracks: &[crate::engine::graph::Track],
        track_levels: &[crate::engine::graph::TrackLevel],
    ) -> Option<KropelkaInsight> {
        if tracks.len() < 2 || track_levels.is_empty() {
            return None;
        }

        let mut max_rms: f32 = 0.0;
        let mut loudest_idx = 0;
        let mut min_rms: f32 = 1.0;
        let mut quietest_idx = 0;

        for (idx, level) in track_levels.iter().enumerate() {
            if let Some(&rms) = level.rms.first() {
                if rms > max_rms {
                    max_rms = rms;
                    loudest_idx = idx;
                }
                if rms > 0.005 && rms < min_rms {
                    min_rms = rms;
                    quietest_idx = idx;
                }
            }
        }

        // If one track is heavily dominating an active quiet track
        if max_rms > 0.4 && min_rms > 0.005 && max_rms > min_rms * 4.0 {
            let loudest_name = tracks.get(loudest_idx).map(|t| t.name.clone()).unwrap_or_else(|| format!("Track {}", loudest_idx + 1));
            let quietest_name = tracks.get(quietest_idx).map(|t| t.name.clone()).unwrap_or_else(|| format!("Track {}", quietest_idx + 1));

            // Only suggest if these tracks are different
            if loudest_idx != quietest_idx {
                return Some(KropelkaInsight {
                    text: format!("Mix Balance check: '{}' is dominating the mix, while '{}' is buried. Want me to balance them out?", loudest_name, quietest_name),
                    category: self.localized_category("Mixing"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("BalanceTracks".to_string()),
                    action_data: Some(serde_json::json!({
                        "loud_track": loudest_idx,
                        "loud_gain_delta": -3.0,
                        "quiet_track": quietest_idx,
                        "quiet_gain_delta": 2.5
                    })),
                    choices: Some(vec![
                        self.localized_choice("apply"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        None
    }

    fn generate_smart_eq_insight(
        &self,
        analysis: &crate::engine::kropelka::MixAnalysis,
    ) -> Option<KropelkaInsight> {
        let bands = analysis.frequency_bands;
        // Check for common masking patterns
        if bands[0] > 0.4 && bands[1] > 0.3 {
            return Some(KropelkaInsight {
                text: self.t("muddy_mix", None),
                category: self.localized_category("SmartEQ"),
                state: KropelkaState::ProducerMode,
                action_type: Some("ApplySmartEQ".to_string()),
                action_data: Some(serde_json::json!({"cut_hz": 60, "gain": -3.0})),
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }
        if bands[3] > 0.4 {
            return Some(KropelkaInsight {
                text: "Ouch, the 2-4kHz range is very aggressive. Lowering this can make the mix sound sweeter and less fatiguing.".to_string(),
                category: self.localized_category("SmartEQ"),
                state: KropelkaState::ProducerMode,
                action_type: Some("ApplySmartEQ".to_string()),
                action_data: Some(serde_json::json!({"cut_hz": 3000, "gain": -2.5})),
                choices: Some(vec![
                    self.localized_choice("apply"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }
        None
    }

    fn analyze_cross_track_masking(
        &self,
        tracks: &[crate::engine::graph::Track],
        track_levels: &[crate::engine::graph::TrackLevel],
    ) -> Option<KropelkaInsight> {
        let mut kick_idx: Option<usize> = None;
        let mut bass_idx: Option<usize> = None;

        for (i, t) in tracks.iter().enumerate() {
            let name = t.name.to_lowercase();
            if name.contains("kick") { kick_idx = Some(i); }
            if name.contains("bass") || name.contains("808") { bass_idx = Some(i); }
        }

        // If we found both kick and bass
        if let (Some(k), Some(b)) = (kick_idx, bass_idx) {
            let k_rms = track_levels.get(k).and_then(|tl| tl.rms.first()).cloned().unwrap_or(0.0);
            let b_rms = track_levels.get(b).and_then(|tl| tl.rms.first()).cloned().unwrap_or(0.0);

            // If both are hitting hard simultaneously (masking in the sub range)
            if k_rms > 0.3 && b_rms > 0.3 {
                return Some(KropelkaInsight {
                    text: format!("I hear masking! The '{}' and '{}' are fighting loudly for the low-end. Want to set up Sidechain ducking?", tracks[k].name, tracks[b].name),
                    category: self.localized_category("Mixing"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("SidechainSuggestion".to_string()),
                    action_data: Some(serde_json::json!({"trigger": k, "target": b})),
                    choices: Some(vec![
                        self.localized_choice("apply"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: Some("concerned".to_string()),
                });
            }
        }

        None
    }

    fn generate_groove_insight(&self, scene: &SceneDensity) -> Option<KropelkaInsight> {
        // If density is high, check if we want to suggest a groove
        if scene.midi_note_density > 5.0 && rand::random::<f32>() > 0.8 {
            return Some(KropelkaInsight {
                text: self.t("stiff_rhythm", None),
                category: self.localized_category("Groove"),
                state: KropelkaState::CreativeSpark,
                action_type: Some("ApplyGroove".to_string()),
                action_data: Some(serde_json::json!({"template": "MPC 60 Classic Swing"})),
                choices: Some(vec![
                    self.localized_choice("swing_it"),
                    self.localized_choice("nah"),
                ]),
                emotion: None,
            });
        }
        None
    }

    fn update_state(&mut self, analysis: &crate::engine::kropelka::MixAnalysis, context: &str) {
        if analysis.clipping_detected || analysis.rms_level > 0.95 {
            self.current_state = KropelkaState::TechnicalGuardian;
        } else if context == "Empty" {
            self.current_state = KropelkaState::CreativeSpark;
        } else if context == "Mastering" {
            self.current_state = KropelkaState::VibeCheck;
        } else {
            // New Phase 4 logic:
            // If user has been active for a while and mix is clean, enter Producer Mode
            // For now, heuristic: 20% chance to be in ProducerMode if in FlowState
            if self.current_state == KropelkaState::FlowState
                || self.current_state == KropelkaState::ProducerMode
            {
                if rand::random::<f32>() > 0.7 {
                    self.current_state = KropelkaState::ProducerMode;
                } else {
                    self.current_state = KropelkaState::FlowState;
                }
            } else {
                self.current_state = KropelkaState::FlowState;
            }
        }
    }

    /// SONG STRUCTURE ANALYSIS (Phase 3)
    pub fn analyze_structure(
        &self,
        tracks: &[crate::engine::graph::Track],
        markers: &[crate::engine::graph::Marker],
    ) -> Option<KropelkaInsight> {
        if tracks.is_empty() {
            return None;
        }

        // 1. Calculate Clip Density across timeline (Heuristic: 120bpm, 48k => ~8sec blocks)
        const BLOCK_SIZE: u64 = 48000 * 8;
        let mut density = vec![0; 60]; // ~480 seconds (8 minutes)

        let mut max_pos = 0;
        for track in tracks {
            for clip in &track.clips {
                let end = clip.start_sample + clip.length_in_samples;
                if end > max_pos {
                    max_pos = end;
                }
                let start_idx = (clip.start_sample / BLOCK_SIZE) as usize;
                let end_idx = (end / BLOCK_SIZE) as usize;
                for i in start_idx..=end_idx {
                    if i < density.len() {
                        density[i] += 1;
                    }
                }
            }
            for m_clip in &track.midi_clips {
                let end = m_clip.start_sample + m_clip.length_samples;
                if end > max_pos {
                    max_pos = end;
                }
                let start_idx = (m_clip.start_sample / BLOCK_SIZE) as usize;
                let end_idx = (end / BLOCK_SIZE) as usize;
                for i in start_idx..=end_idx {
                    if i < density.len() {
                        density[i] += 1;
                    }
                }
            }
        }

        if max_pos == 0 {
            return None;
        }

        // Find "Energy Holes" (sudden drop in density followed by rise)
        for i in 1..density.len() - 1 {
            if i * (BLOCK_SIZE as usize) > max_pos as usize {
                break;
            }
            if density[i] < density[i - 1] / 2 && density[i + 1] > density[i] * 2 {
                return Some(KropelkaInsight {
                    text: self.t("energy_drop", None),
                    category: self.localized_category("Vibe"),
                    state: KropelkaState::CreativeSpark,
                    action_type: Some("SuggestTransition".to_string()),
                    action_data: Some(
                        serde_json::json!({"pos": i as u64 * BLOCK_SIZE, "type": "Riser"}),
                    ),
                    choices: Some(vec![
                        self.localized_choice("riser"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // Suggest markers if empty
        if markers.is_empty() {
            use super::gosposia::Gosposia;
            let detected_sections = Gosposia::autolabel_sections(max_pos, &density);

            return Some(KropelkaInsight {
                text: self.t("no_markers", None),
                category: self.localized_category("Theory"),
                state: KropelkaState::CreativeSpark,
                action_type: Some("AutoLabelSections".to_string()),
                action_data: Some(serde_json::json!({ "sections": detected_sections })),
                choices: Some(vec![
                    self.localized_choice("yes"),
                    self.localized_choice("no"),
                ]),
                emotion: None,
            });
        }

        None
    }

    /// LEVEL 2: GOSPOSIA (Housekeeper) - Organization Analysis
    pub fn analyze_organization(
        &self,
        tracks: &[crate::engine::graph::Track],
    ) -> Option<KropelkaInsight> {
        use super::gosposia::Gosposia;

        // 1. Check for Dead Tracks
        for track in tracks {
            if Gosposia::is_dead_track(track) {
                return Some(KropelkaInsight {
                    text: self.t("dead_track", Some(&[&track.name])),
                    category: self.localized_category("Housekeeping"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("RemoveTrack".to_string()),
                    action_data: Some(serde_json::json!({ "track_id": track.id })),
                    choices: Some(vec![
                        self.localized_choice("remove"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // 2. Clip Tidy (Micro-fragments)
        for track in tracks {
            if let Some(clip_id) = Gosposia::suggest_clip_tidy(track) {
                return Some(KropelkaInsight {
                    text: self.t("clip_tidy", None),
                    category: self.localized_category("Housekeeping"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("RemoveClip".to_string()),
                    action_data: Some(serde_json::json!({ "clip_id": clip_id })),
                    choices: Some(vec![
                        self.localized_choice("remove"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // 3. Plugin Dusting (Bypassed plugins)
        for track in tracks {
            if let Some((plugin_id, plugin_name)) = Gosposia::suggest_plugin_dusting(track) {
                return Some(KropelkaInsight {
                    text: self.t("plugin_dusting", Some(&[&plugin_name, &track.name])),
                    category: self.localized_category("Housekeeping"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("RemovePlugin".to_string()),
                    action_data: Some(
                        serde_json::json!({ "track_id": track.id, "plugin_id": plugin_id }),
                    ),
                    choices: Some(vec![
                        self.localized_choice("remove"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // 4. Automation Cleaner
        for track in tracks {
            if let Some(param_id) = Gosposia::suggest_automation_cleanup(track) {
                return Some(KropelkaInsight {
                    text: self.t("automation_cleaner", Some(&[&track.name])),
                    category: self.localized_category("Housekeeping"),
                    state: KropelkaState::ProducerMode,
                    action_type: Some("ClearAutomation".to_string()),
                    action_data: Some(
                        serde_json::json!({ "track_id": track.id, "param_id": param_id }),
                    ),
                    choices: Some(vec![
                        self.localized_choice("tidy"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // 5. Sample Librarian (Duplicates)
        if let Some(sample_name) = Gosposia::suggest_sample_cleanup(tracks) {
            return Some(KropelkaInsight {
                text: self.t("sample_librarian", Some(&[&sample_name])),
                category: self.localized_category("Organization"),
                state: KropelkaState::ProducerMode,
                action_type: Some("ConsolidateSamples".to_string()),
                action_data: None,
                choices: Some(vec![
                    self.localized_choice("tidy"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // 6. Smart Foldering Suggestions
        let folders = Gosposia::suggest_folders(tracks);
        if let Some((role, track_ids)) = folders.first() {
            let role_name = Gosposia::get_role_folder_name(role);
            return Some(KropelkaInsight {
                text: self.t(
                    "smart_foldering",
                    Some(&[&track_ids.len().to_string(), role_name]),
                ),
                category: self.localized_category("Organization"),
                state: KropelkaState::ProducerMode,
                action_type: Some("GroupTracks".to_string()),
                action_data: Some(
                    serde_json::json!({ "track_ids": track_ids, "folder_name": role_name }),
                ),
                choices: Some(vec![
                    self.localized_choice("tidy"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        None
    }

    /// LEVEL 3: TECHNIK (System Guardian) - Technical Analysis
    pub fn analyze_technical_health(
        &self,
        tracks: &[crate::engine::graph::Track],
        sample_rate: f64,
        cpu_load: f32,
    ) -> Option<KropelkaInsight> {
        use super::technik::Technik;

        // 1. Critical System CPU Alert (>85%)
        let health = Technik::monitor_system_health(cpu_load);
        if health.cpu_percent > 85.0 {
            return Some(KropelkaInsight {
                text: self.t(
                    "high_cpu_alert",
                    Some(&[&format!("{:.0}", health.cpu_percent)]),
                ),
                category: self.localized_category("Technical"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("ShowPerformance".to_string()),
                action_data: None,
                choices: Some(vec![
                    self.localized_choice("explain"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // 2. Latency / Buffer size suggestion (Low latency during heavy mixing)
        if health.cpu_percent > 60.0 {
            return Some(KropelkaInsight {
                text: self.t("latency_control", None),
                category: self.localized_category("Technical"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("OptimizeBuffer".to_string()),
                action_data: None,
                choices: Some(vec![
                    self.localized_choice("yes"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // 3. Silence Sweeper
        for track in tracks {
            if Technik::suggest_silence_sweep(track).is_some() {
                return Some(KropelkaInsight {
                    text: self.t("silence_sweeper", Some(&[&track.name])),
                    category: self.localized_category("Technical"),
                    state: KropelkaState::TechnicalGuardian,
                    action_type: Some("RemoveSilences".to_string()),
                    action_data: Some(serde_json::json!({ "track_id": track.id })),
                    choices: Some(vec![
                        self.localized_choice("tidy"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        // 4. Check for Freeze suggestions (>15% CPU for a single track)
        if let Some((track_id, track_name, cpu_percent)) =
            Technik::suggest_freeze(tracks, sample_rate)
        {
            return Some(KropelkaInsight {
                text: self.t(
                    "freeze_advisor",
                    Some(&[&track_name, &format!("{:.1}", cpu_percent)]),
                ),
                category: self.localized_category("Technical"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("FreezeTrack".to_string()),
                action_data: Some(serde_json::json!({ "track_id": track_id })),
                choices: Some(vec![
                    self.localized_choice("freeze"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // 2. Project Integrity check
        let issues = Technik::check_project_integrity(tracks);
        if let Some(first_issue) = issues.first() {
            return Some(KropelkaInsight {
                text: self.t("project_integrity", Some(&[first_issue])),
                category: self.localized_category("Technical"),
                state: KropelkaState::TechnicalGuardian,
                action_type: Some("FixIntegrity".to_string()),
                action_data: None,
                choices: Some(vec![
                    self.localized_choice("fix"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        None
    }

    /// LEVEL 4: CO-PILOT (Creative Partner) - Creative Analysis
    pub fn analyze_creative_opportunities(
        &self,
        tracks: &[crate::engine::graph::Track],
        _bpm: f64,
        _sample_rate: f64,
    ) -> Option<KropelkaInsight> {
        // 1. Drum Generation (If no drums detected)
        let has_drums = tracks.iter().any(|t| {
            let name = t.name.to_lowercase();
            name.contains("drums") || name.contains("beat") || name.contains("kick")
        });

        if !has_drums && !tracks.is_empty() {
            return Some(KropelkaInsight {
                text: self.t("suggest_drums", None),
                category: self.localized_category("Creative"),
                state: KropelkaState::CreativeSpark,
                action_type: Some("GenerateDrums".to_string()),
                action_data: Some(serde_json::json!({ "style": "TechnoHouse" })),
                choices: Some(vec![
                    self.localized_choice("yes"),
                    self.localized_choice("ignore"),
                ]),
                emotion: None,
            });
        }

        // 2. Melody Inpainting (If a track has a short midi clip)
        for track in tracks {
            if track.track_type == crate::engine::graph::TrackType::MIDI {
                for clip in &track.midi_clips {
                    if clip.notes.len() >= 2 && clip.notes.len() <= 8 {
                        return Some(KropelkaInsight {
                            text: self.t("suggest_melody_extend", Some(&[&track.name])),
                            category: self.localized_category("Creative"),
                            state: KropelkaState::CreativeSpark,
                            action_type: Some("ExtendMelody".to_string()),
                            action_data: Some(
                                serde_json::json!({ "track_id": track.id, "clip_id": clip.id }),
                            ),
                            choices: Some(vec![
                                self.localized_choice("yes"),
                                self.localized_choice("ignore"),
                            ]),
                            emotion: None,
                        });
                    }
                }
            }
        }

        // 3. Chord Wizard (If there are midi chords detected)
        // Simplified detection: if track "Pad" or "Chords" exists
        for track in tracks {
            let name = track.name.to_lowercase();
            if name.contains("chord") || name.contains("pad") || name.contains("keys") {
                return Some(KropelkaInsight {
                    text: self.t("suggest_chord_wizard", None),
                    category: self.localized_category("Theory"),
                    state: KropelkaState::CreativeSpark,
                    action_type: Some("SuggestNextChord".to_string()),
                    action_data: Some(serde_json::json!({ "track_id": track.id })),
                    choices: Some(vec![
                        self.localized_choice("yes"),
                        self.localized_choice("ignore"),
                    ]),
                    emotion: None,
                });
            }
        }

        None
    }
}
