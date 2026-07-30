use crate::engine::graph::{MidiClip, MidiNote};
use uuid::Uuid;
use rand::Rng;

pub struct DrumGeneratorSettings {
    pub genre: String,
    pub bpm: f32,
    pub sample_rate: f64,
    pub num_bars: u64,
    pub density: f32, // 0.0 to 1.0 (how busy)
    pub swing: f32,   // 0.0 to 1.0
    // --- Upgrades ---
    pub humanization: f32, // 0.0 to 1.0
    pub groove_archetype: String, // "Straight", "Funky", "Half-Time", "Broken"
    pub fill_frequency: u8, // Bars per fill, 0 = none, e.g., 4 or 8
    pub interplay: f32, // 0.0 to 1.0 Call & response intensity
    pub micro_layering: bool, // E.g., add layer under kick
}

// Standard GM Drum Map
const KICK: u16 = 36;
const SUB_KICK: u16 = 35; // Used for layering
const SNARE: u16 = 38;
const RIM: u16 = 37;
const CLAP: u16 = 39;
const CHAT: u16 = 42; // Closed Hat
const OHAT: u16 = 46; // Open Hat
const CRASH: u16 = 49;
const TOM_HI: u16 = 50;
const TOM_MID: u16 = 47;
const TOM_LOW: u16 = 43;

pub fn generate_drums(settings: &DrumGeneratorSettings) -> MidiClip {
    let mut notes = Vec::new();
    let samples_per_beat = (60.0 / settings.bpm as f64) * settings.sample_rate;
    let samples_per_16th = samples_per_beat / 4.0;
    
    let total_16ths = settings.num_bars * 16;
    let mut rng = rand::thread_rng();

    // Loop through 16ths
    for i in 0..total_16ths {
        // Base timing with swing
        let start_sample = add_swing(i, samples_per_16th, settings.swing);
        
        let is_fill_bar = settings.fill_frequency > 0 && 
            (i / 16) % (settings.fill_frequency as u64) == (settings.fill_frequency as u64 - 1);
        
        let is_fill_zone = is_fill_bar && (i % 16 >= 8); // Last half of the fill bar
        
        // Archetype modifiers
        let mut is_kick = false;
        let mut is_snare = false;
        let mut is_hat = false;
        let mut hat_vel = 0;
        let mut hat_open = false;

        match settings.groove_archetype.to_lowercase().as_str() {
            "funky" => {
                // Ghost notes and syncopation
                if i % 8 == 0 || (i % 16 == 5 && rng.gen::<f32>() < settings.density) { is_kick = true; }
                if i % 16 == 4 || i % 16 == 12 { is_snare = true; } // Standard 2 & 4
                // Off-beat and 16th hats
                if i % 2 == 0 { 
                    is_hat = true; 
                    hat_vel = if i % 4 == 0 { rng.gen_range(80..100) } else { rng.gen_range(50..70) };
                } else if rng.gen::<f32>() < settings.density * 0.8 {
                    is_hat = true;
                    hat_vel = rng.gen_range(30..50); // Ghost hat
                }
            },
            "half-time" => {
                // Snare on beat 3
                if i % 16 == 0 || (i % 16 == 10 && rng.gen::<f32>() < settings.density) { is_kick = true; }
                if i % 16 == 8 { is_snare = true; }
                if i % 2 == 0 { is_hat = true; hat_vel = rng.gen_range(70..100); }
            },
            "broken" => { // UK Garage / Breakbeat feel
                if i % 16 == 0 || (i % 16 == 7 && rng.gen::<f32>() < settings.density) || (i % 16 == 10 && rng.gen::<f32>() < settings.density) { is_kick = true; }
                if i % 16 == 4 || i % 16 == 12 { is_snare = true; }
                
                // Shuffle hats
                if i % 4 == 2 { is_hat = true; hat_open = true; hat_vel = 100; }
                if i % 4 == 1 || i % 4 == 3 { is_hat = true; hat_vel = rng.gen_range(40..60); }
            },
            _ => { // "Straight"
                if i % 4 == 0 { is_kick = true; }
                if i % 16 == 4 || i % 16 == 12 { is_snare = true; }
                if i % 2 == 0 { is_hat = true; hat_vel = if i % 4 == 0 { 100 } else { 70 }; }
            }
        }

        // Call & Response Interplay
        // If interplay is high, hats get busy when kick is NOT playing
        if !is_kick && settings.interplay > 0.5 && rng.gen::<f32>() < (settings.interplay * settings.density * 0.5) {
            is_hat = true;
            hat_vel = rng.gen_range(40..80);
        }

        // Fill Generation Override
        if is_fill_zone {
            is_kick = i % 2 == 0 && rng.gen::<f32>() < 0.5;
            is_snare = i % 2 != 0 && rng.gen::<f32>() < settings.density;
            is_hat = false; // Drop hats in fill
            
            // Add Toms
            if rng.gen::<f32>() < 0.3 {
                let tom = if i % 16 < 12 { TOM_HI } else if i % 16 < 14 { TOM_MID } else { TOM_LOW };
                notes.push(create_note_human(start_sample, samples_per_16th as u64, tom, rng.gen_range(80..120), settings.humanization, 0.5));
            }
        }

        // Note Construction with Humanization

        if is_kick {
            // Kick timing is tight (low humanization weight 0.2)
            let vel = if is_fill_zone { rng.gen_range(90..110) } else { rng.gen_range(110..127) };
            notes.push(create_note_human(start_sample, samples_per_16th as u64, KICK, vel, settings.humanization, 0.2));
            
            if settings.micro_layering {
                notes.push(create_note_human(start_sample, samples_per_16th as u64, SUB_KICK, vel - 10, 0.0, 0.0));
            }
        }

        if is_snare {
            // Snare uses different mapping based on genre/archetype
            let snare_note = if settings.genre.to_lowercase() == "lo-fi" && !is_fill_zone { RIM } 
                            else if settings.genre.to_lowercase() == "house" { CLAP } 
                            else { SNARE };

            let vel = if is_fill_zone { rng.gen_range(80..115) } else { rng.gen_range(100..120) };
            notes.push(create_note_human(start_sample, samples_per_16th as u64, snare_note, vel, settings.humanization, 0.4));
        }

        if is_hat {
            let note = if hat_open { OHAT } else { CHAT };
            notes.push(create_note_human(start_sample, (samples_per_16th * 0.5) as u64, note, hat_vel, settings.humanization, 1.0));
        }

        // Crash on beat 1 of measure 1, or after a fill
        if (i == 0) || (is_fill_bar && i == 15 && rng.gen::<f32>() < 0.8) {
             let hit_i = if i == 15 { i + 1 } else { i }; // Next downbeat after fill
             if hit_i < total_16ths {
                 let crash_start = add_swing(hit_i, samples_per_16th, settings.swing);
                 notes.push(create_note_human(crash_start, (samples_per_16th * 8.0) as u64, CRASH, 110, settings.humanization, 0.3));
             }
        }
    }

    MidiClip {
        id: Uuid::new_v4(),
        name: format!("{} {} Drums", settings.genre, settings.groove_archetype),
        start_sample: 0,
        length_samples: (total_16ths as f64 * samples_per_16th) as u64,
        notes,
        cc_events: vec![],
        color: "#E25488".into(),
        is_muted: false,
        is_looped: true,
        scale: None,
        chord_markers: vec![],
        groove_template: None,
        pattern_id: None,
        tuning_steps: None,
        time_signature_num: Some(4),
        time_signature_den: Some(4),
        reference_clip_id: None,
    }
}

/// Utility to create a note with style-aware humanization
fn create_note_human(base_start: u64, length: u64, note: u16, base_vel: u32, humanize_amt: f32, element_weight: f32) -> MidiNote {
    let mut rng = rand::thread_rng();
    
    // Timing Humanization (element_weight makes hats sloppier than kicks)
    let max_shift = 44100.0 * 0.02 * humanize_amt * element_weight; // max 20ms shift
    let shift = rng.gen_range(-max_shift..max_shift) as i64;
    let final_start = (base_start as i64 + shift).max(0) as u64;

    // Velocity Humanization
    let vel_var = (15.0 * humanize_amt * element_weight) as i32;
    let shift_vel = rng.gen_range(-vel_var..vel_var);
    let final_vel = (base_vel as i32 + shift_vel).clamp(1, 127) as u32;

    MidiNote {
        start_sample: final_start,
        length_samples: length,
        note,
        velocity: final_vel,
        channel: 0,
        pitch_bend: None,
        pressure: None,
        timbre: None,
        probability: 1.0,
        velocity_random: 0, // Handled internally now
        timing_random: 0,
    }
}

// Utility to apply swing dynamically during generation
fn add_swing(step_16th: u64, samples_per_16th: f64, swing_amount: f32) -> u64 {
    let mut offset = 0.0;
    if !step_16th.is_multiple_of(2) {
        offset = samples_per_16th * (swing_amount as f64 * 0.5);
    }
    ((step_16th as f64 * samples_per_16th) + offset) as u64
}
