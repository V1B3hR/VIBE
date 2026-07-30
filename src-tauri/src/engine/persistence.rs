use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Magic bytes for VIBE project files: "V1B3" (VIBE)
const VIBE_MAGIC: [u8; 4] = *b"V1B3";

/// Current file format version
const VIBE_VERSION: u32 = 2; // V2: MIDI Sequencer support

/// File header for .vibe binary format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeFileHeader {
    /// Magic bytes: "V1B3"
    pub magic: [u8; 4],
    /// Format version (for backward compatibility)
    pub version: u32,
    /// CRC32 checksum of the data section
    pub checksum: u32,
    /// Offset to data section (after header)
    pub data_offset: u64,
    /// Size of data section in bytes
    pub data_size: u64,
}

impl VibeFileHeader {
    pub fn new(data_size: u64, checksum: u32) -> Self {
        Self {
            magic: VIBE_MAGIC,
            version: VIBE_VERSION,
            checksum,
            data_offset: std::mem::size_of::<VibeFileHeader>() as u64,
            data_size,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        const MAX_PROJECT_DATA_SIZE: u64 = 524_288_000; // 500MB security cap

        if self.magic != VIBE_MAGIC {
            return Err("Invalid VIBE file: magic bytes mismatch".to_string());
        }
        if self.version > VIBE_VERSION {
            return Err(format!(
                "Unsupported VIBE version: {} (current: {})",
                self.version, VIBE_VERSION
            ));
        }
        if self.data_size > MAX_PROJECT_DATA_SIZE {
            return Err(format!(
                "Security Exception: Project data size exceeds maximum threshold ({} bytes > 500MB)",
                self.data_size
            ));
        }
        Ok(())
    }
}

/// Complete project snapshot for binary serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    /// Project name
    pub name: String,
    /// BPM (tempo)
    pub bpm: f64,
    /// Sample rate
    pub sample_rate: f64,
    /// Track data
    pub tracks: Vec<TrackSnapshot>,
    /// Master volume (f64 for bit-perfect precision)
    pub master_volume: f64,
    /// Master pan
    pub master_pan: f64,
    /// Input aliases (Phase 1.3: Hardware I/O routing)
    pub input_aliases: Vec<super::io_manager::InputAlias>,
    /// Global MIDI mappings (Phase 4.1)
    pub midi_bindings: Vec<super::midi_mapping::MidiBinding>,
    /// Loop settings
    pub loop_enabled: bool,
    pub loop_start: u64,
    pub loop_end: u64,
    /// VCA Groups (Phase 5: Advanced Mixing)
    pub vca_groups: Vec<VcaGroupSnapshot>,
}

/// Parameter snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSnapshot {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub automation: Vec<crate::engine::automation::AutomationKnot>,
}

/// Individual track snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSnapshot {
    /// Track UUID
    pub id: String,
    /// Track name
    pub name: String,
    /// Volume fader
    pub volume: ParameterSnapshot,
    /// Pan position
    pub pan: ParameterSnapshot,
    /// Stereo width
    pub width: ParameterSnapshot,
    /// Input drive (saturation)
    pub input_drive: ParameterSnapshot,
    /// Mute state
    pub muted: bool,
    /// Solo state
    pub solo: bool,
    /// Record arm state
    pub is_armed: bool,
    /// Phase invert state
    pub phase_inverted: bool,
    /// Track color
    pub color: String,
    /// Audio clips on this track
    pub clips: Vec<ClipSnapshot>,
    /// VST plugins on this track
    pub plugins: Vec<PluginSnapshot>,
    /// Input alias ID (Phase 1.3: Hardware I/O routing)
    pub input_alias_id: Option<String>,

    // ========== MIDI SEQUENCER (Phase 2) ==========
    /// MIDI clips on this track
    pub midi_clips: Vec<MidiClipSnapshot>,
    /// Quantize setting (optional)
    pub quantize_division: Option<super::graph::QuantizeDivision>,
}

/// Audio clip snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipSnapshot {
    /// Clip UUID
    pub id: String,
    /// Path to audio file (relative to project)
    pub audio_path: String,
    /// Start time in samples
    pub start_sample: u64,
    /// Length in samples
    pub duration_samples: u64,
    /// Offset into source file (for trimming)
    pub offset_in_data: u64,
    /// Fade in length
    pub fade_in_len: u64,
    /// Fade out length
    pub fade_out_len: u64,
    /// Fade in type
    pub fade_in_type: super::fades::FadeType,
    /// Fade out type
    pub fade_out_type: super::fades::FadeType,
}

/// VCA Group Snapshot (Phase 5: Advanced Mixing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcaGroupSnapshot {
    pub id: String,
    pub name: String,
    pub member_tracks: Vec<String>,
    pub gain: ParameterSnapshot,
    pub is_muted: bool,
    pub is_solo: bool,
}

// ============================================================================
// MIDI SEQUENCER PERSISTENCE (Phase 2: The Composer Suite)
// ============================================================================

/// MIDI Note Snapshot (bitwise identical to MidiNote for efficiency)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiNoteSnapshot {
    pub start_sample: u64,
    pub length_samples: u64,
    pub note: u16,
    pub velocity: u32,
    pub channel: u8,
    pub pitch_bend: Option<i16>,
    pub pressure: Option<u8>,
    pub timbre: Option<u8>,
    pub probability: f32,
    pub velocity_random: u32,
    pub timing_random: i32,
}

/// MIDI CC Event Snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiCCSnapshot {
    pub sample: u64,
    pub cc_number: u8,
    pub value: u32,
    pub channel: u8,
}

/// MIDI Clip Snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiClipSnapshot {
    pub id: String,
    pub name: String,
    pub start_sample: u64,
    pub length_samples: u64,
    pub notes: Vec<MidiNoteSnapshot>,
    pub cc_events: Vec<MidiCCSnapshot>,
    pub color: String,
    pub is_muted: bool,
    pub is_looped: bool,
    // Composition tools
    pub scale: Option<super::graph::Scale>,
    pub chord_markers: Vec<super::graph::ChordMarker>,
    pub groove_template: Option<String>,
    pub pattern_id: Option<String>,
    pub tuning_steps: Option<u8>,
    pub time_signature_num: Option<u8>,
    pub time_signature_den: Option<u8>,
}

/// VST plugin snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSnapshot {
    /// Plugin UUID
    pub id: String,
    /// Plugin name/path
    pub plugin_path: String,
    /// Binary state blob (VST chunk)
    pub state_blob: Vec<u8>,
    /// Parameters
    pub parameters: Vec<ParameterSnapshot>,
}

/// Save project to binary .vibe file safely (Atomic Save)
pub fn save_project(snapshot: &ProjectSnapshot, path: &Path) -> Result<(), String> {
    // 1. Serialize project data using bincode (in memory)
    // TODO: For very large projects, we should stream this to avoid RAM spike
    let data =
        bincode::serialize(snapshot).map_err(|e| format!("Failed to serialize project: {}", e))?;

    // 2. Calculate CRC32 checksum
    let checksum = crc32fast::hash(&data);

    // 3. Create header
    let header = VibeFileHeader::new(data.len() as u64, checksum);

    // 4. Create Temporary File (Atomicity Step 1)
    let tmp_path = path.with_extension("vibe.tmp");
    let mut file =
        File::create(&tmp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    // 5. Write header manually (fixed size)
    file.write_all(&header.magic)
        .map_err(|e| format!("Failed to write magic: {}", e))?;
    file.write_all(&header.version.to_le_bytes())
        .map_err(|e| format!("Failed to write version: {}", e))?;
    file.write_all(&header.checksum.to_le_bytes())
        .map_err(|e| format!("Failed to write checksum: {}", e))?;
    file.write_all(&header.data_offset.to_le_bytes())
        .map_err(|e| format!("Failed to write data_offset: {}", e))?;
    file.write_all(&header.data_size.to_le_bytes())
        .map_err(|e| format!("Failed to write data_size: {}", e))?;

    // 6. Write data section
    file.write_all(&data)
        .map_err(|e| format!("Failed to write data: {}", e))?;

    // 7. Sync to Disk (Atomicity Step 2)
    // Ensures data is physically on the platter/NAND before we rename
    file.sync_all()
        .map_err(|e| format!("Failed to sync file to disk: {}", e))?;

    // 8. Atomic Rename (Atomicity Step 3)
    // This is the "Commit" point. Either the old file exists, or the new one does. Never half-written.
    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed to rename temp file to target: {}", e))?;

    println!(
        "VIBE: Saved project safely to {:?} ({} bytes)",
        path,
        data.len()
    );
    Ok(())
}

/// Load project from binary .vibe file
pub fn load_project(path: &Path) -> Result<ProjectSnapshot, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

    // Read header manually (fixed size: 4 + 4 + 4 + 8 + 8 = 28 bytes)
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| format!("Failed to read magic: {}", e))?;

    let mut version_bytes = [0u8; 4];
    file.read_exact(&mut version_bytes)
        .map_err(|e| format!("Failed to read version: {}", e))?;
    let version = u32::from_le_bytes(version_bytes);

    let mut checksum_bytes = [0u8; 4];
    file.read_exact(&mut checksum_bytes)
        .map_err(|e| format!("Failed to read checksum: {}", e))?;
    let checksum = u32::from_le_bytes(checksum_bytes);

    let mut data_offset_bytes = [0u8; 8];
    file.read_exact(&mut data_offset_bytes)
        .map_err(|e| format!("Failed to read data_offset: {}", e))?;
    let data_offset = u64::from_le_bytes(data_offset_bytes);

    let mut data_size_bytes = [0u8; 8];
    file.read_exact(&mut data_size_bytes)
        .map_err(|e| format!("Failed to read data_size: {}", e))?;
    let data_size = u64::from_le_bytes(data_size_bytes);

    // Reconstruct header
    let header = VibeFileHeader {
        magic,
        version,
        checksum,
        data_offset,
        data_size,
    };

    // Validate header
    header.validate()?;

    // Read data section
    let mut data = vec![0u8; data_size as usize];
    file.read_exact(&mut data)
        .map_err(|e| format!("Failed to read data: {}", e))?;

    // Verify checksum
    let actual_checksum = crc32fast::hash(&data);
    if actual_checksum != header.checksum {
        return Err(format!(
            "Checksum mismatch: expected {}, got {}",
            header.checksum, actual_checksum
        ));
    }

    // Deserialize project
    let snapshot: ProjectSnapshot =
        bincode::deserialize(&data).map_err(|e| format!("Failed to deserialize project: {}", e))?;

    println!("VIBE: Loaded project from {:?}", path);
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn test_binary_roundtrip() {
        // Create test snapshot with precise f64 values
        let snapshot = ProjectSnapshot {
            name: "Test Project".to_string(),
            bpm: 120.5,
            sample_rate: 48000.0,
            master_volume: 0.7853981633974483, // π/4 for precision test
            master_pan: 0.0,
            input_aliases: vec![],
            midi_bindings: vec![],
            loop_enabled: false,
            loop_start: 0,
            loop_end: 48000 * 4,
            vca_groups: vec![],
            tracks: vec![TrackSnapshot {
                id: "track-1".to_string(),
                name: "Vocal".to_string(),
                volume: ParameterSnapshot {
                    id: "vol-1".to_string(),
                    name: "Volume".to_string(),
                    value: 0.6366197723675814,
                    automation: vec![],
                },
                pan: ParameterSnapshot {
                    id: "pan-1".to_string(),
                    name: "Pan".to_string(),
                    value: -0.5,
                    automation: vec![],
                },
                width: ParameterSnapshot {
                    id: "width-1".to_string(),
                    name: "Width".to_string(),
                    value: 1.0,
                    automation: vec![],
                },
                input_drive: ParameterSnapshot {
                    id: "drive-1".to_string(),
                    name: "Drive".to_string(),
                    value: 0.0,
                    automation: vec![],
                },
                muted: false,
                solo: true,
                is_armed: false,
                phase_inverted: false,
                color: "#ff00ff".to_string(),
                input_alias_id: None,
                clips: vec![],
                plugins: vec![PluginSnapshot {
                    id: "plugin-1".to_string(),
                    plugin_path: "test.vst3".to_string(),
                    state_blob: vec![0xDE, 0xAD, 0xBE, 0xEF], // Binary blob
                    parameters: vec![],
                }],
                // MIDI Sequencer
                midi_clips: vec![],
                quantize_division: None,
            }],
        };

        // Save
        let temp_path = Path::new("test_project.vibe");
        save_project(&snapshot, temp_path).expect("Failed to save");

        // Load
        let loaded = load_project(temp_path).expect("Failed to load");

        // Verify bit-perfect equality
        assert_eq!(loaded.name, snapshot.name);
        assert_eq!(loaded.bpm, snapshot.bpm); // Exact f64 match
        assert_eq!(loaded.master_volume, snapshot.master_volume); // Exact f64 match
        assert_eq!(
            loaded.tracks[0].volume.value,
            snapshot.tracks[0].volume.value
        ); // Exact f64 match
        assert_eq!(
            loaded.tracks[0].plugins[0].state_blob,
            snapshot.tracks[0].plugins[0].state_blob
        ); // Exact binary match

        // Cleanup
        fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_invalid_magic_bytes() {
        let temp_path = Path::new("test_invalid.vibe");
        let mut file = File::create(temp_path).unwrap();

        // Write invalid magic bytes
        file.write_all(b"FAKE").unwrap();
        // Write dummy rest of header (24 more bytes to make 28 total)
        file.write_all(&[0u8; 24]).unwrap();

        let result = load_project(temp_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("magic bytes"));

        fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_midi_clip_roundtrip() {
        // Test MIDI Sequencer persistence (Phase 2)
        let snapshot = ProjectSnapshot {
            name: "MIDI Test".to_string(),
            bpm: 140.0,
            sample_rate: 48000.0,
            master_volume: 1.0,
            master_pan: 0.0,
            input_aliases: vec![],
            midi_bindings: vec![],
            loop_enabled: false,
            loop_start: 0,
            loop_end: 48000 * 4,
            vca_groups: vec![],
            tracks: vec![TrackSnapshot {
                id: "midi-track-1".to_string(),
                name: "MIDI Track".to_string(),
                volume: ParameterSnapshot {
                    id: "vol-1".to_string(),
                    name: "Volume".to_string(),
                    value: 0.8,
                    automation: vec![],
                },
                pan: ParameterSnapshot {
                    id: "pan-1".to_string(),
                    name: "Pan".to_string(),
                    value: 0.0,
                    automation: vec![],
                },
                width: ParameterSnapshot {
                    id: "width-1".to_string(),
                    name: "Width".to_string(),
                    value: 1.0,
                    automation: vec![],
                },
                input_drive: ParameterSnapshot {
                    id: "drive-1".to_string(),
                    name: "Drive".to_string(),
                    value: 0.0,
                    automation: vec![],
                },
                muted: false,
                solo: false,
                is_armed: false,
                phase_inverted: false,
                color: "#FF0000".to_string(),
                clips: vec![],
                plugins: vec![],
                input_alias_id: None,
                midi_clips: vec![MidiClipSnapshot {
                    id: "clip-1".to_string(),
                    name: "C Major Scale".to_string(),
                    start_sample: 0,
                    length_samples: 192000,
                    color: "#4a9eff".to_string(),
                    is_muted: false,
                    is_looped: false,
                    scale: None,
                    chord_markers: vec![],
                    groove_template: None,
                    pattern_id: None,
                    tuning_steps: None,
                    time_signature_num: None,
                    time_signature_den: None,
                    cc_events: vec![MidiCCSnapshot {
                        sample: 48000,
                        cc_number: 1, // Mod Wheel
                        value: 64 << 25,
                        channel: 0,
                    }],
                    notes: vec![
                        MidiNoteSnapshot {
                            start_sample: 0,
                            length_samples: 24000,
                            note: 60,
                            velocity: 100 << 25,
                            channel: 0,
                            pitch_bend: Some(100),
                            pressure: Some(64),
                            timbre: Some(32),
                            probability: 1.0,
                            velocity_random: 0,
                            timing_random: 0,
                        },
                        MidiNoteSnapshot {
                            start_sample: 24000,
                            length_samples: 24000,
                            note: 62, // D
                            velocity: 110 << 25,
                            channel: 0,
                            pitch_bend: None,
                            pressure: None,
                            timbre: None,
                            probability: 1.0,
                            velocity_random: 0,
                            timing_random: 0,
                        },
                    ],
                }],
                quantize_division: Some(crate::engine::graph::QuantizeDivision::Sixteenth),
            }],
        };

        // Save → Load → Verify
        let path = Path::new("test_midi.vibe");
        save_project(&snapshot, path).unwrap();
        let loaded = load_project(path).unwrap();

        // Verify MIDI data
        assert_eq!(loaded.tracks[0].midi_clips.len(), 1);
        assert_eq!(loaded.tracks[0].midi_clips[0].notes.len(), 2);
        assert_eq!(loaded.tracks[0].midi_clips[0].notes[0].note, 60);
        assert_eq!(loaded.tracks[0].midi_clips[0].notes[0].velocity, 100 << 25);
        assert_eq!(
            loaded.tracks[0].midi_clips[0].notes[0].pitch_bend,
            Some(100)
        );
        assert_eq!(loaded.tracks[0].midi_clips[0].cc_events.len(), 1);
        assert_eq!(loaded.tracks[0].midi_clips[0].cc_events[0].cc_number, 1);
        assert_eq!(
            loaded.tracks[0].quantize_division,
            Some(crate::engine::graph::QuantizeDivision::Sixteenth)
        );
        assert_eq!(loaded.tracks[0].midi_clips[0].is_looped, false);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_project_serialization_complex() {
        let mut snapshot = ProjectSnapshot {
            name: "Complex Setup".to_string(),
            bpm: 175.0,
            sample_rate: 96000.0,
            master_volume: 0.5,
            master_pan: 0.1,
            input_aliases: vec![],
            midi_bindings: vec![],
            loop_enabled: false,
            loop_start: 0,
            loop_end: 48000 * 4,
            vca_groups: vec![],
            tracks: vec![],
        };

        // Add a track with complex automation and layers
        let track = TrackSnapshot {
            id: Uuid::new_v4().to_string(),
            name: "Automated Synth".to_string(),
            volume: ParameterSnapshot {
                id: Uuid::new_v4().to_string(),
                name: "Volume".to_string(),
                value: 0.5,
                automation: vec![
                    crate::engine::automation::AutomationKnot {
                        sample_pos: 0,
                        value: 0.5,
                        tension: 0.0,
                    },
                    crate::engine::automation::AutomationKnot {
                        sample_pos: 96000,
                        value: 1.0,
                        tension: 0.5,
                    },
                ],
            },
            pan: ParameterSnapshot {
                id: Uuid::new_v4().to_string(),
                name: "Pan".to_string(),
                value: 0.0,
                automation: vec![],
            },
            width: ParameterSnapshot {
                id: Uuid::new_v4().to_string(),
                name: "Width".to_string(),
                value: 1.0,
                automation: vec![],
            },
            input_drive: ParameterSnapshot {
                id: Uuid::new_v4().to_string(),
                name: "Drive".to_string(),
                value: 0.0,
                automation: vec![],
            },
            muted: false,
            solo: false,
            is_armed: true,
            phase_inverted: false,
            color: "#00ffff".to_string(),
            clips: vec![],
            plugins: vec![],
            input_alias_id: None,
            midi_clips: vec![],
            quantize_division: None,
        };
        snapshot.tracks.push(track);

        let path = Path::new("complex_test.vibe");
        save_project(&snapshot, path).unwrap();
        let loaded = load_project(path).unwrap();

        assert_eq!(loaded.bpm, 175.0);
        assert_eq!(loaded.tracks[0].volume.automation.len(), 2);
        assert_eq!(loaded.tracks[0].volume.automation[1].sample_pos, 96000);
        assert_eq!(loaded.tracks[0].color, "#00ffff");

        fs::remove_file(path).ok();
    }
}
