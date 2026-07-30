#![allow(dead_code)]
use super::graph::{Track, TrackType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TrackRole {
    Drums,
    Bass,
    Vocals,
    Lead,
    Pad,
    Synth,
    FX,
    Ambience,
    Unknown,
}

pub struct Gosposia;

impl Gosposia {
    pub fn detect_role(track: &Track) -> TrackRole {
        let name = track.name.to_lowercase();

        // 1. Name-based detection (most common)
        if name.contains("kick")
            || name.contains("snare")
            || name.contains("drums")
            || name.contains("perc")
            || name.contains("hat")
        {
            return TrackRole::Drums;
        }
        if name.contains("bass") || name.contains("sub") {
            return TrackRole::Bass;
        }
        if name.contains("vox") || name.contains("vocal") || name.contains("lead vox") {
            return TrackRole::Vocals;
        }
        if name.contains("lead") {
            return TrackRole::Lead;
        }
        if name.contains("pad") || name.contains("atmosphere") {
            return TrackRole::Pad;
        }
        if name.contains("synth") || name.contains("arp") {
            return TrackRole::Synth;
        }
        if name.contains("fx") || name.contains("riser") || name.contains("impact") {
            return TrackRole::FX;
        }
        if name.contains("ambience") || name.contains("texture") || name.contains("noise") {
            return TrackRole::Ambience;
        }

        // 2. Content-based detection (fallback)
        if track.track_type == TrackType::MIDI || track.track_type == TrackType::Instrument {
            // Check processors for clues
            for proc in &track.processors {
                let p_name = proc.name().to_lowercase();
                if p_name.contains("drum") || p_name.contains("kontakt") && name.contains("drum") {
                    return TrackRole::Drums;
                }
            }
        }

        TrackRole::Unknown
    }

    pub fn get_role_color(role: &TrackRole) -> &'static str {
        match role {
            TrackRole::Drums => "#E74C3C",    // Red
            TrackRole::Bass => "#3498DB",     // Blue
            TrackRole::Vocals => "#F1C40F",   // Yellow
            TrackRole::Lead => "#9B59B6",     // Purple
            TrackRole::Pad => "#1ABC9C",      // Turquoise
            TrackRole::Synth => "#E67E22",    // Orange
            TrackRole::FX => "#95A5A6",       // Gray
            TrackRole::Ambience => "#2ECC71", // Green
            TrackRole::Unknown => "#4a9eff",  // Default VIBE Blue
        }
    }

    pub fn get_role_folder_name(role: &TrackRole) -> &'static str {
        match role {
            TrackRole::Drums => "DRUMS",
            TrackRole::Bass => "BASS",
            TrackRole::Vocals => "VOCALS",
            TrackRole::Lead => "LEADS",
            TrackRole::Pad => "PADS",
            TrackRole::Synth => "SYNTHS",
            TrackRole::FX => "FX",
            TrackRole::Ambience => "AMBIENCE",
            TrackRole::Unknown => "OTHER",
        }
    }

    pub fn is_dead_track(track: &Track) -> bool {
        // A track is "dead" if it has no clips, no midi clips, no processors, and is a default name
        let is_default_name = track.name.starts_with("Audio ")
            || track.name.starts_with("Midi ")
            || track.name.starts_with("Instrument ");
        track.clips.is_empty()
            && track.midi_clips.is_empty()
            && track.processors.is_empty()
            && is_default_name
    }

    pub fn suggest_folders(tracks: &[Track]) -> Vec<(TrackRole, Vec<Uuid>)> {
        let mut groups: std::collections::HashMap<TrackRole, Vec<Uuid>> =
            std::collections::HashMap::new();

        for track in tracks {
            if track.track_type == TrackType::Folder || track.track_type == TrackType::Group {
                continue;
            }
            if track.parent_id.is_some() {
                continue; // Already in a folder
            }

            let role = Self::detect_role(track);
            if role != TrackRole::Unknown {
                groups.entry(role).or_default().push(track.id);
            }
        }

        // Only suggest if there are at least 2 tracks for a role
        groups
            .into_iter()
            .filter(|(_, ids)| ids.len() >= 2)
            .collect()
    }

    pub fn suggest_clip_tidy(track: &Track) -> Option<Uuid> {
        for clip in &track.clips {
            if clip.length_in_samples < 2400 {
                // < 50ms at 48k
                return Some(clip.id);
            }
        }
        for clip in &track.midi_clips {
            if clip.length_samples < 2400 {
                return Some(clip.id);
            }
        }
        None
    }

    pub fn autolabel_sections(max_pos: u64, density: &[u32]) -> Vec<(u64, String)> {
        let mut sections = Vec::new();
        const BLOCK_SIZE: u64 = 48000 * 8;

        if density.len() < 2 {
            return sections;
        }

        sections.push((0, "Intro".to_string()));

        for i in 1..density.len() - 1 {
            let pos = i as u64 * BLOCK_SIZE;
            if pos > max_pos {
                break;
            }

            // Heuristic for "Drop" (large rise in density)
            if density[i] > density[i - 1] * 3 && density[i] > 4 {
                sections.push((pos, "Drop / Chorus".to_string()));
            }
            // Heuristic for "Breakdown" (large drop in density)
            else if density[i] < density[i - 1] / 3 && density[i - 1] > 4 {
                sections.push((pos, "Breakdown".to_string()));
            }
        }

        if max_pos > BLOCK_SIZE * 2 {
            sections.push((max_pos - BLOCK_SIZE, "Outro".to_string()));
        }

        sections
    }

    pub fn suggest_automation_cleanup(track: &Track) -> Option<Uuid> {
        // Find parameters with flat automation (all knots same value) or redundant knots
        for param in track.get_all_parameters_ref() {
            let curve = param.curve.load();
            if curve.knots.len() > 5 {
                let first_val = curve.knots[0].value;
                let is_flat = curve
                    .knots
                    .iter()
                    .all(|k| (k.value - first_val).abs() < 0.0001);
                if is_flat {
                    return Some(param.id);
                }
            }
        }
        None
    }

    pub fn suggest_plugin_dusting(track: &Track) -> Option<(Uuid, String)> {
        for proc in &track.processors {
            if proc.is_bypassed() {
                // In a real app we'd track timestamp of bypass, here we simulate detection
                return Some((proc.id(), proc.name()));
            }
        }
        None
    }

    pub fn suggest_sample_cleanup(tracks: &[Track]) -> Option<String> {
        let mut paths = std::collections::HashSet::new();
        let mut duplicates = Vec::new();
        for track in tracks {
            for clip in &track.clips {
                if let Some(path) = &clip.path {
                    if !paths.insert(path.clone()) {
                        duplicates.push(clip.name.clone());
                    }
                }
            }
        }
        if !duplicates.is_empty() {
            return Some(duplicates[0].clone());
        }
        None
    }
}
