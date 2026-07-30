use serde::{Serialize, Deserialize};
use crate::engine::graph::MidiClipInfo;
use crate::engine::generators::drum_generator::{DrumGeneratorSettings, generate_drums};
use crate::engine::generators::chord_generator::{ChordGeneratorSettings, generate_chords};
use crate::engine::generators::melody_generator::{MelodyGeneratorSettings, generate_melody};
use rand::Rng;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArrangementRegion {
    pub name: String,
    pub start_bar: u64,
    pub length_bars: u64,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeneratedTrackNode {
    pub name: String,
    pub track_type: String, // "Drums", "Chords", "Melody"
    pub clips: Vec<MidiClipInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectArrangement {
    pub regions: Vec<ArrangementRegion>,
    pub tracks: Vec<GeneratedTrackNode>,
    pub tempo: f32,
    pub root_note: u8,
    pub scale_type: String,
}

pub struct ArrangementSettings {
    pub genre: String,
    pub root_note: u8,
    pub scale_type: String,
    pub bpm: f32,
    pub sample_rate: f64,
    pub _length_profile: String, // "Radio Edit", "Extended Mix", "Short Loop"
}

pub fn generate_arrangement(settings: &ArrangementSettings) -> ProjectArrangement {
    let mut rng = rand::thread_rng();

    // 1. Define Song Structure based on profile
    let mut regions = Vec::new();
    let mut current_bar = 0;

    // A common Pop/EDM structure: Intro -> Verse 1 -> Pre-Chorus -> Chorus -> Verse 2 -> Chorus -> Outro
    let structure_plan = match settings.genre.to_lowercase().as_str() {
        "edm" | "house" | "techno" => vec![
            ("Intro", 16, "#34495e"),
            ("Build", 8, "#d35400"),
            ("Drop 1", 16, "#c0392b"),
            ("Breakdown", 16, "#2980b9"),
            ("Build 2", 8, "#d35400"),
            ("Drop 2", 16, "#c0392b"),
            ("Outro", 16, "#7f8c8d"),
        ],
        "hip-hop" | "trap" | "lo-fi" => vec![
            ("Intro", 8, "#8e44ad"),
            ("Chorus", 8, "#27ae60"),
            ("Verse 1", 16, "#16a085"),
            ("Chorus", 8, "#27ae60"),
            ("Verse 2", 16, "#16a085"),
            ("Chorus", 8, "#27ae60"),
            ("Outro", 8, "#2c3e50"),
        ],
        _ => vec![
            ("Intro", 8, "#95a5a6"),
            ("Verse 1", 16, "#2980b9"),
            ("Pre-Chorus", 8, "#8e44ad"),
            ("Chorus", 16, "#f39c12"),
            ("Verse 2", 16, "#2980b9"),
            ("Chorus", 16, "#f39c12"),
            ("Outro", 8, "#bdc3c7"),
        ]
    };

    for (name, length, color) in structure_plan {
        regions.push(ArrangementRegion {
            name: name.to_string(),
            start_bar: current_bar,
            length_bars: length,
            color: color.to_string(),
        });
        current_bar += length;
    }

    let samples_per_beat = (60.0 / settings.bpm as f64) * settings.sample_rate;
    let samples_per_bar = samples_per_beat * 4.0;

    let mut drum_clips = Vec::new();
    let mut chord_clips = Vec::new();
    let mut melody_clips = Vec::new();
    
    // Choose global progressions to keep it cohesive
    let progression_preset = match settings.genre.to_lowercase().as_str() {
         "edm" | "house" | "techno" => "EDM",
         "hip-hop" | "lo-fi" => "Jazzy Loop",
         _ => "Pop"
    };

    // 2. Orchestrate generators per region
    for region in &regions {
        // Density and intensity mappings based on section name
        let (density, motif_strength, fill_freq, complexity) = match region.name.as_str() {
            "Intro" | "Outro" => (0.3, 0.9, 0, 0.2), // Sparse, repetitive, simple
            "Verse 1" | "Verse 2" | "Breakdown" => (0.5, 0.7, 8, 0.4), // Medium density
            "Pre-Chorus" | "Build" | "Build 2" => (0.7, 0.5, 4, 0.6), // Building tension
            "Chorus" | "Drop 1" | "Drop 2" => (0.9, 0.8, 8, 0.8), // Max energy, strong motifs, complex chords
            _ => (0.6, 0.6, 8, 0.5)
        };

        let start_sample = (region.start_bar as f64 * samples_per_bar) as u64;

        // Generate Drums
        // Avoid drums in Intro/Breakdown sometimes for effect
        if region.name != "Breakdown" && !(region.name == "Intro" && rng.gen::<f32>() < 0.5) {
             let drum_settings = DrumGeneratorSettings {
                genre: settings.genre.clone(),
                bpm: settings.bpm,
                sample_rate: settings.sample_rate,
                num_bars: region.length_bars,
                density,
                swing: if settings.genre == "lo-fi" { 0.6 } else { 0.1 },
                humanization: 0.5,
                groove_archetype: if settings.genre.to_lowercase().contains("house") { "Straight".into() } else { "Funky".into() },
                interplay: 0.5,
                fill_frequency: fill_freq,
                micro_layering: density > 0.7,
            };
            
            let mut clip = generate_drums(&drum_settings);
            clip.start_sample = start_sample; // Offset the clip to the region start!
            clip.name = format!("{} Drums", region.name);
            clip.color = region.color.clone();
            drum_clips.push(to_dto(clip));
        }

        let chord_settings = ChordGeneratorSettings {
            genre: settings.genre.clone(),
            bpm: settings.bpm,
            sample_rate: settings.sample_rate,
            root_note: settings.root_note,
            complexity,
            progression_preset: progression_preset.into(),
            voicing_style: if density < 0.5 { "Pad Cluster".into() } else { "Piano Wide".into() },
            rhythm_complexity: if density > 0.6 { 0.4 } else { 0.0 }, // somewhat rhythmic in chorus
            substitutions: complexity * 0.5,
        };

        // We generate exactly the region length (in loops)
        // Since generator uses predefined 4 or 8 bar arrays, it wraps around. 
        // We will just let chord generator produce the whole length or loop it. Modifying chord generator to accept num_bars would be better, but we can just use length_samples in graph.
        let mut chord_clip = generate_chords(&chord_settings);
        
        // Loop out the chords to fill the region
        chord_clip.start_sample = start_sample;
        chord_clip.length_samples = (region.length_bars as f64 * samples_per_bar) as u64;
        chord_clip.name = format!("{} Chords", region.name);
        chord_clip.color = region.color.clone();
        chord_clips.push(to_dto(chord_clip));

        // Generate Melody
        if region.name != "Intro" && region.name != "Build 2" {
            let melody_settings = MelodyGeneratorSettings {
                genre: settings.genre.clone(),
                bpm: settings.bpm,
                sample_rate: settings.sample_rate,
                num_bars: region.length_bars,
                root_note: settings.root_note,
                scale_type: settings.scale_type.clone(),
                density: density * 0.8, // Slightly less dense than drums
                instrument_type: if density > 0.7 { "Synth".into() } else { "Piano".into() },
                motif_strength,
                syncopation: 0.3,
                articulation_style: if density > 0.7 { "Legato".into() } else { "Staccato".into() },
                contour: if region.name.contains("Build") { "Ascending".into() } else { "Arch".into() },
                breathing: 1.0 - density, // More breath when less dense
            };

            let mut mel_clip = generate_melody(&melody_settings);
            mel_clip.start_sample = start_sample;
            mel_clip.name = format!("{} Melody", region.name);
            mel_clip.color = region.color.clone();
            melody_clips.push(to_dto(mel_clip));
        }
    }

    let mut tracks = Vec::new();
    if !drum_clips.is_empty() {
        tracks.push(GeneratedTrackNode { name: "Drums".into(), track_type: "Drums".into(), clips: drum_clips });
    }
    if !chord_clips.is_empty() {
        tracks.push(GeneratedTrackNode { name: "Chords".into(), track_type: "Chords".into(), clips: chord_clips });
    }
    if !melody_clips.is_empty() {
        tracks.push(GeneratedTrackNode { name: "Melody".into(), track_type: "Melody".into(), clips: melody_clips });
    }

    ProjectArrangement {
        regions,
        tracks,
        tempo: settings.bpm,
        root_note: settings.root_note,
        scale_type: settings.scale_type.clone(),
    }
}

fn to_dto(clip: crate::engine::graph::MidiClip) -> MidiClipInfo {
     MidiClipInfo {
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
    }
}
