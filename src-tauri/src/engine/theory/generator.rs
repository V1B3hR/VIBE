#![allow(dead_code)]
use super::{Chord, Key};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::engine::graph::{MidiClip, MidiNote};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressionTemplate {
    pub name: String,
    pub degrees: Vec<String>, // e.g., ["I", "V", "vi", "IV"]
    pub vibe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequenceCycle {
    Fifths,     // Circle of 5ths (Descending 4ths) -> V -> I
    Fourths,    // Circle of 4ths (Ascending 4ths) -> IV -> I
    Thirds,     // Chromatic Mediant / Cycle of 3rds -> I -> vi -> IV, etc.
    Sevenths,   // Sequential 7ths (falling 5th with 7ths)
    Chromatic,  // Modal Interchange / Chromatic stepwise -> I -> bII -> ii -> bIII
}

pub struct Generator;

impl Generator {
    /// Helper: Find the root index of a note name
    fn get_root_idx(root_name: &str) -> usize {
        let roots = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        roots.iter().position(|&r| r == root_name).unwrap_or(0)
    }

    /// Helper: Get Note string from index
    fn get_root_str(idx: usize) -> String {
        let roots = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        roots[idx % 12].to_string()
    }

    /// ZAAWANSOWANE: Generowanie Progresji Akordów opartych na Cyklach Harmonicznych (The Circles)
    pub fn generate_cycle_progression(key: &Key, cycle: SequenceCycle, length: usize) -> Vec<Chord> {
        let mut progression = Vec::new();
        let mut current_root_idx = Self::get_root_idx(&key.root);
        
        for i in 0..length {
            let quality = match cycle {
                SequenceCycle::Fifths | SequenceCycle::Fourths => {
                    // For pure dominant chains, use dominant 7ths, otherwise resolving to Maj7/Min7.
                    if i == length - 1 { "Maj7" } else { "7" }
                },
                SequenceCycle::Thirds => {
                    // Chromatic mediants - alternative between Maj and Min
                    if i % 2 == 0 { "Maj7" } else { "min7" }
                },
                SequenceCycle::Sevenths => {
                    // Diatonic 7ths waterfall
                    if i % 3 == 0 { "Maj7" } else if i % 3 == 1 { "min7" } else { "m7b5" }
                },
                SequenceCycle::Chromatic => {
                    // Constant structure passing chords
                    if i % 2 == 0 { "min7" } else { "Dim" } // Tension -> resolution -> tension
                }
            };
            
            progression.push(Chord::new(current_root_idx as u8, quality));
            
            // Advance to the next root in the circle
            current_root_idx = match cycle {
                SequenceCycle::Fifths => (current_root_idx + 7) % 12,      // Circle of 5ths (+7 semitones)
                SequenceCycle::Fourths => (current_root_idx + 5) % 12,     // Circle of 4ths (+5 semitones)
                SequenceCycle::Thirds => (current_root_idx + 4) % 12,      // Major 3rd cycle (+4 semitones)
                SequenceCycle::Sevenths => (current_root_idx + 10) % 12,   // Moving by Minor 7ths / Whole step down
                SequenceCycle::Chromatic => (current_root_idx + 1) % 12,   // Chromatic ascent
            };
        }
        
        progression
    }

    /// Funkcja dziedziczona z The One Plan: Generowanie Progresji opartych na interwałach w skali
    pub fn generate_progression(key: &Key, template: &ProgressionTemplate) -> Vec<Chord> {
        let root_idx = Self::get_root_idx(&key.root);

        let scale_intervals = if key.scale_type == "Major" {
            vec![0, 2, 4, 5, 7, 9, 11]
        } else {
            vec![0, 2, 3, 5, 7, 8, 10] // Natural Minor
        };

        let mut progression = Vec::new();

        for degree in &template.degrees {
            let (interval_idx, quality) = match degree.as_str() {
                // Major Key
                "I" => (0, "Maj"), "ii" => (1, "Min"), "iii" => (2, "Min"),
                "IV" => (3, "Maj"), "V" => (4, "Maj"), "vi" => (5, "Min"), "vii" => (6, "Dim"),
                
                // Minor Key
                "i" => (0, "Min"), "ii°" => (1, "Dim"), "III" => (2, "Maj"),
                "iv" => (3, "Min"), "v" => (4, "Min"), "VI" => (5, "Maj"), "VII" => (6, "Maj"),

                // Advanced Extensions (7ths, 9ths, secondary dominants)
                "Imaj7" => (0, "Maj7"), "ii7" => (1, "min7"), "V7" => (4, "7"),
                "IVmaj7" => (3, "Maj7"), "vi7" => (5, "min7"), "III7" => (2, "7"), // Secondary dominant targeting vi
                "V/V" => (1, "7"), // Secondary dominant of V (II7 -> V)

                _ => (0, "Maj"), // Fallback
            };

            let note_idx = (root_idx + scale_intervals[interval_idx % 7] as usize) % 12;
            progression.push(Chord::new(note_idx as u8, quality));
        }

        progression
    }

    /// Inteligentna Sugestia Harmony (LEVEL 4 Co-Pilot) z uwzględnieniem koła kwint i passing chords
    pub fn suggest_next_chord(key: &Key, current_progression: &[Chord]) -> Chord {
        let root_idx = Self::get_root_idx(&key.root);
        let last_chord = current_progression.last();

        let degree = if let Some(last) = last_chord {
            let interval = (last.root as i32 - root_idx as i32 + 12) % 12;
            match interval {
                0 => 7, // I -> V (Dominant)
                7 => 0, // V -> I (Resolution)
                5 => 7, // IV -> V (Preparation)
                9 => 5, // vi -> IV
                2 => 7, // ii -> V (Jazz 2-5-1)
                _ => (interval + 5) % 12, // Default to resolving backward by a fourth (Circle of Fifths logic)
            }
        } else {
            0 // Start with I
        };

        let note_idx = (root_idx + degree as usize) % 12;
        let quality = if key.scale_type == "Major" {
            match degree {
                0 | 5 => "Maj7",
                7 => "7", // V dominant 7
                2 | 4 | 9 => "min7",
                11 => "m7b5",
                _ => "Maj",
            }
        } else {
            match degree {
                3 | 5 | 10 => "Maj7",
                0 | 2 | 7 => "min7",
                1 => "m7b5",
                _ => "Min",
            }
        };

        Chord::new(note_idx as u8, quality)
    }

    /// Oparta na AI Generacja Linii Melodycznej używająca skali, chromatyki i passings notes
    pub fn generate_melody(chords: &[Chord], steps_per_chord: u32) -> Vec<u8> {
        let mut melody = Vec::new();
        let mut rng = rand::thread_rng();

        for chord in chords {
            let tones = chord.get_intervals(); // relative to 0
            let root = chord.root;

            for step in 0..steps_per_chord {
                // Determine if we want a chord tone, or a chromatic passing tone
                let is_passing_tone = rng.gen_bool(0.2) && step % 2 != 0; // Only on off-beats
                
                if is_passing_tone {
                    // Pick a note 1 semitone away from a target chord tone (Chromatic approach)
                    let target = tones.choose(&mut rng).unwrap_or(&0);
                    let direction = if rng.gen_bool(0.5) { 1 } else { -1 };
                    let note = ((root as i32 + *target as i32 + direction + 12) % 12) as u8;
                    melody.push(60 + note); // Octave 4
                } else {
                    // Pure chord tone
                    if let Some(&tone) = tones.choose(&mut rng) {
                        let note = (root + tone) % 12;
                        melody.push(60 + note); // Octave 4
                    }
                }
            }
        }
        melody
    }

    /// Groove Genetix: Generowanie Perkusji z Fills'ami (Przejściami) i swingiem
    pub fn generate_drums(
        style: &str,
        length_bars: u32,
        sample_rate: f64,
        bpm: f64,
        add_fill_at_end: bool,
    ) -> MidiClip {
        let mut notes = Vec::new();
        let bar_samples = (60.0 / bpm * 4.0 * sample_rate) as u64;
        let step_samples = bar_samples / 16;
        let mut rng = rand::thread_rng();

        for bar in 0..length_bars {
            let bar_offset = bar as u64 * bar_samples;
            let is_last_bar = bar == length_bars - 1;
            
            if is_last_bar && add_fill_at_end {
                // >>> ZAAWANSOWANY DRUM FILL <<<
                // Kick on 1
                notes.push(MidiNote {
                    start_sample: bar_offset,
                    length_samples: step_samples / 2,
                    note: 36, velocity: 110, channel: 9, ..Default::default()
                });
                
                // Toms barrage from step 8 to 15 (16th notes or triplets)
                let toms = [50, 48, 47, 45, 43, 41]; // High to Low Toms
                for step in 8..16 {
                    let tom_idx = ((step - 8) as f32 / 8.0 * (toms.len() - 1) as f32).round() as usize;
                    notes.push(MidiNote {
                        start_sample: bar_offset + step * step_samples,
                        length_samples: step_samples / 2,
                        note: toms[tom_idx], // Descending tom roll
                        velocity: 80 + (step as u32 * 2), // Crescendo
                        channel: 9, ..Default::default()
                    });
                }
                
                // Crash on the start of the next beat (implicit, handled ideally by next loop, but we can add hihat pedal here)
                continue;
            }

            // --- STANDARD GROOVE ---
            match style {
                "techno" | "house" => {
                    // Four on the floor + Humanization
                    for &step in &[0, 4, 8, 12] {
                        let micro_shift = (rng.gen_range(-0.02..0.02) * step_samples as f64) as i64;
                        let vel_shift = rng.gen_range(-5..5);
                        notes.push(MidiNote {
                            start_sample: (bar_offset as i64 + step as i64 * step_samples as i64 + micro_shift).max(0) as u64,
                            length_samples: step_samples / 2,
                            note: 36, velocity: (100_i32 + vel_shift).max(1).min(127) as u32, channel: 9, ..Default::default()
                        });
                    }
                    // Off-beat hats
                    for &step in &[2, 6, 10, 14] {
                        let micro_shift = (rng.gen_range(-0.04..0.05) * step_samples as f64) as i64; // More loose hats
                        let vel_shift = rng.gen_range(-15..5);
                        notes.push(MidiNote {
                            start_sample: (bar_offset as i64 + step as i64 * step_samples as i64 + micro_shift).max(0) as u64,
                            length_samples: step_samples / 4,
                            note: 46, velocity: (90_i32 + vel_shift).max(1).min(127) as u32, channel: 9, ..Default::default()
                        });
                    }
                    // Clap/Snare on 4 and 12
                    for &step in &[4, 12] {
                        let micro_shift = (rng.gen_range(-0.01..0.03) * step_samples as f64) as i64; // Slight rush or drag
                        notes.push(MidiNote {
                            start_sample: (bar_offset as i64 + step as i64 * step_samples as i64 + micro_shift).max(0) as u64,
                            length_samples: step_samples / 2,
                            note: 39, velocity: 105, channel: 9, ..Default::default()
                        });
                    }
                }
                "rock" | _ => {
                    // Kick on 1 and 3
                    for &step in &[0, 8] {
                        let micro_shift = (rng.gen_range(-0.02..0.02) * step_samples as f64) as i64;
                        notes.push(MidiNote {
                            start_sample: (bar_offset as i64 + step as i64 * step_samples as i64 + micro_shift).max(0) as u64,
                            length_samples: step_samples / 2,
                            note: 36, velocity: rng.gen_range(95..105), channel: 9, ..Default::default()
                        });
                    }
                    // Snare on 2 and 4
                    for &step in &[4, 12] {
                        let micro_shift = (rng.gen_range(-0.02..0.04) * step_samples as f64) as i64; // Snare often dragged in rock
                        let vel_shift = rng.gen_range(-5..5);
                        notes.push(MidiNote {
                            start_sample: (bar_offset as i64 + step as i64 * step_samples as i64 + micro_shift).max(0) as u64,
                            length_samples: step_samples / 2,
                            note: 38, velocity: (110_i32 + vel_shift).max(1).min(127) as u32, channel: 9, ..Default::default()
                        });
                    }
                    // 8th note hats
                    for step in 0..8 {
                        let step_pos = step * 2;
                        let micro_shift = (rng.gen_range(-0.03..0.03) * step_samples as f64) as i64;
                        let vel = if step % 2 == 0 { 95 } else { 75 }; // Accent strong beats
                        notes.push(MidiNote {
                            start_sample: (bar_offset as i64 + step_pos as i64 * step_samples as i64 + micro_shift).max(0) as u64,
                            length_samples: step_samples / 4,
                            note: 42, velocity: vel, channel: 9, ..Default::default()
                        });
                    }
                }
            }
        }

        MidiClip {
            id: Uuid::new_v4(),
            name: format!("{} Drums {}", style, if add_fill_at_end { "w/Fill" } else { "" }),
            start_sample: 0,
            length_samples: length_bars as u64 * bar_samples,
            notes,
            cc_events: Vec::new(),
            color: "#ffcc00".to_string(),
            is_muted: false,
            is_looped: true,
            scale: None,
            chord_markers: Vec::new(),
            groove_template: None,
            pattern_id: None,
            tuning_steps: Some(12),
            time_signature_num: Some(4),
            time_signature_den: Some(4),
            reference_clip_id: None,
        }
    }
}
