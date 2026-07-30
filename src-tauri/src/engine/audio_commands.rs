use crate::engine::graph::{
    AudioProcessor, Bus, GrooveTemplate, MidiCCEvent, MidiClip, MidiNote, QuantizeDivision, Scale,
    Track,
};
use crate::engine::synth::ModSlot;
use std::path::PathBuf;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct MidiEvent {
    pub sample_offset: u32,
    pub status: u8,
    pub data1: u16, // Support 16-bit Note/Index (MIDI 2.0)
    pub data2: u32, // Support 32-bit Value (MIDI 2.0 / NRPN High-Res)
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct ParamChange {
    pub id: Uuid,
    pub value: f64,
}

#[allow(dead_code)]
pub enum GraphCommand {
    AddTrack(Track),
    RemoveTrack(usize),
    AddProcessor(usize, Box<dyn AudioProcessor>),
    SetTrackMute(usize, bool),
    SetTrackSolo(usize, bool),
    SetTrackPan(usize, f64),
    SetTrackWidth(usize, f64),
    SetTrackDrive(usize, f64),
    SetTrackPhaseInvert(usize, bool),
    SetTrackArm(usize, bool),
    SetTrackType(usize, super::graph::TrackType),
    SetTrackParent(usize, Option<Uuid>),
    SetTrackCollapsed(usize, bool),
    SetTrackHeight(usize, f32),
    SetTrackFrozen(usize, bool),
    SetTrackDisabled(usize, bool),
    SetTrackAutomationArmed(usize, bool),
    SetTrackAutomationMode(usize, super::graph::AutomationMode),
    SetTrackColor(usize, String),
    SetAudioClipPitch(usize, Uuid, f32),
    SetAudioClipWarpMode(usize, Uuid, super::graph::WarpMode),
    SetAudioClipWarp(usize, Uuid, bool, f64),
    SetAudioClipGain(usize, Uuid, f32),
    HumanizeMidiClip(usize, Uuid, f32, f32),
    // Bus Commands
    AddBus(Bus),
    RemoveBus(usize),
    AddBusProcessor(usize, Box<dyn AudioProcessor>),
    SetBusVolume(usize, f64),
    UpdateModMatrix(usize, usize, Vec<ModSlot>),
    LoadPreset(usize, usize, crate::engine::synth::SynthPreset),
    SetTrackInput(usize, Option<Uuid>, Option<Vec<usize>>), // Track Index, Alias ID, Resolved Channels
    SetTrackSidechain(usize, Option<Uuid>),                 // Track Index, Sidechain Source ID
    // FX Rack
    SetEffectBypass(usize, usize, bool), // track_idx, proc_idx, bypass
    MoveEffect(usize, usize, usize),     // track_idx, from, to
    RemoveEffect(usize, usize),          // track_idx, proc_idx
    // Clip Operations (DSP)
    ReverseAudioClip(usize, Uuid),
    NormalizeAudioClip(usize, Uuid, f32), // track_idx, clip_id, target_db
    SetAudioClipFades(usize, Uuid, u64, u64),
    SetCrossfade(usize, Uuid, Uuid, u64),
    // Timeline Operations
    InsertSilence(u64, u64),
    DeleteTime(u64, u64),
    DuplicateTime(u64, u64),
    // MIDI Advanced
    TransposeMidiClip(usize, Uuid, i32),
    DuplicateMidiNotes(usize, Uuid, Vec<usize>),
    SetAudioClipReverse(usize, Uuid, bool),
    MoveTrack(usize, usize),
    // VCA Group Commands
    AddVcaGroup(crate::engine::vca_group::VcaGroup),
    RemoveVcaGroup(Uuid),
    SetVcaGain(Uuid, f64),
    SetVcaMute(Uuid, bool),
    SetVcaSolo(Uuid, bool),
    AddTrackToVca(Uuid, Uuid),      // vca_id, track_id
    RemoveTrackFromVca(Uuid, Uuid), // vca_id, track_id
    SetMonitoringMode(usize, super::graph::MonitoringMode),
    // Mixer Sends (Phase 8)
    AddTrackSend(usize, super::graph::TrackSend),
    RemoveTrackSend(usize, Uuid),
    SetTrackSendGain(usize, Uuid, f64),
    SetTrackSendMute(usize, Uuid, bool),
}

/// Command Pattern: All GUI interactions are sent as commands to the audio thread
/// This ensures complete separation between UI and DSP, preventing any blocking operations
/// in the real-time audio callback. Commands are processed in the management loop,
/// not in the audio callback itself.
#[allow(dead_code)]
pub enum AudioCommand {
    Play,
    Pause,
    Stop,
    AddTrack(Track),
    SetTrackVolume(usize, f64),
    SetTrackPan(usize, f64),
    SetTrackWidth(usize, f64),
    SetTrackDrive(usize, f64),
    SetTrackMute(usize, bool),
    SetTrackSolo(usize, bool),
    SetTrackPhaseInvert(usize, bool),
    SetTrackArm(usize, bool),
    SetTrackInput(usize, Option<String>),
    SetTrackOutput(usize, Option<String>),
    SetTrackSidechain(usize, Option<String>),
    SetParameter(Uuid, f64), // Processor ID, New Value
    ImportToLibrary(PathBuf),
    AddClipToTrack(usize, Uuid, u64),
    DeleteClip(usize, Uuid),
    SliceClip(usize, Uuid, u64),
    MoveClip(usize, Uuid, usize, u64),
    AddEffect(usize, String),
    SetPlayhead(u64),
    SetBPM(f32),
    SetTrackType(usize, super::graph::TrackType),
    SetTrackParent(usize, Option<String>),
    SetTrackCollapsed(usize, bool),
    SetTrackHeight(usize, f32),
    SetTrackFrozen(usize, bool),
    SetTrackDisabled(usize, bool),
    SetTrackAutomationArmed(usize, bool),
    SetTrackAutomationMode(usize, super::graph::AutomationMode),
    SetTrackColor(usize, String),
    SetAudioClipPitch(usize, Uuid, f32),
    SetAudioClipWarpMode(usize, Uuid, super::graph::WarpMode),
    SetAudioClipWarp(usize, Uuid, bool, f64),
    SetAudioClipGain(usize, Uuid, f32),
    HumanizeMidiClip(usize, Uuid, f32, f32),
    ReEnableAutomation(Uuid),
    AddBus(String, String),       // Name, Color
    RouteTrackToBus(usize, Uuid), // Track Index, Bus ID
    AddAutomationPoint(Uuid, u64, f64),
    SetAutomationTension(Uuid, u64, f64),
    ClearAutomation(Uuid),
    LoadSynthPreset(usize, usize, String), // TrackIdx, ProcIdx, Path
    SaveSynthPreset(usize, usize, String), // TrackIdx, ProcIdx, Path
    UpdateModMatrix(usize, usize, Vec<ModSlot>),
    MidiNoteOn(u8, u8),
    MidiNoteOff(u8),
    MidiCC(u8, u8),           // Legacy 7-bit CC
    MidiControl(u8, u8, f64), // Channel, CC Number, Normalized Value (0.0 - 1.0)
    MapMidi(u8, Uuid),        // CC Number, Parameter ID
    ToggleRecord,
    Undo,
    Redo,
    Checkout(Uuid),
    CreateBranch(String),
    ScanPlugins,
    AddPluginToTrack(usize, PathBuf),
    SetClipFades(usize, Uuid, u64, u64),
    ResizeClip(usize, Uuid, u64, u64, u64),
    SetMetronome(bool),

    // --- MIDI Commands (Phase 2) ---
    AddMidiClip(usize, MidiClip),
    DeleteMidiClip(usize, Uuid),
    UpdateMidiClip(usize, Uuid, MidiClip),
    StartMidiRecording(usize),
    StopMidiRecording(usize),
    SetQuantization(usize, Option<QuantizeDivision>),

    // MIDI Note CRUD
    AddMidiNote(usize, String, MidiNote), // track_idx, clip_id, note
    DeleteMidiNote(usize, String, usize), // track_idx, clip_id, note_idx
    SetAudioConfig(crate::engine::audio_device::AudioDeviceConfig),
    UpdateMidiNote(usize, String, usize, MidiNote), // track_idx, clip_id, note_idx, new_note

    // CC Event CRUD
    AddCCEvent(usize, String, MidiCCEvent), // track_idx, clip_id, cc_event
    DeleteCCEvent(usize, String, usize),    // track_idx, clip_id, cc_idx

    // Composition Tools
    SetClipScale(usize, String, Option<Scale>), // track_idx, clip_id, scale
    DetectChords(usize, String),                // track_idx, clip_id
    ApplyGrooveTemplate(usize, String, String), // track_idx, clip_id, template_name
    ApplyGrooveCustom(usize, String, GrooveTemplate), // track_idx, clip_id, template
    QuantizeNotes(usize, String, QuantizeDivision), // track_idx, clip_id, division
    StartMidiLearn(Uuid),

    // Stability & Stress Testing
    GenerateStressNotes(usize, String, usize), // track_idx, clip_id, count

    // Phase 3: Prisma EQ
    UpdateEqBand(usize, String, crate::engine::eq_module::EqBand), // track_idx, processor_id, band
    SetEqBands(usize, String, Vec<crate::engine::eq_module::EqBand>), // track_idx, processor_id, bands

    // Phase 3.10: Advanced Routing Matrix
    GraphConnect {
        from_node: Uuid,
        to_node: Uuid,
        from_port: u32,
        to_port: u32,
        gain_db: f64,
    },
    GraphDisconnect {
        from_node: Uuid,
        to_node: Uuid,
    },
    AddBinding(crate::engine::midi_mapping::MidiBinding),
    RemoveBinding(Uuid),
    GraphAddNode(crate::engine::audio_graph::GraphNode),
    GraphRemoveNode(Uuid),
    // Kinetic Automation
    SetAutomationInterpolation(Uuid, crate::engine::automation::InterpolationType),
    SetAutomationLayer(Uuid, crate::engine::automation::AutomationLayer),
    PreviewSampleSynced(Vec<f32>, Option<QuantizeDivision>, bool, f32, f32), // data, quantize, stretch, strength, swing
    PreviewSeek(f32),                                              // 0.0 - 1.0
    StopPreview,
    SetGlobalSwing(f32),

    // IO Commands (Phase 1.3)
    CreateInputAlias(String, bool, Vec<usize>, String), // Name, IsStereo, Channels, Color
    DeleteInputAlias(Uuid),
    // FX Rack / WASM
    SetEffectBypass(usize, String, bool),
    MoveEffect(usize, String, usize),
    RemoveEffect(usize, String), // track_idx, processor_id string
    AddWasmPlugin(usize, String),
    SetLoopEnabled(bool),
    SetLoopRange(u64, u64),
    // Arrangement Operations
    InsertSilence(u64, u64),
    DeleteTime(u64, u64),
    DuplicateTime(u64, u64),
    AddMarker(String, u64, String), // label, position, color
    RemoveMarker(Uuid),
    // Clip Operations
    ReverseClip(usize, Uuid),
    NormalizeClip(usize, Uuid, f32),
    ConsolidateClips(usize, Vec<Uuid>),
    RenameClip(usize, Uuid, String),
    SetCrossfade(usize, Uuid, Uuid, u64),
    SetClipColor(usize, Uuid, String),  // track_idx, clip_id, color
    // MIDI Advanced
    TransposeMidiClip(usize, Uuid, i32),
    LegatoMidiClip(usize, Uuid),
    DuplicateMidiNotes(usize, Uuid, Vec<usize>),
    ConvertMidiToAudio(usize, Uuid),
    // Track Management
    RenameTrack(usize, String),
    DuplicateTrack(usize),
    RemoveTrack(usize),
    MoveTrack(usize, usize),  // from_idx, to_idx
    // New Project
    NewProject,
    // Finalization (Phase 8/Arrangement)
    ConvertAudioToMidi(usize, Uuid, String), // track_idx, clip_id, mode
    ExtractGroove(usize, Uuid),              // track_idx, clip_id
    SetTimeSignature(u8, u8),                // numerator, denominator
    PasteTime(u64),                          // position

    // Pro-Level Arrangement (Phase 8 Finalization)
    SetGlobalQuantization(QuantizeDivision),
    SetTempoAutomation(super::automation::AutomationCurve),
    SetCompMode(usize, bool),              // track_idx, enabled
    SetActiveTake(usize, usize),           // track_idx, take_idx
    AddTakeFromSelection(usize, u64, u64), // track_idx, start, end (creates take from selection)
    AddPlaylist(usize, String),            // track_idx, name
    SetActivePlaylist(usize, usize),       // track_idx, playlist_idx
    DetectTransients(usize, Uuid),         // track_idx, clip_id
    SetClipEnvelope(usize, Uuid, String, super::automation::AutomationCurve), // track_idx, clip_id, type ("gain", "pitch", "pan"), curve

    // Dropel Phase 4: Active Intervention Upgrade
    SetParamSmooth(Uuid, Uuid, f64, f32), // node_id, param_id, value, smooth_ms
    AddWarpMarker(Uuid, u64, f64),        // clip_id, at_sample, timeline_beats
    ApplyEQBand(Uuid, usize, f64, f64, f64), // node_id, band_idx, freq, gain, q
    ToggleBypass(Uuid),                   // node_id
    SuggestAndPreview(Uuid, u32),         // suggestion_id, preview_duration_ms
    // VCA Groups (Phase 5)
    AddVcaGroup(String),
    RemoveVcaGroup(Uuid),
    SetVcaGain(Uuid, f64),
    SetVcaMute(Uuid, bool),
    SetVcaSolo(Uuid, bool),
    AddTrackToVca(Uuid, Uuid),      // vca_id, track_id
    RemoveTrackFromVca(Uuid, Uuid), // vca_id, track_id
    SetMonitoringMode(usize, super::graph::MonitoringMode),
    // Mixer Sends & Snapshots (Phase 8)
    AddTrackSend(usize, Uuid, f64, bool), // track_idx, target_track_id, gain, is_post_fader
    RemoveTrackSend(usize, Uuid),        // track_idx, target_track_id
    SetTrackSendGain(usize, Uuid, f64),
    SetTrackSendMute(usize, Uuid, bool),
    SaveMixSnapshot(String),
    LoadMixSnapshot(Uuid),
}
