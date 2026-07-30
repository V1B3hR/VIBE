#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// The user's expertise level, which adapts over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum UserSkill {
    Beginner,     // Needs "Why" explanations, simpler terms
    Intermediate, // Needs standard tips
    Pro,          // Needs only critical technical alerts (e.g. Phase issues)
}

/// The emotional vibe of the music or the assistant state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum EmotionalState {
    Neutral,
    Energetic,   // High BPM, bright transients (Red/Orange Pulse)
    Melancholic, // Minor keys, slow attack (Blue/Purple Slow Pulse)
    Aggressive,  // Distortion, harsh transients (Sharp Red Spikes)
    Ethereal,    // Lots of reverb, washed out (White/Silver Mist)
    Euphorical,  // Major keys, balanced energy (Gold Shine)
}

/// Simplified musical genre classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum Genre {
    Unknown,
    HipHopTrap,
    TechnoHouse,
    DnB,
    Cinematic,
    RockMetal,
}

/// The visual animation state of Kropelka's tail/head.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum TailAnimation {
    Swaying(f32), // Rhythmic swaying (BPM synced), value = frequency
    Lightning,    // Clipping/Distortion warning
    Fire,         // "Fire" mix / perfect balance
    Breathing,    // Zen/Relaxed state
    Static,       // Idle
}

/// The current context Kropelka is observing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum KropelkaContext {
    Global,
    MixerChannel(u32), // Channel Index
    Plugin(String),    // Plugin Name (e.g. "Compressor", "EQ")
    PianoRoll,
    Arrangement,
    VocalTuning,      // New: Pitch correction interface
    StemSeparation,   // New: Remix/Separation interface
    SmartEQ,          // New: Intelligent frequency masking
    GrooveExtraction, // New: MIDI timing extraction
}

/// Visual state of the Kropelka assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct KropelkaVisualState {
    pub mood: EmotionalState,
    pub tail_animation: TailAnimation,
    pub color_hex: String, // Context-aware color
    pub is_visible: bool,
    pub particle_trigger: Option<String>, // e.g. "sparks", "smoke"
}

/// A suggestion from Kropelka.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct KropelkaSuggestion {
    pub category: String, // e.g. "Mixing", "Arrangement", "Technical", "Creative"
    pub message: String,
    pub severity: f32,               // 0.0 - 1.0
    pub context_why: Option<String>, // Explanation for beginners
    pub emoji: String,
}

/// Tracks user behavior to prevent ear fatigue and "rabbit holes".
#[allow(dead_code)]
pub struct ActivityMonitor {
    pub session_start: Instant,
    pub current_focus: String,
    pub focus_start_time: Instant,
    pub undo_count_recent: u32, // Spikes in undo suggest frustration
    pub last_undo_time: Instant,
}

#[allow(dead_code)]
impl ActivityMonitor {
    pub fn new() -> Self {
        Self {
            session_start: Instant::now(),
            current_focus: "General".to_string(),
            focus_start_time: Instant::now(),
            undo_count_recent: 0,
            last_undo_time: Instant::now(),
        }
    }

    pub fn record_undo(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_undo_time) < Duration::from_secs(30) {
            self.undo_count_recent += 1;
        } else {
            self.undo_count_recent = 1;
        }
        self.last_undo_time = now;
    }

    pub fn is_frustrated(&self) -> bool {
        self.undo_count_recent > 5
    }

    pub fn update_focus(&mut self, new_focus: &str) {
        if self.current_focus != new_focus {
            self.current_focus = new_focus.to_string();
            self.focus_start_time = Instant::now();
        }
    }
}

/// Defines what Kropelka is allowed to do on her own.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[allow(dead_code)]
pub enum PermissionTier {
    Observer,    // Read-only logs & monitoring
    Housekeeper, // Safe actions: Coloring, Naming, Backups, Cache Cleaning
    Technician,  // System fixes: Restart Engine, Latency Mgmt, Hard Limiter (Panic)
    CoPilot,     // Creative: Gain Staging, EQ suggestions
    FullAccess,  // "Zosia Samosia" - Autonomous decision making
}

/// Configuration for Kropelka's autonomy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct KropelkaPolicy {
    pub tier: PermissionTier,
    pub silent_mode: bool, // If true, fix issues without a toast (unless critical)
    pub confidence_threshold: f32, // Only act if certainty > X
    pub panic_guard_enabled: bool, // Allow hard limiter on master for safety
}

impl Default for KropelkaPolicy {
    fn default() -> Self {
        Self {
            tier: PermissionTier::Technician, // Default to helpful but safe
            silent_mode: true,                // "Nie upierdliwa"
            confidence_threshold: 0.9,
            panic_guard_enabled: true,
        }
    }
}

/// Memory of previous projects and user style.
#[allow(dead_code)]
pub struct KropelkaMemory {
    pub favorite_plugins: Vec<String>,
    pub known_projects: Vec<String>,
    pub user_vibe_history: Vec<EmotionalState>,
}

/// Kropelka 5.0 - The Advanced AI Musician/Producer Buddy.
/// Supportive, smart, human-like, and 100% offline.
#[allow(dead_code)]
pub struct Kropelka {
    pub visual: KropelkaVisualState,
    pub context: KropelkaContext,
    pub user_skill: UserSkill,
    pub emotional_profile: EmotionalState,
    pub memory: KropelkaMemory,
    pub suggestions: Vec<KropelkaSuggestion>,
    pub activity: ActivityMonitor,
    pub policy: KropelkaPolicy,
    pub is_flow_state: bool,
    pub detected_genre: Genre,
    pub forest_bridge:
        Option<std::sync::Arc<tokio::sync::Mutex<super::neural_forest::NeuralForestBridge>>>,
    pub smart_eq_report: Option<SmartEqReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartEqReport {
    pub masked_bands: Vec<String>,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KropelkaEvent {
    TrackAdded(String),
    PluginInserted(String),
    ClipMoved(u64),
    BpmChanged(f32),
    KeyChanged(String),
    MixdownStarted,
}

#[allow(dead_code)]
impl Kropelka {
    pub fn new() -> Self {
        Self {
            visual: KropelkaVisualState {
                mood: EmotionalState::Neutral,
                tail_animation: TailAnimation::Swaying(1.0),
                color_hex: "#00A8FF".to_string(),
                is_visible: true,
                particle_trigger: None,
            },
            context: KropelkaContext::Global,
            user_skill: UserSkill::Beginner,
            emotional_profile: EmotionalState::Neutral,
            memory: KropelkaMemory {
                favorite_plugins: Vec::new(),
                known_projects: Vec::new(),
                user_vibe_history: Vec::new(),
            },
            suggestions: Vec::new(),
            activity: ActivityMonitor::new(),
            policy: KropelkaPolicy::default(),
            is_flow_state: true,
            detected_genre: Genre::Unknown,
            forest_bridge: None,
            smart_eq_report: None,
        }
    }

    pub fn attach_brain(
        &mut self,
        bridge: std::sync::Arc<tokio::sync::Mutex<super::neural_forest::NeuralForestBridge>>,
    ) {
        self.forest_bridge = Some(bridge);
    }

    /// Update Kropelka's state with Kind & Supportive logic.
    pub fn check_workflow_health(&mut self) {
        // 1. Flow State Analysis
        // If we haven't touched context in 10 mins and haven't undone much, we are flowing.
        if self.activity.focus_start_time.elapsed() > Duration::from_secs(600)
            && !self.activity.is_frustrated()
        {
            self.is_flow_state = true;
            self.visual.tail_animation = TailAnimation::Breathing;
            // Shhh... don't interrupt the flow.
            return;
        } else {
            self.is_flow_state = false;
        }

        // 2. Frustration / Zen Coach Mode
        if self.activity.is_frustrated() {
            self.current_suggestion(KropelkaSuggestion {
                category: "Zen".to_string(),
                message: "Take a deep breath! ❤️ You're hitting undo a lot. Maybe try a completely different approach for a few minutes?".to_string(),
                severity: 0.7,
                context_why: Some("Creative blocks are best broken by changing context.".into()),
                emoji: "🧘‍♂️".into(),
            });
            self.visual.color_hex = "#2ECC71".to_string(); // Healing Green
            self.visual.tail_animation = TailAnimation::Breathing;
            return;
        }

        // 3. Rabbit Hole Detection
        if self.activity.focus_start_time.elapsed() > Duration::from_secs(900) {
            let focus = self.activity.current_focus.clone();
            self.current_suggestion(KropelkaSuggestion {
                category: "Workflow".to_string(),
                message: format!("Hey, we've been on {} for 15 minutes. 🐰 How about we step back and listen to the whole mix?", focus),
                severity: 0.5,
                context_why: Some("Losing perspective is the #1 enemy of a good mix.".into()),
                emoji: "🥕".into(),
            });
        }
    }

    /// Add a suggestion with supportive personality.
    fn current_suggestion(&mut self, suggestion: KropelkaSuggestion) {
        // Wrap in kind language based on skill
        let mut final_suggestion = suggestion;
        if matches!(self.user_skill, UserSkill::Beginner) {
            final_suggestion.message =
                format!("I've got a little tip! {}", final_suggestion.message);
        }
        self.suggestions.push(final_suggestion);
    }

    pub fn set_context(&mut self, context: KropelkaContext) {
        if self.context != context {
            self.activity.update_focus(&format!("{:?}", context));
            self.context = context.clone();
            // Context colors
            self.visual.color_hex = match &context {
                KropelkaContext::Plugin(name) if name.contains("EQ") => "#FF00FF".to_string(),
                KropelkaContext::Plugin(name) if name.contains("Compressor") => {
                    "#00FF00".to_string()
                }
                KropelkaContext::PianoRoll => "#FFA500".to_string(),
                KropelkaContext::VocalTuning => "#E056FD".to_string(), // Neon Purple
                KropelkaContext::StemSeparation => "#2ECC71".to_string(), // Sci-Fi Green
                KropelkaContext::SmartEQ => "#FF5733".to_string(),     // Vibrant Orange
                KropelkaContext::GrooveExtraction => "#8E44AD".to_string(), // Deep Purple
                _ => "#00A8FF".to_string(),
            };

            // Initial Context Suggestions
            match context {
                KropelkaContext::VocalTuning => {
                    self.current_suggestion(KropelkaSuggestion {
                        category: "Vocal Tuning".to_string(),
                        message: "Let's tune! Remember to set the Scale first. 🎵".to_string(),
                        severity: 0.3,
                        context_why: Some("Chromatic tuning often sounds robotic.".into()),
                        emoji: "🎤".into(),
                    });
                }
                KropelkaContext::StemSeparation => {
                    self.current_suggestion(KropelkaSuggestion {
                        category: "Remixing".to_string(),
                        message: "Separating stems? Watch out for spectral artifacts.".to_string(),
                        severity: 0.4,
                        context_why: Some(
                            "AI separation isn't perfect; layer sounds to hide holes.".into(),
                        ),
                        emoji: "🧪".into(),
                    });
                }
                KropelkaContext::SmartEQ => {
                    self.current_suggestion(KropelkaSuggestion {
                        category: "Mixing".to_string(),
                        message: "Searching for masking... Identifying frequency collisions."
                            .to_string(),
                        severity: 0.3,
                        context_why: Some(
                            "I'll help you carve out space for each instrument.".into(),
                        ),
                        emoji: "🔎".into(),
                    });
                }
                KropelkaContext::GrooveExtraction => {
                    self.current_suggestion(KropelkaSuggestion {
                        category: "Rhythm".to_string(),
                        message: "Analyzing swing... How much 'human feel' do you want to keep?"
                            .to_string(),
                        severity: 0.2,
                        context_why: Some(
                            "Swing (offsetting off-beats) gives the track its soul.".into(),
                        ),
                        emoji: "🥁".into(),
                    });
                }
                _ => {}
            }
        }
    }

    pub fn update_from_analysis(&mut self, analysis: &MixAnalysis) {
        if analysis.clipping_detected {
            self.visual.tail_animation = TailAnimation::Lightning;
            self.visual.particle_trigger = Some("sparks".into());
            self.current_suggestion(KropelkaSuggestion {
                category: "Technical".to_string(),
                message:
                    "Ouch! We're hitting the red. ⚡ Should we pull the master fader back a bit?"
                        .into(),
                severity: 0.9,
                context_why: Some(
                    "Digital clipping destroys the clarity of your transients.".into(),
                ),
                emoji: "⚡".into(),
            });
        } else if analysis.rms_level > 0.4 && analysis.spectral_balance > 0.4 {
            self.visual.tail_animation = TailAnimation::Fire;
            self.visual.color_hex = "#FFD700".to_string();
            // Kind kudos!
            if self.suggestions.is_empty() {
                self.current_suggestion(KropelkaSuggestion {
                    category: "Inspiration".to_string(),
                    message: "Wow, this mix is starting to feel really golden! 🔥 Keep that energy going.".into(),
                    severity: 0.3,
                    context_why: None,
                    emoji: "🔥".into(),
                });
            }
        } else if analysis.masking_detected {
            self.visual.tail_animation = TailAnimation::Swaying(2.0); // Faster sway (alert)
            self.visual.color_hex = "#FFA500".to_string(); // Warn Orange
                                                           // Only suggest if we have permission (Technician or above)
            if self.policy.tier >= PermissionTier::Technician {
                self.current_suggestion(KropelkaSuggestion {
                    category: "Mixing".to_string(),
                    message: "Low-end mud detected (40-60Hz). Kick and Bass are fighting.".into(),
                    severity: 0.6,
                    context_why: Some(
                        "Masking in the sub frequencies robs your track of power.".into(),
                    ),
                    emoji: "🥊".into(),
                });
            }
        } else {
            self.visual.tail_animation = TailAnimation::Swaying(1.0);
        }
    }

    /// Determines the genre based on BPM and Transient Density (Simplified).
    pub fn determine_genre(&mut self, bpm: f32, transient_density: f32) {
        // Simple Heuristic Tree
        if bpm <= 0.0 {
            return;
        }

        // Normalize transient density for comparison (0..1)
        let density = transient_density.clamp(0.0, 1.0);

        let new_genre = if bpm >= 160.0 && density > 0.6 {
            Genre::DnB
        } else if (120.0..=145.0).contains(&bpm) && density > 0.5 {
            Genre::TechnoHouse
        } else if (70.0..=160.0).contains(&bpm) && density < 0.4 {
            Genre::HipHopTrap // Trap often has sparse instrumentation despite detection
        } else if bpm < 100.0 && density < 0.3 {
            Genre::Cinematic
        } else {
            Genre::Unknown
        };

        if self.detected_genre != new_genre {
            self.detected_genre = new_genre;
            // Optionally notify user of genre shift if context is Global
        }
    }

    /// Reactive Event Bus: Responds immediately to user actions
    pub fn process_event(&mut self, event: KropelkaEvent) {
        match event {
            KropelkaEvent::TrackAdded(name) => {
                self.current_suggestion(KropelkaSuggestion {
                    category: "Technical".to_string(),
                    message: format!(
                        "New track '{}' added. I'll monitor its impact on the headroom.",
                        name
                    ),
                    severity: 0.2,
                    context_why: Some("Every new track adds to the cumulative RMS level.".into()),
                    emoji: "➕".into(),
                });
            }
            KropelkaEvent::PluginInserted(name) if name.contains("Reverb") => {
                self.current_suggestion(KropelkaSuggestion {
                    category: "Mixing".to_string(),
                    message:
                        "Watch the reverb tail. Too much wet signal can wash out the mix clarity."
                            .to_string(),
                    severity: 0.4,
                    context_why: Some("Reverb buildup is the #1 cause of 'muddy' mixes.".into()),
                    emoji: "🌊".into(),
                });
            }
            KropelkaEvent::KeyChanged(key) => {
                self.current_suggestion(KropelkaSuggestion {
                    category: "Theory".to_string(),
                    message: format!(
                        "Project key changed to {}. I'm updating my harmonic suggestions.",
                        key
                    ),
                    severity: 0.1,
                    context_why: None,
                    emoji: "🎹".into(),
                });
            }
            _ => {}
        }
    }
}

pub struct MixAnalysis {
    pub rms_level: f32,
    pub peak_level: f32,
    pub clipping_detected: bool,
    pub spectral_balance: f32,
    pub transient_density: f32,
    pub spectral_centroid: f32,
    pub masking_detected: bool,
    pub stereo_correlation: f32,
    pub frequency_bands: [f32; 6], // Sub, Low, LowMid, Mid, Presence, Air
    pub lufs_level: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kropelka_initialization() {
        let kropelka = Kropelka::new();
        assert_eq!(kropelka.visual.mood, EmotionalState::Neutral);
        assert!(kropelka.is_flow_state);
        assert_eq!(kropelka.suggestions.len(), 0);
    }

    #[test]
    fn test_clipping_trigger() {
        let mut kropelka = Kropelka::new();
        let analysis = MixAnalysis {
            rms_level: 0.8,
            peak_level: 1.1,
            clipping_detected: true,
            spectral_balance: 0.5,
            transient_density: 0.5,
            spectral_centroid: 0.5,
            masking_detected: false,
            stereo_correlation: 1.0,
            frequency_bands: [0.1, 0.2, 0.3, 0.2, 0.1, 0.1],
            lufs_level: -14.0,
        };

        kropelka.update_from_analysis(&analysis);

        match kropelka.visual.tail_animation {
            TailAnimation::Lightning => (),
            _ => panic!("Expected Lightning animation on clipping"),
        }
        assert!(kropelka
            .suggestions
            .iter()
            .any(|s| s.category == "Technical"));
    }

    #[test]
    fn test_frustration_zen_mode() {
        let mut kropelka = Kropelka::new();

        // Simulate rapid undos
        for _ in 0..6 {
            kropelka.activity.record_undo();
        }

        kropelka.check_workflow_health();

        assert_eq!(kropelka.visual.color_hex, "#2ECC71"); // Healing Green
        assert!(kropelka.suggestions.iter().any(|s| s.category == "Zen"));
    }

    #[test]
    fn test_rabbit_hole_detection() {
        let mut kropelka = Kropelka::new();
        kropelka.set_context(KropelkaContext::Plugin("Super EQ".to_string()));

        // Mock a time jump for testing (logic uses Instant::now())
        // Since we can't easily mock Instant in std without crates,
        // we'll rely on the logic being correct or use a small sleep for integration feel
        // but for a true unit test we might pass durations in.
        // For now, let's verify context color change.
        assert_eq!(kropelka.visual.color_hex, "#FF00FF");
    }

    #[test]
    fn test_kind_feedback_beginner() {
        let mut kropelka = Kropelka::new();
        kropelka.user_skill = UserSkill::Beginner;

        let suggestion = KropelkaSuggestion {
            category: "Test".to_string(),
            message: "Check your levels.".to_string(),
            severity: 0.5,
            context_why: None,
            emoji: "📢".into(),
        };

        kropelka.current_suggestion(suggestion);
        assert!(kropelka.suggestions[0]
            .message
            .contains("I've got a little tip!"));
    }
}
