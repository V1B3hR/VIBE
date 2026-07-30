use super::automation::AutomationCurve;
use crate::engine::synth::ModSlot;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Maximum number of channels supported (e.g., 7.1.4 = 12, 9.1.6 = 16)
pub const MAX_CHANNELS: usize = 16;
pub const MAX_BUFFER_SIZE: usize = 4096;

/// Denormal protection: flush very small numbers to zero
/// Prevents massive CPU slowdown from denormal floats in IIR filters
#[inline(always)]
pub fn flush_denormal_f64(x: f64) -> f64 {
    const DENORMAL_THRESHOLD: f64 = 1e-15;
    if x.abs() < DENORMAL_THRESHOLD {
        0.0
    } else {
        x
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub enum AutomationMode {
    Read,
    Write,
    Touch,
    Latch,
    Off,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub enum MonitoringMode {
    Auto,
    In,
    Off,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub enum WarpMode {
    Beats,
    Tones,
    Texture,
    Complex,
    Repitch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Marker {
    pub id: Uuid,
    pub label: String,
    pub position: u64,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Parameter {
    pub id: Uuid,
    pub name: String,
    #[serde(skip)]
    pub atomic_value: Arc<AtomicU64>, // Stores bits of f64
    pub value: f64, // For serialization
    pub min_value: f64,
    pub max_value: f64,
    #[serde(skip, default = "default_curve")]
    pub curve: Arc<ArcSwap<AutomationCurve>>,
}

fn default_curve() -> Arc<ArcSwap<AutomationCurve>> {
    Arc::new(ArcSwap::from_pointee(AutomationCurve::new(0.0)))
}

impl Parameter {
    pub fn new(name: &str, initial: f64, min: f64, max: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            atomic_value: Arc::new(AtomicU64::new(initial.to_bits())),
            value: initial,
            min_value: min,
            max_value: max,
            curve: Arc::new(ArcSwap::from_pointee(AutomationCurve::new(initial))),
        }
    }

    pub fn set_value(&self, val: f64) {
        let clamped = val.clamp(self.min_value, self.max_value);
        self.atomic_value
            .store(clamped.to_bits(), Ordering::Relaxed);
    }

    pub fn get_value_at(&self, sample_pos: u64) -> f64 {
        // First check automation
        let curve = self.curve.load();
        if !curve.knots.is_empty() {
            curve.get_value_at(sample_pos)
        } else {
            f64::from_bits(self.atomic_value.load(Ordering::Relaxed))
        }
    }

    pub fn get_current_value(&self) -> f64 {
        f64::from_bits(self.atomic_value.load(Ordering::Relaxed))
    }

    pub fn record_value(&self, sample_pos: u64, value: f64) {
        let mut curve = (**self.curve.load()).clone();
        curve.record_value(sample_pos, value);
        self.curve.store(Arc::new(curve));
    }

    pub fn add_knot(&self, sample_pos: u64, value: f64) {
        let mut curve = (**self.curve.load()).clone();
        curve.add_knot(sample_pos, value);
        self.curve.store(Arc::new(curve));
    }

    pub fn clear_automation(&self) {
        let mut curve = (**self.curve.load()).clone();
        curve.knots.clear();
        self.curve.store(Arc::new(curve));
    }

    pub fn set_automation_tension(&self, sample_pos: u64, tension: f64) {
        let mut curve = (**self.curve.load()).clone();
        curve.set_tension(sample_pos, tension);
        self.curve.store(Arc::new(curve));
    }

    pub fn set_automation_interpolation(&self, interpolation: crate::engine::automation::InterpolationType) {
        let mut curve = (**self.curve.load()).clone();
        curve.interpolation = interpolation;
        self.curve.store(Arc::new(curve));
    }
}

impl Clone for Parameter {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            atomic_value: self.atomic_value.clone(),
            value: self.get_current_value(),
            min_value: self.min_value,
            max_value: self.max_value,
            curve: self.curve.clone(),
        }
    }
}

pub struct AudioBuffer {
    pub channels_data: Vec<Vec<f64>>,
    pub frames: usize,
    pub num_channels: usize,
}

impl AudioBuffer {
    pub fn new() -> Self {
        let mut channels = Vec::with_capacity(MAX_CHANNELS);
        for _ in 0..MAX_CHANNELS {
            channels.push(vec![0.0; MAX_BUFFER_SIZE]);
        }
        Self {
            channels_data: channels,
            frames: 0,
            num_channels: 0,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        for c in 0..self.num_channels {
            // Using fast clear for pre-allocated arrays
            self.channels_data[c][..self.frames].fill(0.0);
        }
    }

    #[allow(dead_code)]
    pub fn channel_mut(&mut self, channel: usize) -> &mut [f64] {
        &mut self.channels_data[channel][..self.frames]
    }

    #[allow(dead_code)]
    pub fn channel(&self, channel: usize) -> &[f64] {
        &self.channels_data[channel][..self.frames]
    }

    #[allow(dead_code)]
    pub fn get_stereo_mut(&mut self) -> (&mut [f64], &mut [f64]) {
        let (left_slice, right_slice) = self.channels_data.split_at_mut(1);
        (
            &mut left_slice[0][..self.frames],
            &mut right_slice[0][..self.frames],
        )
    }
}

/// Context for audio processing (passed to every processor)
pub struct ProcessingContext<'a> {
    pub sample_rate: f64,
    pub playhead: u64,
    pub sidechain: Option<&'a AudioBuffer>,
}

/// Trait for anything that can process or generate audio.
pub trait AudioProcessor: Send {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext);
    fn id(&self) -> Uuid;
    fn clone_box(&self) -> Box<dyn AudioProcessor>;
    fn name(&self) -> String {
        "Generic Processor".to_string()
    }
    fn on_midi_event(&mut self, _status: u8, _data1: u16, _data2: u32) {}

    // Phase 4: Interaction & Expression
    #[allow(dead_code)]
    fn on_mpe_event(&mut self, _event: crate::engine::mpe_handler::MpeOutputEvent) {}
    #[allow(dead_code)]
    fn on_midi2_event(&mut self, _event: crate::engine::midi2_support::Midi2Output) {}
    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        Vec::new()
    }
    /// Reports the processing latency in samples (for PDC)
    fn latency_samples(&self) -> usize {
        0
    }

    /// Serialization support for plugin state (VST chunks, etc.)
    fn get_state(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Deserialization support for plugin state
    fn set_state(&mut self, _state: &[u8]) {}

    /// Open external editor window (VST3 GUI), returns requested dimensions (width, height)
    fn open_editor(&mut self, _handle: *mut std::ffi::c_void) -> Option<(u32, u32)> {
        None
    }

    /// Close external editor window
    #[allow(dead_code)]
    fn close_editor(&mut self) {}

    /// Downcasting support for specialized processors (like PrismaEQ)
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        panic!("Downcasting not supported for this processor");
    }

    /// Set bypass state (for SmartProcessorWrapper support)
    fn set_bypass(&mut self, _bypass: bool) {}

    /// Get bypass state
    fn is_bypassed(&self) -> bool {
        false
    }

    /// Drain GUI-driven parameter changes (plugin -> host)
    fn drain_plugin_feedback(&self) -> Vec<(String, f64)> {
        Vec::new()
    }

    /// Report CPU usage (0.0 - 1.0)
    fn get_cpu_usage(&self) -> f32 {
        0.0
    }

    /// Get list of factory programs
    fn get_programs(&self) -> Vec<String> {
        Vec::new()
    }

    /// Set active program
    fn set_program(&mut self, _index: i32) {}

    /// Check if latency changed and needs PDC recalculation
    fn needs_pdc_recalc(&self) -> bool {
        false
    }

    /// Reset the recalc flag
    fn reset_pdc_recalc(&mut self) {}

    /// Poll for any editor resize requests (new_width, new_height)
    fn poll_editor_resize(&self) -> Option<(u32, u32)> {
        None
    }
}

/// Lightweight representation of a processor for the management thread.
/// Shares parameters via Arc, but performs no processing.
pub struct DummyProcessor {
    pub id: Uuid,
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[allow(dead_code)]
impl DummyProcessor {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            name,
            parameters: Vec::new(),
        }
    }

    pub fn new_with_params(id: Uuid, name: String, parameters: Vec<Parameter>) -> Self {
        Self {
            id,
            name,
            parameters,
        }
    }
}

impl AudioProcessor for DummyProcessor {
    fn process(&mut self, _buffer: &mut AudioBuffer, _context: &ProcessingContext) {}
    fn id(&self) -> Uuid {
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        self.parameters.iter_mut().collect()
    }
    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            name: self.name.clone(),
            parameters: self.parameters.to_vec(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioClipInfo {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub start_sample: u64,
    pub duration_samples: u64,
    pub peaks: Vec<Vec<f32>>,
    pub offset_in_data: u64,
    pub fade_in_len: u64,
    pub fade_out_len: u64,
    pub fade_in_type: String,
    pub fade_out_type: String,
    pub gain: f32,
    pub pitch_semitones: f32,
    pub playback_speed: f64,
    pub is_warped: bool,
    pub has_gain_envelope: bool,
    pub has_pitch_envelope: bool,
    pub transient_count: usize,
    /// Per-clip color override (empty = use track color)
    pub color: String,
}

// ============================================================================
// MIDI SEQUENCER STRUCTURES (Phase 2: The Composer Suite)
// ============================================================================

/// MIDI Note with full articulation data (MPE + MIDI 2.0 ready)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MidiNote {
    /// Start position in samples (sample-accurate timing)
    pub start_sample: u64,
    /// Duration in samples (0 = infinite until Note Off)
    pub length_samples: u64,
    /// MIDI note number (0-127 for MIDI 1.0, 0-65535 for MIDI 2.0)
    pub note: u16,
    /// Velocity (0-127 for MIDI 1.0, upscaled to 32-bit for MIDI 2.0)
    pub velocity: u32,
    /// MIDI channel (0-15, or per-note for MPE)
    pub channel: u8,
    /// Optional: Per-note pitch bend (for MPE, vibrato, slides)
    pub pitch_bend: Option<i16>,
    /// Optional: Per-note pressure/aftertouch (for MPE)
    pub pressure: Option<u8>,
    /// Optional: Per-note timbre/brightness (CC74 for MPE)
    pub timbre: Option<u8>,

    // --- Advanced Features ---
    /// Note probability (0.0-1.0, 1.0 = always plays)
    pub probability: f32,
    /// Velocity randomization (+/- this value)
    pub velocity_random: u32,
    /// Timing randomization in samples (signed)
    pub timing_random: i32,
}

/// MIDI CC (Control Change) Event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiCCEvent {
    /// Sample position
    pub sample: u64,
    /// CC number (0-127)
    pub cc_number: u8,
    /// CC value (0-127 for MIDI 1.0, upscaled to 32-bit for MIDI 2.0)
    pub value: u32,
    /// MIDI channel
    pub channel: u8,
}

/// MIDI Clip containing notes and automation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiClip {
    pub id: Uuid,
    pub name: String,
    /// Timeline position (start)
    pub start_sample: u64,
    /// Clip length (defines loop boundary)
    pub length_samples: u64,
    /// All notes in this clip (relative to clip start)
    pub notes: Vec<MidiNote>,
    /// CC events (mod wheel, expression, sustain, etc.)
    pub cc_events: Vec<MidiCCEvent>,
    /// Color for visual identification
    pub color: String,
    /// Mute state (per-clip mute independent of track)
    pub is_muted: bool,
    /// Loop mode (for repeating patterns)
    pub is_looped: bool,

    // --- Composition Tools ---
    /// Scale metadata for snap-to-scale
    pub scale: Option<Scale>,
    /// Detected chord markers
    pub chord_markers: Vec<ChordMarker>,
    /// Groove template ID
    /// Groove template ID
    pub groove_template: Option<String>,
    /// Linked Pattern ID
    pub pattern_id: Option<String>,
    /// Microtonal steps per octave (default 12)
    pub tuning_steps: Option<u8>,
    pub time_signature_num: Option<u8>, // Default 4
    pub time_signature_den: Option<u8>, // Default 4

    // Phase 8: Ghost Clips
    pub reference_clip_id: Option<Uuid>,
}

impl MidiClip {
    pub fn quantize(&mut self, division: QuantizeDivision, bpm: f32, sample_rate: f64) {
        let samples_per_beat = (60.0 / bpm as f64) * sample_rate;
        let grid_size = match division {
            QuantizeDivision::Whole => samples_per_beat * 4.0,
            QuantizeDivision::Half => samples_per_beat * 2.0,
            QuantizeDivision::Quarter => samples_per_beat,
            QuantizeDivision::Eighth => samples_per_beat / 2.0,
            QuantizeDivision::Sixteenth => samples_per_beat / 4.0,
            QuantizeDivision::ThirtySecond => samples_per_beat / 8.0,
            QuantizeDivision::Triplet => samples_per_beat / 3.0,
        };

        for note in &mut self.notes {
            let grid_pos = (note.start_sample as f64 / grid_size).round() * grid_size;
            // Ensure we don't quantize to negative (which u64 can't handle, but round() handles +)
            // Note: round() of -0.0 is -0.0, but u64 cast should be fine for 0.0.
            note.start_sample = grid_pos.max(0.0) as u64;
        }
    }

    pub fn apply_groove(&mut self, template: &GrooveTemplate, bpm: f32, sample_rate: f64) {
        let samples_per_beat = (60.0 / bpm as f64) * sample_rate;
        let samples_per_16th = samples_per_beat / 4.0;

        for note in &mut self.notes {
            // Determine position in 16th notes
            let grid_index = (note.start_sample as f64 / samples_per_16th).round() as usize;
            let pattern_index = grid_index % 16;

            // Apply Timing
            let offset = template.timing_offsets[pattern_index] as f64 * samples_per_16th;
            let new_start = note.start_sample as f64 + offset;
            note.start_sample = new_start.max(0.0) as u64;

            // Apply Velocity
            let vel_scale = template.velocity_scale[pattern_index];
            let new_vel = (note.velocity as f32 * vel_scale).clamp(0.0, 127.0);
            note.velocity = new_vel as u32;
        }

        self.groove_template = Some(template.name.clone());
    }

    pub fn humanize(&mut self, timing_amount: f32, velocity_amount: f32, sample_rate: f64) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for note in &mut self.notes {
            // Randomize timing (+/- timing_amount in ms)
            let timing_offset_samples =
                (rng.gen_range(-1.0..1.0) * timing_amount as f64 * 0.001 * sample_rate) as i64;
            let new_start = note.start_sample as i64 + timing_offset_samples;
            note.start_sample = new_start.max(0) as u64;

            // Randomize velocity (+/- velocity_amount)
            let vel_offset = rng.gen_range(-velocity_amount..velocity_amount) as i32;
            let new_vel = note.velocity as i32 + vel_offset;
            note.velocity = new_vel.clamp(1, 127) as u32;
        }
    }
}

/// Scale definition
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scale {
    pub root_note: u8, // 0-11 (C-B)
    pub scale_type: ScaleType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ScaleType {
    Major,
    Minor,
    Pentatonic,
    Blues,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Custom(Vec<u8>), // Intervals
}

/// Chord marker for visualization
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChordMarker {
    pub sample: u64,
    pub chord_name: String, // "Cmaj7", "Dm", "G7"
    pub confidence: f32,    // 0.0-1.0 (auto-detected)
}

/// Groove template for micro-timing
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrooveTemplate {
    pub name: String,
    /// 16 slots for 16th note timing offsets (-1.0 to 1.0)
    pub timing_offsets: [f32; 16],
    /// 16 slots for velocity scaling (0.0 to 2.0)
    pub velocity_scale: [f32; 16],
}

/// Frontend DTO for MidiClip (lightweight for timeline rendering)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiClipInfo {
    pub id: String,
    pub name: String,
    pub start_sample: u64,
    pub length_samples: u64,
    pub note_count: usize,
    pub color: String,
    pub is_muted: bool,
    pub is_looped: bool,
    /// Lightweight preview data (first 100 notes for timeline rendering)
    /// Lightweight preview data (first 100 notes for timeline rendering)
    pub preview_notes: Vec<(u64, u16, u32)>, // (start, note, velocity)
    pub pattern_id: Option<String>,
    pub tuning_steps: Option<u8>,
    pub time_signature_num: Option<u8>,
    pub time_signature_den: Option<u8>,
    pub gain_offset: f32,
    pub has_envelope: bool,
}

/// Quantization divisions (musical grid)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum QuantizeDivision {
    Whole,        // 1/1 note
    Half,         // 1/2 note
    Quarter,      // 1/4 note
    Eighth,       // 1/8 note
    Sixteenth,    // 1/16 note
    ThirtySecond, // 1/32 note
    Triplet,      // 1/8T (triplet)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParameterInfo {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub automation: Vec<super::automation::AutomationKnot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EffectInfo {
    pub id: String,
    pub name: String,
    pub is_bypassed: bool,
    pub parameters: Vec<ParameterInfo>,
    pub mod_matrix: Option<Vec<ModSlot>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub enum TrackType {
    Audio,
    MIDI,
    Instrument,
    Aux,
    Group,
    Folder,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackInfo {
    pub id: String,
    pub name: String,
    pub volume: ParameterInfo,
    pub pan: ParameterInfo,
    pub width: ParameterInfo,
    pub input_drive: ParameterInfo,
    pub is_muted: bool,
    pub is_solo: bool,
    pub is_armed: bool,
    pub phase_inverted: bool,
    pub clips: Vec<AudioClipInfo>,
    pub midi_clips: Vec<MidiClipInfo>,
    pub color: String,
    pub is_frozen: bool,
    pub is_disabled: bool,
    pub is_automation_armed: bool,
    pub bus_id: Option<String>,
    pub input_source: Option<String>,
    pub output_target: Option<String>,
    pub sidechain_source_id: Option<String>,
    pub effects: Vec<EffectInfo>,
    pub console_eq: EffectInfo,
    pub console_comp: EffectInfo,
    pub eq_pre_dynamics: ParameterInfo,
    pub track_type: TrackType,
    pub monitoring_mode: MonitoringMode,
    pub parent_id: Option<String>,
    pub is_collapsed: bool,
    pub height: f32,
    pub peak_l: f32,
    pub peak_r: f32,
    pub rms_l: f32,
    pub rms_r: f32,
    pub lufs_l: f32,
    pub lufs_r: f32,
    // Phase 8: Pro Features
    pub playlist_count: usize,
    pub active_playlist_name: String,
    pub take_count: usize,
    pub comp_mode_enabled: bool,
    pub comp_lanes: Vec<Vec<AudioClipInfo>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackLevel {
    pub id: String,
    pub peaks: Vec<f32>,
    pub rms: Vec<f32>,
    pub true_peaks: Vec<f32>,
    pub lufs_momentary: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub struct WarpMarker {
    pub id: Uuid,
    pub original_pos_samples: u64,
    pub timeline_pos_beats: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioClip {
    pub id: Uuid,
    pub name: String,
    pub head_data: Arc<Vec<f32>>, // First 750ms (or entire clip if small)
    pub peaks: Vec<Vec<f32>>,
    pub start_sample: u64,
    pub offset_in_data: u64,
    pub length_in_samples: u64,
    pub sample_rate: u32,
    /// Per-clip color override (empty = inherit track color)
    pub color: String,

    // Liquid Fades
    pub fade_in_len: u64,
    pub fade_out_len: u64,
    pub fade_in_type: super::fades::FadeType,
    pub fade_out_type: super::fades::FadeType,
    pub gain: f32,
    pub pitch_semitones: f32,
    pub playback_speed: f64,
    pub is_warped: bool,
    pub is_reversed: bool,
    pub warp_mode: WarpMode,

    pub path: Option<String>,
    #[serde(skip)]
    pub waveform_cache: Option<Arc<crate::engine::waveform::PyramidCache>>,
    pub is_streaming: bool,

    #[cfg(target_os = "windows")]
    #[serde(skip)]
    pub file: Option<Arc<std::fs::File>>,

    // Phase 8: Clip Envelopes
    pub gain_envelope: Option<super::automation::AutomationCurve>,
    pub pitch_envelope: Option<super::automation::AutomationCurve>,
    pub pan_envelope: Option<super::automation::AutomationCurve>,

    // Transient Detection
    pub transients: Vec<u64>, // sample offsets

    // Warping System
    pub warp_markers: Vec<WarpMarker>,
    pub base_bpm: f64,

    // Phase 8: Ghost Clips (Visual/Data Referencing)
    pub reference_clip_id: Option<Uuid>,
}

impl AudioClip {
    /// Calculate instantaneous playback rate for warping
    pub fn get_warp_playback_rate(&self, sample_pos: f64, project_bpm: f64, sample_rate: f64) -> f64 {
        if !self.is_warped {
            return self.playback_speed;
        }

        if self.warp_markers.len() < 2 {
            // General tempo sync: ratio = (project_bpm / base_bpm)
            return project_bpm / self.base_bpm.max(20.0);
        }

        // Find segments between markers
        let mut m1 = &self.warp_markers[0];
        let mut m2 = &self.warp_markers[self.warp_markers.len() - 1];

        // Binary search or linear scan for the segment
        for i in 0..self.warp_markers.len() - 1 {
            if sample_pos as u64 >= self.warp_markers[i].original_pos_samples && (sample_pos as u64) < self.warp_markers[i+1].original_pos_samples {
                m1 = &self.warp_markers[i];
                m2 = &self.warp_markers[i+1];
                break;
            }
        }

        let beats_diff = (m2.timeline_pos_beats - m1.timeline_pos_beats).abs();
        let samples_diff = (m2.original_pos_samples as f64 - m1.original_pos_samples as f64).abs();

        if samples_diff < 1.0 || beats_diff < 0.001 {
             return project_bpm / self.base_bpm.max(20.0);
        }

        // Rate = (beats/samples) * (samples_per_beat_at_project_bpm)
        let samples_per_beat = sample_rate * 60.0 / project_bpm;
        (beats_diff / samples_diff) * samples_per_beat
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TrackPlaylist {
    pub name: String,
    pub clips: Vec<AudioClip>,
    pub midi_clips: Vec<MidiClip>,
}

pub struct Track {
    pub id: Uuid,
    pub name: String,
    pub volume: Parameter,
    pub pan: Parameter,
    pub width: Parameter,
    pub input_drive: Parameter,
    pub processors: Vec<Box<dyn AudioProcessor>>,
    pub clips: Vec<AudioClip>,
    pub is_muted: bool,
    pub is_solo: bool,
    pub is_armed: bool,
    pub phase_inverted: bool,
    pub color: String,
    pub is_frozen: bool,
    pub is_disabled: bool,
    pub is_automation_armed: bool,
    pub automation_mode: AutomationMode,
    pub bus_id: Option<Uuid>,
    pub input_source: Option<String>,
    pub output_target: Option<String>,
    pub sidechain_source_id: Option<Uuid>,
    pub input_alias_id: Option<Uuid>,
    pub track_type: TrackType,
    pub monitoring_mode: MonitoringMode,
    pub parent_id: Option<Uuid>,
    pub is_collapsed: bool,
    pub height: f32,

    // Phase 8: Pro Features
    pub takes: Vec<Vec<AudioClip>>, // Comping lanes
    pub playlists: Vec<TrackPlaylist>,
    pub active_playlist_idx: usize,
    pub comp_mode_enabled: bool,

    // Runtime cache for input routing (resolved hardware channels)
    pub input_channels: Option<Vec<usize>>,
    // Pre-allocated buffer for this track
    pub internal_buffer: AudioBuffer,
    // Liquid Core: Active voices for overlapping clips
    pub active_voices: Vec<ActiveVoice>,
    // Zero-Allocation Summing: Pre-allocated output buffer (AudioBuffer)
    pub output_buffer: AudioBuffer,
    // Aux Input Summing: Pre-allocated buffer for incoming sends
    pub aux_input_buffer: AudioBuffer,
    // PDC: Plugin Delay Compensation buffers
    #[allow(dead_code)]
    pub pdc_delay_buffer: Vec<Vec<f64>>, // [channel][samples]
    #[allow(dead_code)]
    pub pdc_delay_samples: usize,
    /// Circular buffer write cursor (Phase 5 PDC Optimization)
    pub pdc_write_index: usize,

    // ========== MIDI SEQUENCER (Phase 2) ==========
    /// MIDI clips on this track
    pub midi_clips: Vec<MidiClip>,
    /// MIDI recording buffer (for live input capture)
    pub midi_recording_buffer: Vec<MidiNote>,
    /// Quantize settings (for snap-to-grid on playback/record)
    pub quantize_division: Option<QuantizeDivision>,

    // ========== PERFORMANCE & ANALYSIS (Phase 3) ==========
    /// GPU-Optimized Metering (Peak/RMS)
    pub meter: Arc<crate::engine::metering::GpuMeter>,
    /// Per-track spectrum analyzer
    pub spectrum_analyzer: crate::engine::eq_module::analysis::spectrum::SpectrumAnalyzer,

    // Console Strip
    pub equalizer: crate::engine::eq_module::dsp::equalizer::Equalizer,
    pub compressor: crate::engine::dynamics_module::dsp::compressor::Compressor,
    pub eq_pre_dynamics: Parameter,
    pub cpu_usage: Arc<AtomicU64>, // Performance monitoring (micros)

    // Mixer Sends (Phase 5: Mixer Sends/Returns)
    pub sends: Vec<TrackSend>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackSend {
    pub id: Uuid,
    pub target_id: Uuid, // Destination Track (Aux/Group)
    pub gain: Parameter,
    pub is_post_fader: bool,
    pub is_muted: bool,
}

pub struct ActiveVoice {
    pub clip_id: Uuid,
    pub start_sample_on_timeline: u64,
    pub current_sample_in_clip: f64,
    pub total_samples: u64,
    pub fade_in_len: u64,
    pub fade_out_len: u64,
    pub fade_in_type: super::fades::FadeType,
    pub fade_out_type: super::fades::FadeType,
    pub gain: f32,
    pub pitch_semitones: f32,
    pub playback_speed: f64,
    pub is_warped: bool,
    // HyperStream support
    pub streamer_state: Option<Box<dyn std::any::Any + Send>>,
}

impl Track {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            // Volume in Decibels: -60dB (Silence) to +6dB (Max)
            volume: Parameter::new("Volume", 0.0, -60.0, 6.0),
            pan: Parameter::new("Pan", 0.0, -1.0, 1.0),
            width: Parameter::new("Width", 1.0, 0.0, 2.0),
            input_drive: Parameter::new("Drive", 0.0, 0.0, 1.0), // 0.0 = Clean, 1.0 = Max Saturation
            processors: Vec::new(),
            clips: Vec::new(),
            is_muted: false,
            is_solo: false,
            is_armed: false,
            phase_inverted: false,
            color: "#4a9eff".to_string(),
            bus_id: None,
            input_source: None,
            output_target: None,
            sidechain_source_id: None,
            input_alias_id: None,
            track_type: TrackType::Audio,
            monitoring_mode: MonitoringMode::Auto,
            parent_id: None,
            is_collapsed: false,
            is_frozen: false,
            is_disabled: false,
            is_automation_armed: false,
            automation_mode: AutomationMode::Read,
            height: 80.0,
            input_channels: None,
            internal_buffer: AudioBuffer::new(),
            active_voices: Vec::new(),
            output_buffer: AudioBuffer::new(),
            aux_input_buffer: AudioBuffer::new(),
            pdc_delay_buffer: vec![vec![0.0; 8192]; 2],
            pdc_delay_samples: 0,
            pdc_write_index: 0,
            // MIDI Sequencer
            midi_clips: Vec::new(),
            midi_recording_buffer: Vec::new(),
            quantize_division: None,
            // Performance
            meter: Arc::new(crate::engine::metering::GpuMeter::new(44100)),
            // Spectrum
            spectrum_analyzer: crate::engine::eq_module::analysis::spectrum::SpectrumAnalyzer::new(
                2048,
            ),
            equalizer: crate::engine::eq_module::dsp::equalizer::Equalizer::new(44100.0),
            compressor: crate::engine::dynamics_module::dsp::compressor::Compressor::new(44100.0),
            eq_pre_dynamics: Parameter::new("EQ Pre/Post", 1.0, 0.0, 1.0),
            takes: Vec::new(),
            playlists: Vec::new(),
            active_playlist_idx: 0,
            comp_mode_enabled: false,
            cpu_usage: Arc::new(AtomicU64::new(0)),
            sends: Vec::new(),
        }
    }

    pub fn get_all_parameters(&mut self) -> Vec<&mut Parameter> {
        let mut params = vec![
            &mut self.volume,
            &mut self.pan,
            &mut self.width,
            &mut self.input_drive,
        ];

        // Console Strip
        params.extend(self.equalizer.get_parameters());
        params.extend(self.compressor.get_parameters());

        // Processors
        for proc in &mut self.processors {
            params.extend(proc.get_parameters());
        }

        params
    }

    pub fn get_all_parameters_ref(&self) -> Vec<&Parameter> {
        let params = vec![&self.volume, &self.pan, &self.width, &self.input_drive];
        // Console strip and processors are harder to get refs to because of the trait/struct boundaries
        // but for now we focus on the core track parameters
        params
    }

    pub fn slice_clip(&mut self, clip_id: Uuid, split_pos: u64) {
        // Audio Clips
        let mut new_audio_clip: Option<AudioClip> = None;
        if let Some(pos) = self.clips.iter().position(|c| c.id == clip_id) {
            let clip = &mut self.clips[pos];
            if split_pos > clip.start_sample
                && split_pos < (clip.start_sample + clip.length_in_samples)
            {
                let split_offset = split_pos - clip.start_sample;
                let remaining_len = clip.length_in_samples - split_offset;

                let mut right_clip = clip.clone();
                right_clip.id = Uuid::new_v4();
                right_clip.name = format!("{} (Right)", clip.name);
                right_clip.start_sample = split_pos;
                right_clip.offset_in_data += split_offset;
                right_clip.length_in_samples = remaining_len;
                right_clip.fade_in_len = 0;
                right_clip.gain = clip.gain;

                new_audio_clip = Some(right_clip);

                clip.length_in_samples = split_offset;
                clip.fade_out_len = 0;
            }
        }
        if let Some(c) = new_audio_clip {
            self.clips.push(c);
        }

        // MIDI Clips
        let mut new_midi_clip: Option<MidiClip> = None;
        if let Some(pos) = self.midi_clips.iter().position(|c| c.id == clip_id) {
            let clip = &mut self.midi_clips[pos];
            if split_pos > clip.start_sample
                && split_pos < (clip.start_sample + clip.length_samples)
            {
                let split_offset = split_pos - clip.start_sample;
                let remaining_len = clip.length_samples - split_offset;

                let mut right_clip = clip.clone();
                right_clip.id = Uuid::new_v4();
                right_clip.name = format!("{} (Right)", clip.name);
                right_clip.start_sample = split_pos;
                right_clip.length_samples = remaining_len;

                // Shift Notes
                // Remove notes from right that are before split
                right_clip.notes.retain(|n| n.start_sample >= split_offset);
                for note in &mut right_clip.notes {
                    // Assuming start_sample is u64 based on error message
                    note.start_sample -= split_offset;
                }

                // Remove notes from left that are after split
                clip.notes.retain(|n| n.start_sample < split_offset);

                clip.length_samples = split_offset;
                new_midi_clip = Some(right_clip);
            }
        }
        if let Some(c) = new_midi_clip {
            self.midi_clips.push(c);
        }
    }

    pub fn clone_as_dummy(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            volume: self.volume.clone(),
            pan: self.pan.clone(),
            width: self.width.clone(),
            input_drive: self.input_drive.clone(),
            processors: Vec::new(),
            clips: self.clips.clone(),
            is_muted: self.is_muted,
            is_solo: self.is_solo,
            is_armed: self.is_armed,
            phase_inverted: self.phase_inverted,
            color: self.color.clone(),
            bus_id: self.bus_id,
            input_source: self.input_source.clone(),
            takes: self.takes.clone(),
            playlists: self.playlists.clone(),
            active_playlist_idx: self.active_playlist_idx,
            comp_mode_enabled: self.comp_mode_enabled,
            cpu_usage: self.cpu_usage.clone(),
            output_target: self.output_target.clone(),
            sidechain_source_id: self.sidechain_source_id,
            input_alias_id: self.input_alias_id,
            track_type: self.track_type,
            parent_id: self.parent_id,
            is_collapsed: self.is_collapsed,
            is_frozen: self.is_frozen,
            is_disabled: self.is_disabled,
            is_automation_armed: self.is_automation_armed,
            automation_mode: self.automation_mode,
            height: self.height,
            input_channels: self.input_channels.clone(),
            internal_buffer: AudioBuffer::new(),
            active_voices: Vec::new(),
            output_buffer: AudioBuffer::new(),
            aux_input_buffer: AudioBuffer::new(),
            pdc_delay_buffer: vec![vec![0.0; 8192]; MAX_CHANNELS],
            pdc_delay_samples: 0,
            pdc_write_index: 0,
            // MIDI Sequencer
            midi_clips: self.midi_clips.clone(),
            midi_recording_buffer: Vec::new(),
            quantize_division: self.quantize_division,
            // Mixing
            sends: self.sends.clone(),
            monitoring_mode: self.monitoring_mode,
            // Performance - new instance or share? Usually dummy wants its own or share.
            // Sharing is fine for dummy as it's often used for snapshotting state.
            meter: self.meter.clone(),
            // Spectrum - new instance for dummy
            spectrum_analyzer: crate::engine::eq_module::analysis::spectrum::SpectrumAnalyzer::new(
                2048,
            ),
            equalizer: crate::engine::eq_module::dsp::equalizer::Equalizer::new(44100.0),
            compressor: crate::engine::dynamics_module::dsp::compressor::Compressor::new(44100.0),
            eq_pre_dynamics: self.eq_pre_dynamics.clone(),
        }
    }

    pub fn clone_for_audio_thread(&self) -> Self {
        let mut clone = self.clone_as_dummy();
        clone.processors = self.processors.iter().map(|p| p.clone_box()).collect();
        clone
    }

    pub fn process(
        &mut self,
        frames: usize,
        sample_rate: f64,
        project_bpm: f64,
        playhead: u64,
        sidechain_buffer: Option<&AudioBuffer>,
        fades: &super::fades::FadeLuts,
        hyper_pool: &Arc<crate::engine::streamer::GlobalBufferPool>,
        hyper_streamer: &Arc<crate::engine::streamer::WindowsAsyncStreamer>,
        offline: bool,
        hardware_inputs: &[Vec<f32>],
        is_playing: bool,
    ) {
        if self.is_muted || self.is_disabled || self.is_frozen {
            // Clear output buffers
            self.output_buffer.frames = frames;
            self.output_buffer.num_channels = 2;
            self.output_buffer.clear();
            return;
        }

        self.internal_buffer.frames = frames;
        self.internal_buffer.num_channels = 2;
        self.internal_buffer.clear();
        
        // Sum incoming Mixer Sends
        for c in 0..2 {
            for s in 0..frames {
                self.internal_buffer.channels_data[c][s] = self.aux_input_buffer.channels_data[c][s];
            }
        }

        // Phase 5: Input Monitoring
        let should_monitor = match self.monitoring_mode {
            MonitoringMode::In => true,
            MonitoringMode::Off => false,
            MonitoringMode::Auto => self.is_armed && !is_playing,
        };

        if should_monitor {
            if let Some(channels) = &self.input_channels {
                for (i, &hw_ch) in channels.iter().enumerate() {
                    if hw_ch < hardware_inputs.len() {
                        let target_ch = i % 2; // Map to stereo
                        for s in 0..frames {
                            self.internal_buffer.channels_data[target_ch][s] += hardware_inputs[hw_ch][s] as f64;
                        }
                    }
                }
            }
        }

        // 1. Voice Management: Find new clips that overlap with this buffer
        for clip in &self.clips {
            let clip_end = clip.start_sample + clip.length_in_samples;
            let buffer_end = playhead + frames as u64;

            // Check if clip exists in the [playhead, buffer_end] range
            if clip_end > playhead && clip.start_sample < buffer_end {
                // Check if already active
                if !self.active_voices.iter().any(|v| v.clip_id == clip.id) {
                    // Start new voice
                    let start_in_clip = playhead.saturating_sub(clip.start_sample);

                    self.active_voices.push(ActiveVoice {
                        clip_id: clip.id,
                        start_sample_on_timeline: clip.start_sample,
                        current_sample_in_clip: start_in_clip as f64,
                        total_samples: clip.length_in_samples,
                        fade_in_len: clip.fade_in_len,
                        fade_out_len: clip.fade_out_len,
                        fade_in_type: clip.fade_in_type,
                        fade_out_type: clip.fade_out_type,
                        gain: clip.gain,
                        pitch_semitones: clip.pitch_semitones,
                        playback_speed: if clip.is_warped {
                            project_bpm / clip.base_bpm.max(20.0)
                        } else {
                            clip.playback_speed
                        },
                        is_warped: clip.is_warped,
                        streamer_state: if clip.is_streaming {
                            #[cfg(target_os = "windows")]
                            {
                                if let Some(ref file) = clip.file {
                                    Some(Box::new(super::streamer::reader::HyperStreamReader::new(
                                        clip.id,
                                        Arc::clone(&clip.head_data),
                                        clip.length_in_samples,
                                        Arc::clone(file),
                                        clip.sample_rate,
                                        clip.offset_in_data,
                                        offline,
                                    )))
                                } else {
                                    None
                                }
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                None
                            }
                        } else {
                            None
                        },
                    });
                }
            }
        }

        // 2. Sum Voices
        for voice in &mut self.active_voices {
            // Find underlying clip data
            if let Some(clip) = self.clips.iter().find(|c| c.id == voice.clip_id) {
                let start_idx = if voice.start_sample_on_timeline > playhead {
                    (voice.start_sample_on_timeline - playhead) as usize
                } else {
                    0
                };

                // Determine how many frames to process for this voice
                let remaining_in_clip =
                    (voice.total_samples as f64 - voice.current_sample_in_clip).max(0.0);

                // Effective speed including pitch and warp
                let mut effective_speed = if voice.is_warped {
                    clip.get_warp_playback_rate(voice.current_sample_in_clip, project_bpm, sample_rate)
                } else {
                    voice.playback_speed
                };
                
                let pitch_speed = 2.0f64.powf(voice.pitch_semitones as f64 / 12.0);
                effective_speed *= pitch_speed;

                let frames_available_f =
                    (frames as f64 - start_idx as f64).min(remaining_in_clip / effective_speed);
                let frames_available = frames_available_f.ceil() as usize;

                if frames_available == 0 {
                    continue;
                }

                // Temporary buffers (lightweight allocation)
                let mut temp_l = vec![0.0f32; frames_available];
                let mut temp_r = vec![0.0f32; frames_available];
                let current_pos = voice.current_sample_in_clip;

                // 1. Fetch Samples (Bulk)
                if clip.is_streaming {
                    // HyperStream: Reader handles transition from RAM Head to Disk Tail
                    if let Some(ref mut state) = voice.streamer_state {
                        if let Some(reader) = state
                            .downcast_mut::<crate::engine::streamer::reader::HyperStreamReader>(
                        ) {
                            // TODO: Add interpolation to reader
                            reader.read_samples(
                                current_pos as u64,
                                &mut temp_l,
                                &mut temp_r,
                                hyper_pool,
                                hyper_streamer,
                            );
                        }
                    } else {
                        // Fallback if state is missing but streaming enabled (shouldn't happen)
                        // Read from head if possible
                        for i in 0..frames_available {
                            let s = current_pos + i as f64;
                            let idx = (s * 2.0) as usize + (clip.offset_in_data as usize * 2);
                            // Note: offset_in_data might need to be verified for streaming clips?
                            // Usually streaming clips start at 0 offset in file, but maybe trimmed.
                            // HyperStreamReader handles logical position.
                            // If we fallback here, we assume head_data corresponds to start of clip file.
                            if idx + 1 < clip.head_data.len() {
                                temp_l[i] = clip.head_data[idx];
                                temp_r[i] = clip.head_data[idx + 1];
                            }
                        }
                    }
                } else {
                    // RAM Only (Small Clip)
                    for i in 0..frames_available {
                        let s = current_pos + i as f64;
                        let idx = (clip.offset_in_data as f64 + s) as usize * 2;
                        if idx + 1 < clip.head_data.len() {
                            temp_l[i] = clip.head_data[idx];
                            temp_r[i] = clip.head_data[idx + 1];
                        }
                    }
                }

                // Determination of per-sample gain and interpolation
                for i in 0..frames_available {
                    let out_idx = start_idx + i;
                    let sample_pos = voice.current_sample_in_clip + (i as f64 * effective_speed);

                    if sample_pos >= voice.total_samples as f64 {
                        break;
                    }

                    // Linear Interpolation
                    let idx0 = sample_pos.floor() as usize;
                    let idx1 = (idx0 + 1).min(voice.total_samples as usize - 1);
                    let frac = (sample_pos - idx0 as f64) as f32;

                    let (s0_l, s0_r) = if clip.is_streaming {
                        (temp_l[i], temp_r[i]) // TODO: Reader needs proper interpolation for fractional reads
                    } else {
                        let base_p = (idx0 as u64 + clip.offset_in_data) * 2;
                        if (base_p as usize + 1) < clip.head_data.len() {
                            (
                                clip.head_data[base_p as usize],
                                clip.head_data[base_p as usize + 1],
                            )
                        } else {
                            (0.0, 0.0)
                        }
                    };

                    let (s1_l, s1_r) = if clip.is_streaming {
                        (temp_l[i], temp_r[i])
                    } else {
                        let next_p = (idx1 as u64 + clip.offset_in_data) * 2;
                        if (next_p as usize + 1) < clip.head_data.len() {
                            (
                                clip.head_data[next_p as usize],
                                clip.head_data[next_p as usize + 1],
                            )
                        } else {
                            (0.0, 0.0)
                        }
                    };

                    let sample_l = s0_l + (s1_l - s0_l) * frac;
                    let sample_r = s0_r + (s1_r - s0_r) * frac;

                    let mut current_gain = voice.gain;

                    // Apply Liquid Fade In
                    if sample_pos < voice.fade_in_len as f64 {
                        let progress = (sample_pos / voice.fade_in_len as f64) as f32;
                        current_gain *= fades.get_gain(&voice.fade_in_type, progress, true);
                    }
                    // Apply Liquid Fade Out
                    else if sample_pos > (voice.total_samples as f64 - voice.fade_out_len as f64)
                    {
                        let dist_from_end = voice.total_samples as f64 - sample_pos;
                        let progress = 1.0 - (dist_from_end as f32 / voice.fade_out_len as f32);
                        current_gain *= fades.get_gain(&voice.fade_out_type, progress, false);
                    }

                    self.internal_buffer.channels_data[0][out_idx] +=
                        sample_l as f64 * current_gain as f64;
                    self.internal_buffer.channels_data[1][out_idx] +=
                        sample_r as f64 * current_gain as f64;
                }

                voice.current_sample_in_clip += frames_available as f64 * effective_speed;
            }
        }

        // 3. Cleanup finished voices
        self.active_voices
            .retain(|v| v.current_sample_in_clip < v.total_samples as f64);

        // 4. MIDI Clip Processing (Phase 2: The Composer Suite)
        // Process MIDI clips and send events to first processor (assumed to be instrument)
        for midi_clip in &self.midi_clips {
            if midi_clip.is_muted {
                continue;
            }

            let clip_end = midi_clip.start_sample + midi_clip.length_samples;
            let buffer_end = playhead + frames as u64;

            // Check if clip overlaps with current buffer
            if clip_end > playhead && midi_clip.start_sample < buffer_end {
                // Process notes
                for note in &midi_clip.notes {
                    let note_abs_start = midi_clip.start_sample + note.start_sample;
                    let note_abs_end = note_abs_start + note.length_samples;

                    // Note On
                    if note_abs_start >= playhead && note_abs_start < buffer_end {
                        if let Some(processor) = self.processors.first_mut() {
                            processor.on_midi_event(
                                0x90 | note.channel, // Note On
                                note.note,
                                note.velocity,
                            );
                        }
                    }

                    // Note Off
                    if note_abs_end >= playhead
                        && note_abs_end < buffer_end
                        && note.length_samples > 0
                    {
                        if let Some(processor) = self.processors.first_mut() {
                            processor.on_midi_event(
                                0x80 | note.channel, // Note Off
                                note.note,
                                0,
                            );
                        }
                    }
                }

                // Process CC events
                for cc in &midi_clip.cc_events {
                    let cc_abs_sample = midi_clip.start_sample + cc.sample;
                    if cc_abs_sample >= playhead && cc_abs_sample < buffer_end {
                        if let Some(processor) = self.processors.first_mut() {
                            processor.on_midi_event(
                                0xB0 | cc.channel, // CC
                                cc.cc_number as u16,
                                cc.value,
                            );
                        }
                    }
                }
            }
        }

        // 0. Console Strip (Dynamics & EQ)
        let context = ProcessingContext {
            sample_rate,
            playhead,
            sidechain: sidechain_buffer,
        };

        // Determine order
        let eq_pre = self.eq_pre_dynamics.get_current_value() > 0.5;

        if eq_pre {
            self.equalizer.process(&mut self.internal_buffer, &context);
            self.compressor.process(&mut self.internal_buffer, &context);
        } else {
            self.compressor.process(&mut self.internal_buffer, &context);
            self.equalizer.process(&mut self.internal_buffer, &context);
        }

        // Apply Insert Effects (Dynamics, EQ, etc.)
        for processor in &mut self.processors {
            processor.process(&mut self.internal_buffer, &context);
        }

        // Phase 3: Per-Track Spectrum Analysis
        // We mix down stereo to mono for analysis
        // We do this BEFORE volume fader to show "pre-fader" signal or AFTER for "post-fader"?
        // Usually visualizers are post-fx, pre-fader (or post-fader).
        // Let's do POST-FX, PRE-FADER for now so we see what the EQ does regardless of track volume.
        // Actually, for EQ work, PRE-FADER is best.
        let mut mix_down = [0.0; MAX_BUFFER_SIZE];
        super::simd_optimized::sum_stereo_to_mono_optimized(
            &mut mix_down[..frames],
            &self.internal_buffer.channels_data[0][..frames],
            &self.internal_buffer.channels_data[1][..frames],
        );
        let mix_f32: Vec<f32> = mix_down[..frames].iter().map(|&x| x as f32).collect();
        self.spectrum_analyzer.push_samples(&mix_f32);
        self.spectrum_analyzer.analyze();

        // Apply automated parameters (Phase -> Width -> Volume -> Pan) and write to output buffers
        // Apply Phase Invert
        if self.phase_inverted {
            for c in 0..2 {
                super::simd_optimized::apply_gain_simd_optimized(
                    &mut self.internal_buffer.channels_data[c][..frames],
                    -1.0,
                );
            }
        }

        let width_curve = self.width.curve.load();
        let vol_curve = self.volume.curve.load();
        let pan_curve = self.pan.curve.load();

        // Pass 1: Width
        if width_curve.knots.is_empty() {
            let w = self.width.get_current_value();
            let (l_slice, r_slice) = self.internal_buffer.get_stereo_mut();
            super::simd_optimized::apply_width_optimized(
                &mut l_slice[..frames],
                &mut r_slice[..frames],
                w,
            );
        } else {
            for i in 0..frames {
                let w = width_curve.get_value_at(playhead + i as u64);
                let l = self.internal_buffer.channels_data[0][i];
                let r = self.internal_buffer.channels_data[1][i];
                let m = (l + r) * 0.5;
                let s = (l - r) * 0.5 * w;
                self.internal_buffer.channels_data[0][i] = m + s;
                self.internal_buffer.channels_data[1][i] = m - s;
            }
        }

        // Pass 2: Volume
        if vol_curve.knots.is_empty() {
            let vol_db = self.volume.get_current_value();
            let vol = if vol_db <= -60.0 {
                0.0
            } else {
                10.0f64.powf(vol_db / 20.0)
            };
            if (vol - 1.0).abs() > 0.0001 {
                for c in 0..2 {
                    super::simd_optimized::apply_gain_simd_optimized(
                        &mut self.internal_buffer.channels_data[c][..frames],
                        vol,
                    );
                }
            }
        } else {
            for i in 0..frames {
                let vol_db = vol_curve.get_value_at(playhead + i as u64);
                let vol = if vol_db <= -60.0 {
                    0.0
                } else {
                    10.0f64.powf(vol_db / 20.0)
                };
                self.internal_buffer.channels_data[0][i] *= vol;
                self.internal_buffer.channels_data[1][i] *= vol;
            }
        }

        // Pass 3: Pan
        if pan_curve.knots.is_empty() {
            let p = self.pan.get_current_value();
            let (l_slice, r_slice) = self.internal_buffer.get_stereo_mut();
            super::simd_optimized::apply_pan_optimized(
                &mut l_slice[..frames],
                &mut r_slice[..frames],
                p,
            );
        } else {
            for i in 0..frames {
                let p = pan_curve.get_value_at(playhead + i as u64);
                let theta = (p + 1.0) * std::f64::consts::FRAC_PI_4;
                self.internal_buffer.channels_data[0][i] *= theta.cos();
                self.internal_buffer.channels_data[1][i] *= theta.sin();
            }
        }

        // Copy Result to Output Buffer
        for c in 0..2 {
            self.output_buffer.channels_data[c][..frames]
                .copy_from_slice(&self.internal_buffer.channels_data[c][..frames]);
        }

        // Phase 4.5: Plugin Delay Compensation (PDC)
        if self.pdc_delay_samples > 0 {
            let mut chans: Vec<&mut [f64]> = self
                .output_buffer
                .channels_data
                .iter_mut()
                .take(self.output_buffer.num_channels)
                .map(|c| c.as_mut_slice())
                .collect();

            super::pdc::PdcManager::apply_compensation(
                &mut self.pdc_delay_buffer,
                &mut chans,
                self.pdc_delay_samples,
                &mut self.pdc_write_index,
                frames,
            );
        }

        // 4. Update Meters for Visual Feedback
        self.meter.update(
            &self.output_buffer.channels_data[0][..frames],
            &self.output_buffer.channels_data[1][..frames],
        );
    }
}

#[allow(dead_code)]
pub struct Bus {
    pub id: Uuid,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub color: String,
    pub processors: Vec<Box<dyn AudioProcessor>>,
    pub volume: Parameter,
    pub internal_buffer: AudioBuffer,
}

impl Bus {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            color,
            processors: Vec::new(),
            volume: Parameter::new("Volume", 1.0, 0.0, 2.0),
            internal_buffer: AudioBuffer::new(),
        }
    }

    #[allow(dead_code)]
    pub fn process(
        &mut self,
        frames: usize,
        input_l: &mut [f64],
        input_r: &mut [f64],
        sample_rate: f64,
        playhead: u64,
    ) {
        // 1. Prepare Internal Buffer (f64 for high precision processing)
        self.internal_buffer.frames = frames;
        self.internal_buffer.num_channels = 2;

        // Copy Input (f64) -> Internal (f64)
        self.internal_buffer.channels_data[0][..frames].copy_from_slice(&input_l[..frames]);
        self.internal_buffer.channels_data[1][..frames].copy_from_slice(&input_r[..frames]);

        // 2. Apply Processors (Inserts)
        let context = ProcessingContext {
            sample_rate,
            playhead,
            sidechain: None,
        };
        for processor in &mut self.processors {
            processor.process(&mut self.internal_buffer, &context);
        }

        // 3. Apply Fader Volume
        // TODO: Automation support for Bus Volume
        let vol = self.volume.get_current_value();
        if (vol - 1.0).abs() > 0.0001 {
            super::simd_optimized::apply_gain_simd_optimized(
                &mut self.internal_buffer.channels_data[0][..frames],
                vol,
            );
            super::simd_optimized::apply_gain_simd_optimized(
                &mut self.internal_buffer.channels_data[1][..frames],
                vol,
            );
        }

        // 4. Output (f64) -> Input Buffers (f64)
        // Replacing the input content with the processed signal
        input_l[..frames].copy_from_slice(&self.internal_buffer.channels_data[0][..frames]);
        input_r[..frames].copy_from_slice(&self.internal_buffer.channels_data[1][..frames]);
    }
}

impl Clone for Bus {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            color: self.color.clone(),
            processors: self.processors.iter().map(|p| p.clone_box()).collect(),
            volume: self.volume.clone(),
            internal_buffer: AudioBuffer::new(),
        }
    }
}

// Effect Implementations

pub struct GainEffect {
    id: Uuid,
    pub gain: Parameter,
}

impl GainEffect {
    pub fn new(gain: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            gain: Parameter::new("Gain", gain, 0.0, 4.0),
        }
    }
}

impl AudioProcessor for GainEffect {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let playhead = context.playhead;
        let curve = self.gain.curve.load();

        if curve.knots.is_empty() {
            let g = self.gain.get_current_value();
            if (g - 1.0).abs() > 0.0001 {
                for c in 0..buffer.num_channels {
                    super::simd_optimized::apply_gain_simd_optimized(
                        &mut buffer.channels_data[c][..buffer.frames],
                        g,
                    );
                }
            }
        } else {
            for i in 0..buffer.frames {
                let g = curve.get_value_at(playhead + i as u64);
                for c in 0..buffer.num_channels {
                    buffer.channels_data[c][i] *= g;
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }
    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            gain: self.gain.clone(),
        })
    }
    fn name(&self) -> String {
        "Gain".to_string()
    }
    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![&mut self.gain]
    }
}

/// Low-pass filter (RBJ biquad coefficients)
/// Currently unused in main chain – prepared as building block for:
/// - Prisma EQ low-shelf / low-pass bands
/// - Delay / reverb damping / feedback smoothing
/// - Anti-aliasing in oversampling stages
/// - Gentle roll-off on master bus
#[allow(dead_code)]
pub struct LowPassFilter {
    id: Uuid,
    pub cutoff: Parameter,
    prev_sample: [f64; MAX_CHANNELS],
}

impl LowPassFilter {
    #[allow(dead_code)]
    pub fn new(cutoff: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            cutoff: Parameter::new("Cutoff", cutoff, 0.001, 0.99),
            prev_sample: [0.0; MAX_CHANNELS],
        }
    }
}

impl AudioProcessor for LowPassFilter {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let playhead = context.playhead;
        for i in 0..buffer.frames {
            let alpha = self.cutoff.get_value_at(playhead + i as u64);
            for c in 0..buffer.num_channels {
                let sample = buffer.channels_data[c][i];
                let out = self.prev_sample[c] + alpha * (sample - self.prev_sample[c]);
                // Denormal protection: prevent CPU slowdown
                self.prev_sample[c] = flush_denormal_f64(out);
                buffer.channels_data[c][i] = out;
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }
    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            cutoff: self.cutoff.clone(),
            prev_sample: [0.0; MAX_CHANNELS],
        })
    }
    fn name(&self) -> String {
        "Low Pass Filter".to_string()
    }
    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![&mut self.cutoff]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_denormal_protection() {
        assert_eq!(flush_denormal_f64(1e-16), 0.0);
        assert_eq!(flush_denormal_f64(-1e-16), 0.0);
        assert_eq!(flush_denormal_f64(0.5), 0.5);
    }

    #[test]
    fn test_audio_buffer_clear() {
        let mut buffer = AudioBuffer::new();
        buffer.frames = 10;
        buffer.num_channels = 2;

        for i in 0..10 {
            buffer.channels_data[0][i] = 1.0;
            buffer.channels_data[1][i] = 1.0;
        }

        buffer.clear();

        for i in 0..10 {
            assert_eq!(buffer.channels_data[0][i], 0.0);
            assert_eq!(buffer.channels_data[1][i], 0.0);
        }
    }

    #[test]
    fn test_parameter_atomic() {
        let param = Parameter::new("Test", 0.5, 0.0, 1.0);
        assert_eq!(param.get_current_value(), 0.5);
        param.set_value(0.75);
        assert_eq!(param.get_current_value(), 0.75);
    }

    #[test]
    fn lowpass_basic() {
        let mut lp = LowPassFilter::new(0.5);
        let mut buffer = AudioBuffer::new();
        buffer.frames = 100;
        buffer.num_channels = 2;
        for c in 0..2 {
            for i in 0..100 {
                buffer.channels_data[c][i] = 1.0;
            }
        }
        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        lp.process(&mut buffer, &context);
        // Low pass of DC step should eventually reach 1.0 but be smoothed
        assert!(buffer.channels_data[0][0] > 0.0);
        assert!(buffer.channels_data[0][0] < 1.0);
    }

    #[test]
    fn test_track_creation() {
        let track = Track::new("Test".to_string());
        // Internal/Output AudioBuffer capacity is boxed fixed size, checking length isn't straightforward on Box<[[f64]]> without access
        // But we can check frames
        assert_eq!(track.output_buffer.frames, 0);
    }
}
