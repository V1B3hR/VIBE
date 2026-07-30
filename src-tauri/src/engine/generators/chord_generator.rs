use crate::engine::graph::{MidiClip, MidiNote, ChordMarker};
use uuid::Uuid;
use rand::Rng;

pub struct ChordGeneratorSettings {
    pub genre: String,
    pub bpm: f32,
    pub sample_rate: f64,
    pub root_note: u8,
    pub complexity: f32, // Dictates extensions (7ths, 9ths)
    // --- Upgrades ---
    pub progression_preset: String, // "Tension Arc", "Story Mode", "Jazzy Loop"
    pub voicing_style: String, // "Piano Wide", "Pad Cluster"
    pub rhythm_complexity: f32, // 0.0=Sustained, 1.0=Highly Syncopated/Arp
    pub substitutions: f32, // 0.0 to 1.0 (Secondary dominants, tritone subs)
}

// Roman numeral maps
const POP_LOOP: [&str; 4] = ["I", "V", "vi", "IV"];
const EDM_MINOR: [&str; 4] = ["i", "VI", "III", "VII"];
const NEO_SOUL: [&str; 4] = ["ii", "V", "I", "VI"]; // VI is borrowed for tension
const TENSION_ARC: [&str; 8] = ["I", "vi", "ii", "V", "iii", "vi", "IV", "V"]; // Building tension over 8 bars
const STORY_MODE: [&str; 8] = ["i", "bVII", "bVI", "V", "i", "iv", "ii°", "V"]; // Harmonic minor descent

pub fn generate_chords(settings: &ChordGeneratorSettings) -> MidiClip {
    let mut notes = Vec::new();
    let mut chord_markers = Vec::new();
    
    let samples_per_beat = (60.0 / settings.bpm as f64) * settings.sample_rate;
    let samples_per_bar = samples_per_beat * 4.0;
    
    let mut rng = rand::thread_rng();

    // Select Progression
    let progression = match settings.progression_preset.to_lowercase().as_str() {
        "tension arc" => &TENSION_ARC[..],
        "story mode" => &STORY_MODE[..],
        "jazzy loop" => &NEO_SOUL[..],
        "edm" => &EDM_MINOR[..],
        _ => &POP_LOOP[..] // Default (Pop)
    };

    let total_bars = progression.len();

    // Context for substitutions
    let mut active_progression = progression.to_vec();

    // Apply Substitutions (Functional Harmony Engine upgrade)
    if settings.substitutions > 0.3 {
        for i in 0..active_progression.len() {
            let next_chord = if i + 1 < active_progression.len() { active_progression[i+1] } else { active_progression[0] };
            
            // Tritone substitution: If current is 'V' going to 'I' or 'i', swap 'V' with 'bII' -> "IIb7"
            if active_progression[i] == "V" && settings.substitutions > 0.7 && rng.gen::<f32>() < settings.substitutions {
                active_progression[i] = "bII"; // E.g., Db7 resolving to C
            }
            
            // Secondary Dominant: 'V/vi' -> "III" instead of whatever was before 'vi' if preceding 'vi'
            if next_chord == "vi" && active_progression[i] != "V" && rng.gen::<f32>() < settings.substitutions * 0.5 {
                 active_progression[i] = "III"; // Secondary dominant V/vi
            }
        }
    }

    let mut current_bar = 0;

    for chord_numeral in active_progression.iter() {
        let start_sample = (current_bar as f64 * samples_per_bar) as u64;
        
        // Translate numeral to MIDI notes with Functional Harmony / Borrowed Chords
        let (mut chord_notes, chord_name) = numeral_to_chord(
            chord_numeral, 
            settings.root_note, 
            settings.complexity
        );

        // Voicing Style Transformation
        match settings.voicing_style.to_lowercase().as_str() {
            "piano wide" => {
                // Drop 2, spread out range
                if chord_notes.len() > 3 {
                    // Move the 2nd highest note down an octave
                    let len = chord_notes.len();
                    let drop2 = chord_notes[len - 2] - 12;
                    chord_notes[len - 2] = chord_notes[0]; // move root up? No, simpler drop 2:
                    chord_notes.insert(1, drop2);
                }
                // Double root an octave lower
                if !chord_notes.is_empty() {
                    chord_notes.insert(0, chord_notes[0] - 12);
                }
            },
            "pad cluster" => {
                // Keep everything tight within 1 octave (Close voicing)
                for n in chord_notes.iter_mut() {
                    while *n > settings.root_note as u16 + 12 { *n -= 12; }
                    while *n < settings.root_note as u16 { *n += 12; }
                }
            },
            _ => {} // Standard
        }

        chord_markers.push(ChordMarker {
            sample: start_sample,
            chord_name: format!("{} {}", chord_name, chord_numeral),
            confidence: 1.0,
        });

        // Rhythm Generation Strategy
        let is_complex_rhythm = settings.rhythm_complexity > 0.5 && rng.gen::<f32>() < settings.rhythm_complexity;
        
        if is_complex_rhythm {
            // "Stabs", Syncopated Rhythms
            let stab_len = (samples_per_beat * 0.25) as u64; // 16th note length
            
            // Generate a 16-step grid of syncopation
            for step in 0..16 {
                let step_prob = match step % 4 {
                    0 => 0.8, // Downbeats
                    2 => 0.5, // 8th upbeats
                    _ => 0.2, // 16th syncopations
                };

                if rng.gen::<f32>() < step_prob * settings.rhythm_complexity {
                    let offset = start_sample + (step as f64 * samples_per_beat / 4.0) as u64;
                    for &n in &chord_notes {
                        notes.push(create_note_human(offset, stab_len, n, rng.gen_range(70..100)));
                    }
                }
            }
        } else if settings.rhythm_complexity > 0.8 {
            // "Arp"
            let arp_len_samples = samples_per_beat / 2.0;
            for step in 0..8 {
                let note_to_play = chord_notes[step % chord_notes.len()];
                let offset = start_sample + (step as f64 * arp_len_samples) as u64;
                notes.push(create_note_human(offset, arp_len_samples as u64, note_to_play, 100));
            }
        } else {
            // "Sustained" (Pad/Basic)
            let length_samples = samples_per_bar as u64;
            for &n in &chord_notes {
                // Very slight strum effect
                let strum_offset = rng.gen_range(0..800);
                notes.push(create_note_human(start_sample + strum_offset, length_samples, n, rng.gen_range(70..90)));
            }
        }

        current_bar += 1;
    }

    MidiClip {
        id: Uuid::new_v4(),
        name: format!("{} {} Chords", settings.genre, settings.progression_preset),
        start_sample: 0,
        length_samples: (total_bars as f64 * samples_per_bar) as u64,
        notes,
        cc_events: vec![],
        color: "#00CECE".into(),
        is_muted: false,
        is_looped: true,
        scale: None,
        chord_markers,
        groove_template: None,
        pattern_id: None,
        tuning_steps: None,
        time_signature_num: Some(4),
        time_signature_den: Some(4),
        reference_clip_id: None,
    }
}

// Translates functional harmony numeral to root+quality
fn numeral_to_chord(numeral: &str, global_root: u8, complexity: f32) -> (Vec<u16>, String) {
    let root_i32 = global_root as i32;
    let mut intervals = [0, 4, 7]; // Default Major

    let (root_offset, name) = match numeral {
        "I" => (0, "Maj"),
        "i" => { intervals[1] = 3; (0, "Min") },
        "bII" => (1, "Maj"), // Tritone sub logic or Neapolitan
        "II" => (2, "Maj"), // Borrowed
        "ii" => { intervals[1] = 3; (2, "Min") },
        "ii°" => { intervals[1] = 3; intervals[2] = 6; (2, "Dim") },
        "bIII" => (3, "Maj"),
        "III" => (4, "Maj"),
        "iii" => { intervals[1] = 3; (4, "Min") },
        "IV" => (5, "Maj"),
        "iv" => { intervals[1] = 3; (5, "Min") },
        "V" => (7, "Maj"),
        "v" => { intervals[1] = 3; (7, "Min") },
        "bVI" => (8, "Maj"), // Borrowed from minor
        "VI" => (9, "Maj"),
        "vi" => { intervals[1] = 3; (9, "Min") },
        "bVII" => (10, "Maj"),
        "VII" => (11, "Maj"),
        "vii°" => { intervals[1] = 3; intervals[2] = 6; (11, "Dim") },
        _ => (0, "Maj"),
    };

    let mut name = String::from(name);

    // Extensions based on complexity
    let mut final_notes = Vec::new();
    let base_note = root_i32 + root_offset;

    final_notes.push(base_note as u16);
    final_notes.push((base_note + intervals[1]) as u16);
    final_notes.push((base_note + intervals[2]) as u16);

    // 7ths
    if complexity > 0.4 {
        let seventh = match numeral {
            // Dominant 7ths
            "V" | "bII" | "III" | "II" => 10,
            // Major 7ths
            "I" | "IV" | "bVI" | "bIII" => 11,
            // Minor 7ths
            "ii" | "iii" | "vi" | "i" | "iv" | "v" => 10,
            _ => 10,
        };
        final_notes.push((base_note + seventh) as u16);
        name = format!("{}7", name);
    }
    
    // 9ths / 11ths / 13ths
    if complexity > 0.8 {
        final_notes.push((base_note + 14) as u16); // 9th
        name = name.replace("7", "9");
    }

    // Octave confinement to prevent extreme jumps
    for n in final_notes.iter_mut() {
        if *n > (global_root as u16 + 18) { *n -= 12; }
        if *n < (global_root as u16 - 6) { *n += 12; }
    }

    (final_notes, name)
}

fn create_note_human(start: u64, length: u64, note: u16, vel: u32) -> MidiNote {
    let mut rng = rand::thread_rng();
    MidiNote {
        start_sample: start,
        length_samples: length,
        note,
        velocity: vel.clamp(1, 127),
        channel: 0,
        pitch_bend: None,
        pressure: None,
        timbre: None,
        probability: 1.0,
        velocity_random: rng.gen_range(0..10),
        timing_random: rng.gen_range(0..25),
    }
}
