use crate::engine::graph::{MidiClip, MidiNote, MidiCCEvent};
use uuid::Uuid;
use rand::Rng;

pub struct MelodyGeneratorSettings {
    pub genre: String,
    pub bpm: f32,
    pub sample_rate: f64,
    pub num_bars: u64,
    pub root_note: u8,
    pub scale_type: String,
    pub density: f32,
    pub instrument_type: String,
    // --- Upgrades ---
    pub motif_strength: f32, // 0.0=Random, 1.0=Highly repetitive
    pub syncopation: f32, // 0.0=Straight, 1.0=Off-beats
    pub articulation_style: String, // "Legato", "Staccato"
    pub contour: String, // "Ascending", "Arch", "Random"
    pub breathing: f32, // Chance of an empty phrase gap
}

const MAJOR_INTERVALS: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
const MINOR_INTERVALS: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];
const PENTATONIC_MINOR: [u8; 5] = [0, 3, 5, 7, 10];
const HIRAJOSHI: [u8; 5] = [0, 2, 3, 7, 8];

pub fn generate_melody(settings: &MelodyGeneratorSettings) -> MidiClip {
    let mut notes = Vec::new();
    let mut cc_events = Vec::new();
    
    let samples_per_beat = (60.0 / settings.bpm as f64) * settings.sample_rate;
    let samples_per_16th = samples_per_beat / 4.0;
    let total_16ths = settings.num_bars * 16;
    
    let mut rng = rand::thread_rng();

    let intervals = match settings.scale_type.to_lowercase().as_str() {
        "minor" => &MINOR_INTERVALS[..],
        "pentatonic" => &PENTATONIC_MINOR[..],
        "hirajoshi" => &HIRAJOSHI[..],
        _ => &MAJOR_INTERVALS[..],
    };

    let mut current_pos_16th = 0;
    let mut last_scale_degree: i32 = 0;

    // Motif engine
    let mut motif_buffer: Vec<(i32, u64)> = Vec::new(); // (scale_degree, length in 16ths)
    let mut recording_motif = true;
    let mut playback_motif_idx = 0;

    while current_pos_16th < total_16ths {
        // Phrasing / Breath
        if settings.breathing > 0.0 && current_pos_16th % 16 == 0 && rng.gen::<f32>() < settings.breathing {
            // Take a breath for half a bar
            current_pos_16th += 8;
            recording_motif = true; // New phrase, new motif
            motif_buffer.clear();
            continue;
        }

        // Decide Step length based on syncopation and density
        let mut step_len = if settings.syncopation > 0.5 {
            if rng.gen::<f32>() < 0.6 { 3 } else { 2 } // dotted 8ths or 8ths
        } else if rng.gen::<f32>() < settings.density { 2 } else { 4 };

        if current_pos_16th + step_len > total_16ths {
            break;
        }

        let start_sample = (current_pos_16th as f64 * samples_per_16th) as u64;
        let mut length_samples = (step_len as f64 * samples_per_16th) as u64;

        // Articulation style modifiers
        if settings.articulation_style.to_lowercase() == "staccato" || settings.instrument_type.to_lowercase() == "pluck" {
            length_samples = (length_samples as f64 * 0.4) as u64; // Short distinct notes
        } else if settings.articulation_style.to_lowercase() == "legato" {
            length_samples = (length_samples as f64 * 1.05) as u64; // Slight overlap for glide/portamento
        }

        // Tension Curve over the clip (0.0 to 1.0 phase)
        let tension_phase = current_pos_16th as f32 / total_16ths as f32;
        
        let mut degree;

        // Motif processing
        if !recording_motif && !motif_buffer.is_empty() && rng.gen::<f32>() < settings.motif_strength {
            // Playback from motif
            degree = motif_buffer[playback_motif_idx % motif_buffer.len()].0;
            step_len = motif_buffer[playback_motif_idx % motif_buffer.len()].1;
            playback_motif_idx += 1;

            // Simple Motif Transformation: Sequence up
            if tension_phase > 0.5 && rng.gen::<f32>() < settings.motif_strength * 0.5 {
                 degree += 1; // transpose motif by step
            }
        } else {
            // Generate new degree
            let dir_roll = rng.gen_range(0..100);
            
            // Contour checking
            let bias = match settings.contour.to_lowercase().as_str() {
                "ascending" => 20, // bias towards jumping up
                "arch" => if tension_phase < 0.5 { 20 } else { -20 },
                _ => 0, // flat random
            };

            if dir_roll < (40 + bias) { degree = last_scale_degree + 1; }
            else if dir_roll < (80 + bias / 2) { degree = last_scale_degree - 1; }
            else if dir_roll < (90 + bias) { degree = last_scale_degree + 2; }
            else { degree = last_scale_degree - 2; }

            // Add to motif if recording (say we record the first bar's motif)
            if recording_motif {
                motif_buffer.push((degree, step_len));
                if current_pos_16th >= 16 { recording_motif = false; playback_motif_idx = 0; }
            }
        }

        degree = degree.clamp(-7, 14); // 2 octaves

        // Register shift based on tension curve (higher peak in tension)
        if tension_phase > 0.5 && tension_phase < 0.8 && rng.gen::<f32>() < 0.4 {
             degree += intervals.len() as i32; // Octave up
        }

        last_scale_degree = degree;

        let octave = degree / intervals.len() as i32;
        let deg_idx = degree.rem_euclid(intervals.len() as i32) as usize;
        let note_num = settings.root_note as i32 + (octave * 12) + intervals[deg_idx] as i32;

        let final_note = note_num.clamp(0, 127) as u16;

        // Velocity humanization + tension volume swelling
        let base_vel = 80 + (tension_phase * 20.0) as i32; // Sweeping up
        let velocity = (base_vel + rng.gen_range(-15..15)).clamp(1, 127) as u32;

        notes.push(MidiNote {
            start_sample,
            length_samples,
            note: final_note,
            velocity,
            channel: 0,
            pitch_bend: None,
            pressure: None,
            timbre: None,
            probability: 1.0,
            velocity_random: 5,
            timing_random: if settings.syncopation > 0.0 { 15 } else { 0 },
        });

        // Add expression (ModWheel) sweeps
        if (settings.instrument_type == "Synth" || settings.instrument_type == "Pad") && current_pos_16th % 4 == 0 {
            cc_events.push(MidiCCEvent {
                sample: start_sample,
                cc_number: 1,
                value: (40.0 + (tension_phase * 60.0)) as u32,
                channel: 0,
            });
        }

        current_pos_16th += step_len;
    }

    MidiClip {
        id: Uuid::new_v4(),
        name: format!("{} {} Melody", settings.genre, settings.contour),
        start_sample: 0,
        length_samples: (total_16ths as f64 * samples_per_16th) as u64,
        notes,
        cc_events,
        color: "#6C5CE7".into(),
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
