use super::graph::{flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use uuid::Uuid;

const MAX_VOICES: usize = 16;
const UNISON_COUNT: usize = 7;

#[derive(Clone, Copy, PartialEq)]
enum OscType {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

impl From<f64> for OscType {
    fn from(v: f64) -> Self {
        match v.round() as i32 {
            0 => OscType::Sine,
            1 => OscType::Saw,
            2 => OscType::Square,
            3 => OscType::Triangle,
            _ => OscType::Noise,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

struct EnvelopeState {
    stage: EnvStage,
    level: f64,
    time_in_stage: f64,
}

impl EnvelopeState {
    fn new() -> Self {
        Self {
            stage: EnvStage::Idle,
            level: 0.0,
            time_in_stage: 0.0,
        }
    }

    fn trigger(&mut self) {
        self.stage = EnvStage::Attack;
        self.time_in_stage = 0.0;
        self.level = 0.0;
    }

    fn release(&mut self) {
        self.stage = EnvStage::Release;
        self.time_in_stage = 0.0;
    }

    fn process(&mut self, dt: f64, a: f64, d: f64, s: f64, r: f64) -> f64 {
        self.time_in_stage += dt;

        match self.stage {
            EnvStage::Idle => 0.0,
            EnvStage::Attack => {
                let rate = 1.0 / a.max(0.001);
                self.level += dt * rate;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvStage::Decay;
                    self.time_in_stage = 0.0;
                }
                self.level
            }
            EnvStage::Decay => {
                let rate = 1.0 / d.max(0.001);
                self.level -= dt * rate;
                if self.level <= s {
                    self.level = s;
                    self.stage = EnvStage::Sustain;
                    self.time_in_stage = 0.0;
                }
                self.level
            }
            EnvStage::Sustain => {
                self.level = s;
                s
            }
            EnvStage::Release => {
                let rate = 1.0 / r.max(0.001);
                self.level -= dt * rate;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = EnvStage::Idle;
                }
                self.level
            }
        }
    }
}

struct Voice {
    active: bool,
    note: u8,
    channel: u8,
    frequency: f64,
    velocity: f64,
    pressure: f64,
    timbre: f64,
    pitch_bend: f64, // Semitones

    // Per-oscillator phase
    osc_phases: [f64; 3],               // 0=Osc1, 1=Osc2, 2=Osc3
    unison_phases: [f64; UNISON_COUNT], // Reserved for Super-Saw modulation

    // Envelopes
    amp_env: EnvelopeState,
    filt_env: EnvelopeState,

    // Filter State (ZDF Ladder)
    s1: f64,
    s2: f64,
    s3: f64,
    s4: f64,

    // V-Drift State (Analog Slop)
    drift_val: f64,
    drift_target: f64,

    // Per-voice constant variance (manufacturing tolerance)
    filter_tolerance: f64,
    env_tolerance: f64,

    // One-pole IIR for cutoff smoothing — eliminates zipper noise at high LFO rates
    cutoff_smooth: f64,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum ModSource {
    None,
    LFO,
    Env1, // Amp
    Env2, // Filter
    Vel,
    Key,
    MacroX,
    MacroY,
    Seq,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum ModDest {
    None,
    Pitch1,
    Shape1,
    Pitch2,
    Shape2,
    Cutoff,
    Res,
    Drive,
    LfoRate,
    LfoAmt,
    FxMixDelay,
    FxMixReverb,
    MasterVol,
    FmAmt,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ModSlot {
    pub src: ModSource,
    pub dest: ModDest,
    pub amount: f64, // -1.0 to 1.0
    pub active: bool,
}

impl ModSlot {
    pub fn new() -> Self {
        Self {
            src: ModSource::None,
            dest: ModDest::None,
            amount: 0.0,
            active: true,
        }
    }
}

// Asymmetric Saturation Function (Pre-Filter)
// f(x) = tanh(x + offset) + drift -> Adds even harmonics
#[inline(always)]
fn saturate_tube(x: f64, drive: f64, drift: f64) -> f64 {
    let offset = 0.2 + drift * 0.1;
    let driven = x * (1.0 + drive * 4.0);
    (driven + offset).tanh() - offset.tanh()
}

impl Voice {
    fn new() -> Self {
        Self {
            active: false,
            note: 0,
            channel: 0,
            frequency: 440.0,
            velocity: 0.0,
            pressure: 0.0,
            timbre: 0.0,
            pitch_bend: 0.0,
            osc_phases: [0.0; 3],
            unison_phases: [0.0; UNISON_COUNT],
            amp_env: EnvelopeState::new(),
            filt_env: EnvelopeState::new(),
            s1: 0.0,
            s2: 0.0,
            s3: 0.0,
            s4: 0.0,
            drift_val: 0.0,
            drift_target: 0.0,

            // "Tolerance" is set once on voice creation (simulating physical circuit)
            filter_tolerance: rand::random::<f64>() * 0.04 - 0.02, // +/- 2%
            env_tolerance: rand::random::<f64>() * 0.02 - 0.01,    // +/- 1%

            cutoff_smooth: 2000.0, // Init to default cutoff to avoid pop on first note
        }
    }

    fn trigger(&mut self, note: u8, vel: f64, channel: u8) {
        self.active = true;
        self.note = note;
        self.channel = channel;
        self.frequency = 440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0);
        self.velocity = vel;
        self.pressure = 0.0;
        self.timbre = 0.0;
        self.pitch_bend = 0.0;
        self.osc_phases = [0.0; 3];
        // Randomize unison start phases to avoid laser gun effect
        for i in 0..UNISON_COUNT {
            self.unison_phases[i] = rand::random::<f64>();
        }

        self.amp_env.trigger();
        self.filt_env.trigger();
        self.s1 = 0.0;
        self.s2 = 0.0;
        self.s3 = 0.0;
        self.s4 = 0.0;

        // Randomize initial drift position slightly on key press
        self.drift_val = rand::random::<f64>() * 2.0 - 1.0;
        self.drift_target = rand::random::<f64>() * 2.0 - 1.0;

        // Let the smoother snap to a reasonable starting value to avoid a click.
        // We can't access current cutoff_base here (it's in SynthV1), so we keep
        // whatever value the smoother is at — it will settle in <2ms.
        // Only reset if voice was completely idle (smoother at extreme value).
        if self.cutoff_smooth < 50.0 || self.cutoff_smooth > 19000.0 {
            self.cutoff_smooth = 2000.0;
        }
    }

    fn release(&mut self) {
        self.amp_env.release();
        self.filt_env.release();
    }

    // Updates the Brownian motion random walk for "V-Drift"
    // slew: how fast we move to target (0.0001 - 0.01)
    fn update_drift(&mut self, slew: f64) {
        let diff = self.drift_target - self.drift_val;
        self.drift_val += diff * slew;

        // If close to target, pick new random target
        if diff.abs() < 0.001 {
            self.drift_target = rand::random::<f64>() * 2.0 - 1.0;
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SynthPreset {
    pub name: String,
    pub params: HashMap<String, f64>,
    pub mod_matrix: Vec<ModSlot>,
}

pub struct VOneSynth {
    id: Uuid,
    voices: Vec<Voice>,
    sample_rate: f64,

    // --- PARAMETERS ---

    // Osc 1
    pub osc1_type: Parameter,
    pub osc1_oct: Parameter,
    pub osc1_semi: Parameter,
    pub osc1_detune: Parameter,
    pub osc1_gain: Parameter,
    pub osc1_shape: Parameter, // Morph/Unison/PWM

    // Osc 2
    pub osc2_type: Parameter,
    pub osc2_oct: Parameter,
    pub osc2_semi: Parameter,
    pub osc2_detune: Parameter,
    pub osc2_gain: Parameter,
    pub osc2_shape: Parameter, // Morph/PWM

    // FM
    pub fm_amount: Parameter, // Osc 1 -> Osc 2 FM

    // Osc 3 (Sub/Noise)
    pub osc3_type: Parameter,
    pub osc3_gain: Parameter,

    // Filter
    pub filt_type: Parameter,
    pub cutoff: Parameter,
    pub res: Parameter,
    pub drive: Parameter,
    pub filt_env_amt: Parameter,
    pub filt_kb: Parameter,

    // Amp Envelope
    pub amp_atk: Parameter,
    pub amp_dec: Parameter,
    pub amp_sus: Parameter,
    pub amp_rel: Parameter,

    // Filter Envelope
    pub filt_atk: Parameter,
    pub filt_dec: Parameter,
    pub filt_sus: Parameter,
    pub filt_rel: Parameter,

    // LFO (Global for now)
    pub lfo_rate: Parameter,
    pub lfo_depth: Parameter,

    // Master
    pub master_vol: Parameter,

    // FX: Delay
    pub delay_mix: Parameter,
    pub delay_time: Parameter,
    pub delay_feedback: Parameter,

    // FX: Chorus
    pub chorus_mix: Parameter,
    pub chorus_rate: Parameter,
    pub chorus_depth: Parameter,

    // FX: Reverb
    pub reverb_mix: Parameter,
    pub reverb_size: Parameter,
    pub reverb_damping: Parameter,

    // FX: Distortion
    pub dist_mix: Parameter,
    pub dist_drive: Parameter,
    pub dist_type: Parameter, // tanh, hard, fold

    // FX: Phaser
    pub phaser_mix: Parameter,
    pub phaser_rate: Parameter,
    pub phaser_feedback: Parameter,

    // Mojo (V-One Vision)
    pub warmth: Parameter,        // Saturation + Low Boost
    pub spread: Parameter,        // Stereo Spread
    pub age: Parameter,           // "Character/Vintage" Macro (0.0 to 1.0)
    pub unison_active: Parameter, // "Super-Saw" Mode
    pub macro_x: Parameter,       // Performance Macro X
    pub macro_y: Parameter,       // Performance Macro Y

    // Step Sequencer (8 Steps)
    pub step_1: Parameter,
    pub step_2: Parameter,
    pub step_3: Parameter,
    pub step_4: Parameter,
    pub step_5: Parameter,
    pub step_6: Parameter,
    pub step_7: Parameter,
    pub step_8: Parameter,
    pub seq_target: Parameter, // 0=Cutoff, 1=Pitch, 2=Res, 3=Vol

    // Air EQ State
    air_eq_s1_l: f64,
    air_eq_s1_r: f64,
    // FX State
    delay_buffer: Vec<f64>,
    delay_write_pos: usize,
    chorus_buffer_l: Vec<f64>,
    chorus_buffer_r: Vec<f64>,
    chorus_write_pos: usize,
    chorus_phase: f64,

    // Reverb State (Freeverb style comb/allpass)
    comb_buffers: Vec<Vec<f64>>,
    comb_pos: Vec<usize>,
    allpass_buffers: Vec<Vec<f64>>,
    allpass_pos: Vec<usize>,

    phaser_stages_l: [f64; 4],
    phaser_stages_r: [f64; 4],
    phaser_phase: f64,

    // Arpeggiator State
    pub arp_active: Parameter,
    pub arp_mode: Parameter, // 0=Up, 1=Down, 2=UpDown, 3=Random
    pub arp_rate: Parameter, // 0=1/16, 1=1/8, 2=1/4 ...
    pub arp_oct: Parameter,  // 1-3 octaves
    pub arp_gate: Parameter, // 0.1-0.9

    held_notes: Vec<u8>, // Currently held physical keys
    arp_phase: f64,      // 0.0 to 1.0 (Step phase)
    arp_step_idx: usize,
    arp_triggered_note: Option<u8>,

    // Modulation Matrix
    pub mod_matrix: [ModSlot; 8],
}

impl VOneSynth {
    pub fn new() -> Self {
        let mut voices = Vec::with_capacity(MAX_VOICES);
        for _ in 0..MAX_VOICES {
            voices.push(Voice::new());
        }

        Self {
            id: Uuid::new_v4(),
            voices,
            sample_rate: 48000.0,

            // Osc 1
            osc1_type: Parameter::new("Osc 1 Type", 1.0, 0.0, 4.0), // Saw
            osc1_oct: Parameter::new("Osc 1 Octave", 0.0, -2.0, 2.0),
            osc1_semi: Parameter::new("Osc 1 Semi", 0.0, -12.0, 12.0),
            osc1_detune: Parameter::new("Osc 1 Detune", 0.0, -100.0, 100.0), // Cents
            osc1_gain: Parameter::new("Osc 1 Gain", 0.7, 0.0, 1.0),
            osc1_shape: Parameter::new("Osc 1 Shape", 0.0, 0.0, 1.0),

            // Osc 2
            osc2_type: Parameter::new("Osc 2 Type", 1.0, 0.0, 4.0), // Saw
            osc2_oct: Parameter::new("Osc 2 Octave", 0.0, -2.0, 2.0),
            osc2_semi: Parameter::new("Osc 2 Semi", 0.0, -12.0, 12.0),
            osc2_detune: Parameter::new("Osc 2 Detune", 5.0, -100.0, 100.0), // Slight detune default
            osc2_gain: Parameter::new("Osc 2 Gain", 0.7, 0.0, 1.0),
            osc2_shape: Parameter::new("Osc 2 Shape", 0.0, 0.0, 1.0),

            fm_amount: Parameter::new("FM Amount", 0.0, 0.0, 1.0),

            // Osc 3
            osc3_type: Parameter::new("Osc 3 Type", 0.0, 0.0, 4.0), // Sine (Sub)
            osc3_gain: Parameter::new("Osc 3 Gain", 0.4, 0.0, 1.0),

            // Filter
            filt_type: Parameter::new("Filt Type", 0.0, 0.0, 3.0), // LP, HP, BP, Notch
            cutoff: Parameter::new("Cutoff", 2000.0, 20.0, 20000.0),
            res: Parameter::new("Resonance", 0.2, 0.0, 0.95),
            drive: Parameter::new("Drive", 0.0, 0.0, 10.0),
            filt_env_amt: Parameter::new("Env Amount", 0.5, -1.0, 1.0),
            filt_kb: Parameter::new("Keytrack", 0.5, 0.0, 2.0),

            // Amp Env
            amp_atk: Parameter::new("Amp Atk", 0.01, 0.001, 5.0),
            amp_dec: Parameter::new("Amp Dec", 0.3, 0.001, 5.0),
            amp_sus: Parameter::new("Amp Sus", 0.7, 0.0, 1.0),
            amp_rel: Parameter::new("Amp Rel", 0.5, 0.001, 5.0),

            // Filt Env
            filt_atk: Parameter::new("Filt Atk", 0.05, 0.001, 5.0),
            filt_dec: Parameter::new("Filt Dec", 0.3, 0.001, 5.0),
            filt_sus: Parameter::new("Filt Sus", 0.0, 0.0, 1.0),
            filt_rel: Parameter::new("Filt Rel", 0.5, 0.001, 5.0),

            // LFO
            lfo_rate: Parameter::new("LFO Rate", 5.0, 0.1, 20.0),
            lfo_depth: Parameter::new("LFO Depth", 0.0, 0.0, 1.0), // Vibrato / Filter wobble depth

            // Master
            master_vol: Parameter::new("Master Vol", 0.8, 0.0, 1.0),

            // FX Init
            delay_mix: Parameter::new("Delay Mix", 0.0, 0.0, 1.0),
            delay_time: Parameter::new("Delay Time", 0.4, 0.01, 2.0),
            delay_feedback: Parameter::new("Delay Feed", 0.3, 0.0, 0.95),

            chorus_mix: Parameter::new("Chorus Mix", 0.0, 0.0, 1.0),
            chorus_rate: Parameter::new("Chorus Rate", 0.5, 0.1, 5.0),
            chorus_depth: Parameter::new("Chorus Depth", 0.5, 0.0, 1.0),

            delay_buffer: vec![0.0; 192000],
            delay_write_pos: 0,
            chorus_buffer_l: vec![0.0; 4096],
            chorus_buffer_r: vec![0.0; 4096],
            chorus_write_pos: 0,
            chorus_phase: 0.0,

            // Reverb Init
            reverb_mix: Parameter::new("Reverb Mix", 0.0, 0.0, 1.0),
            reverb_size: Parameter::new("Reverb Size", 0.5, 0.0, 1.0),
            reverb_damping: Parameter::new("Reverb Damp", 0.5, 0.0, 1.0),

            comb_buffers: vec![
                vec![0.0; 1116],
                vec![0.0; 1188],
                vec![0.0; 1277],
                vec![0.0; 1356],
            ],
            comb_pos: vec![0, 0, 0, 0],
            allpass_buffers: vec![vec![0.0; 556], vec![0.0; 441]],
            allpass_pos: vec![0, 0],

            // Distortion Init
            dist_mix: Parameter::new("Dist Mix", 0.0, 0.0, 1.0),
            dist_drive: Parameter::new("Dist Drive", 1.0, 1.0, 20.0),
            dist_type: Parameter::new("Dist Type", 0.0, 0.0, 2.0), // tanh, hard, fold

            // Phaser Init
            phaser_mix: Parameter::new("Phaser Mix", 0.0, 0.0, 1.0),
            phaser_rate: Parameter::new("Phaser Rate", 0.5, 0.1, 10.0),
            phaser_feedback: Parameter::new("Phaser Feed", 0.5, 0.0, 0.95),
            phaser_stages_l: [0.0; 4],
            phaser_stages_r: [0.0; 4],
            phaser_phase: 0.0,

            // Mojo Init
            warmth: Parameter::new("Warmth", 0.0, 0.0, 1.0),
            spread: Parameter::new("Spread", 0.0, 0.0, 1.0),
            age: Parameter::new("Age/Char", 0.0, 0.0, 1.0),
            unison_active: Parameter::new("Super Saw", 0.0, 0.0, 1.0),
            macro_x: Parameter::new("Macro X", 0.0, 0.0, 1.0),
            macro_y: Parameter::new("Macro Y", 0.0, 0.0, 1.0),

            step_1: Parameter::new("Step 1", 0.5, 0.0, 1.0),
            step_2: Parameter::new("Step 2", 0.5, 0.0, 1.0),
            step_3: Parameter::new("Step 3", 0.5, 0.0, 1.0),
            step_4: Parameter::new("Step 4", 0.5, 0.0, 1.0),
            step_5: Parameter::new("Step 5", 0.5, 0.0, 1.0),
            step_6: Parameter::new("Step 6", 0.5, 0.0, 1.0),
            step_7: Parameter::new("Step 7", 0.5, 0.0, 1.0),
            step_8: Parameter::new("Step 8", 0.5, 0.0, 1.0),
            seq_target: Parameter::new("Seq Target", 0.0, 0.0, 3.0),

            arp_active: Parameter::new("Arp On", 0.0, 0.0, 1.0),
            arp_mode: Parameter::new("Arp Mode", 0.0, 0.0, 3.0),
            arp_rate: Parameter::new("Arp Rate", 2.0, 0.0, 4.0), // 1/16 default
            arp_oct: Parameter::new("Arp Oct", 1.0, 1.0, 3.0),
            arp_gate: Parameter::new("Arp Gate", 0.5, 0.1, 1.0),

            held_notes: Vec::new(),
            arp_phase: 0.0,
            arp_step_idx: 0,
            arp_triggered_note: None,

            mod_matrix: [ModSlot::new(); 8],

            air_eq_s1_l: 0.0,
            air_eq_s1_r: 0.0,
        }
    }
}

impl AudioProcessor for VOneSynth {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        let playhead = context.playhead;
        self.sample_rate = sample_rate;
        let dt = 1.0 / sample_rate;
        let frames = buffer.frames;

        let o1_type: OscType = self.osc1_type.get_value_at(playhead).into();
        let o1_freq_mult = 2.0f64.powf(
            self.osc1_oct.get_value_at(playhead) + self.osc1_semi.get_value_at(playhead) / 12.0,
        );
        let o1_detune = self.osc1_detune.get_value_at(playhead);
        let o1_gain = self.osc1_gain.get_value_at(playhead);
        let o1_shape = self.osc1_shape.get_value_at(playhead);

        let o2_type: OscType = self.osc2_type.get_value_at(playhead).into();
        let o2_freq_mult = 2.0f64.powf(
            self.osc2_oct.get_value_at(playhead) + self.osc2_semi.get_value_at(playhead) / 12.0,
        );
        let o2_detune = self.osc2_detune.get_value_at(playhead);
        let o2_gain = self.osc2_gain.get_value_at(playhead);
        let o2_shape = self.osc2_shape.get_value_at(playhead);

        let o3_type: OscType = self.osc3_type.get_value_at(playhead).into();
        let o3_gain = self.osc3_gain.get_value_at(playhead);

        let f_type = self.filt_type.get_value_at(playhead);
        let cutoff_base = self.cutoff.get_value_at(playhead);
        let res_base = self.res.get_value_at(playhead);
        let drive_base = self.drive.get_value_at(playhead);
        let env_amt = self.filt_env_amt.get_value_at(playhead);
        let filt_kb = self.filt_kb.get_value_at(playhead);

        // LFO
        let lfo_rate = self.lfo_rate.get_value_at(playhead);
        let lfo_depth = self.lfo_depth.get_value_at(playhead);

        let a_atk = self.amp_atk.get_value_at(playhead);
        let a_dec = self.amp_dec.get_value_at(playhead);
        let a_sus = self.amp_sus.get_value_at(playhead);
        let a_rel = self.amp_rel.get_value_at(playhead);

        let f_atk = self.filt_atk.get_value_at(playhead);
        let f_dec = self.filt_dec.get_value_at(playhead);
        let f_sus = self.filt_sus.get_value_at(playhead);
        let f_rel = self.filt_rel.get_value_at(playhead);

        let master = self.master_vol.get_value_at(playhead);

        // FX Params
        let d_time = self.delay_time.get_value_at(playhead);
        let d_feed = self.delay_feedback.get_value_at(playhead);
        let c_mix = self.chorus_mix.get_value_at(playhead);
        let c_rate = self.chorus_rate.get_value_at(playhead);
        let c_depth = self.chorus_depth.get_value_at(playhead);

        let mut r_mix = self.reverb_mix.get_value_at(playhead);
        let r_size = self.reverb_size.get_value_at(playhead) * 0.28 + 0.7; // Scale to 0.7 - 0.98
        let r_damp = self.reverb_damping.get_value_at(playhead);

        let dist_mix = self.dist_mix.get_value_at(playhead);
        let dist_drive = self.dist_drive.get_value_at(playhead);
        let dist_type = self.dist_type.get_value_at(playhead);

        let p_mix = self.phaser_mix.get_value_at(playhead);
        let p_rate = self.phaser_rate.get_value_at(playhead);
        let p_feed = self.phaser_feedback.get_value_at(playhead);

        // Mojo
        let m_warmth = self.warmth.get_value_at(playhead);
        let m_spread = self.spread.get_value_at(playhead);
        let m_age = self.age.get_value_at(playhead);
        let unison_on = self.unison_active.get_value_at(playhead) > 0.5;

        // Arp
        let arp_on = self.arp_active.get_value_at(playhead) > 0.5;
        let arp_mode = self.arp_mode.get_value_at(playhead).round() as usize;
        let arp_rate_param = self.arp_rate.get_value_at(playhead);
        let arp_oct = self.arp_oct.get_value_at(playhead).round() as i32;
        let arp_gate_len = self.arp_gate.get_value_at(playhead);

        // --- MODULATION MATRIX CALCULATION ---

        let mut mods = [0.0f64; 14]; // Map ModDest enum index to value

        let lfo_phase_0to1 = (playhead as f64 / sample_rate * lfo_rate).fract();
        let src_lfo = (lfo_phase_0to1 * 2.0 * PI).sin(); // Bipolar -1..1

        let seq_phase = (playhead as f64 / sample_rate * lfo_rate).fract();
        let step_idx = (seq_phase * 8.0) as usize;
        let src_seq = match step_idx {
            0 => self.step_1.get_value_at(playhead),
            1 => self.step_2.get_value_at(playhead),
            2 => self.step_3.get_value_at(playhead),
            3 => self.step_4.get_value_at(playhead),
            4 => self.step_5.get_value_at(playhead),
            5 => self.step_6.get_value_at(playhead),
            6 => self.step_7.get_value_at(playhead),
            _ => self.step_8.get_value_at(playhead),
        };

        let src_macro_x = self.macro_x.get_value_at(playhead);
        let src_macro_y = self.macro_y.get_value_at(playhead);

        // Calculate Global Matrix Slots
        for slot in &self.mod_matrix {
            if !slot.active || slot.amount.abs() < 0.001 {
                continue;
            }

            let src_val = match slot.src {
                ModSource::LFO => src_lfo,
                ModSource::MacroX => src_macro_x,
                ModSource::MacroY => src_macro_y,
                ModSource::Seq => src_seq,
                _ => 0.0, // Per-voice sources handled inside loop
            };

            if src_val != 0.0 {
                let dest_idx = match slot.dest {
                    ModDest::Pitch1 => 0,
                    ModDest::Shape1 => 1,
                    ModDest::Pitch2 => 2,
                    ModDest::Shape2 => 3,
                    ModDest::Cutoff => 4,
                    ModDest::Res => 5,
                    ModDest::Drive => 6,
                    ModDest::LfoRate => 7,
                    ModDest::LfoAmt => 8,
                    ModDest::FxMixDelay => 9,
                    ModDest::FxMixReverb => 10,
                    ModDest::MasterVol => 11,
                    ModDest::FmAmt => 12,
                    _ => 999,
                };

                if dest_idx < 14 {
                    mods[dest_idx] += src_val * slot.amount;
                }
            }
        }

        // Apply Global Mods
        let d_mix = (self.delay_mix.get_value_at(playhead) + mods[9]).clamp(0.0, 1.0);
        r_mix = (r_mix + mods[10]).clamp(0.0, 1.0);
        let fm_amt = (self.fm_amount.get_value_at(playhead) + mods[12]).clamp(0.0, 1.0);

        // Initial LFO Rate might be modulated too, but we use it for phase update next block
        // so maybe apply it to `lfo_rate` var?
        // Let's leave lfo_rate 'pure' for phase calc this block, and apply mod for *next* block or just ignore for now.
        // Effective LFO Rate for Arp?

        // 1/16 = 0.25 beats? No, BPM specific.
        // Assuming BPM=120 for now in pure synth test, but engine passes dt.
        // Let's rely on lfo_rate as BPM Sync approximation or internal clock.
        // Actually, let's use lfo_rate as Master Clock for now since accurate BPM isn't passed to process()
        // (Wait, Logic passes parameters, but BPM is global? No, LFO Rate is Hz or Sync.
        // For accurate Arp, we need beats. `playhead` is samples.
        // We'll use LFO Rate as the driver for Arp Speed for now to keep it synced with Sequencer.)

        // Rate: Use arp_rate as multiplier (0.5 = half speed, 1.0 = normal, 2.0 = double)
        // Default arp_rate is 2.0? No, let's say 1.0 is default.
        // Actually earlier I set default to 2.0 (lines 471). Let's assume standard multipliers.
        let effective_rate = lfo_rate * (0.5 * arp_rate_param);
        let beat_time = (playhead as f64 / sample_rate * effective_rate).fract(); // 0..1 per LFO cycle

        if arp_on {
            // Trigger Logic
            // If arp_phase wrapped around, trigger next note
            // Simple edge detection on beat_time?
            // "beat_time" is saw 0..1 at LFO rate.
            // Arp Steps per LFO cycle? Let's say LFO = 1 Bar? or 1 Beat?
            // Let's assume LFO Rate IS the Arp Rate (simpler).

            // Pulse:
            if beat_time < self.arp_phase {
                // Wrapped
                // Next Step!
                if !self.held_notes.is_empty() {
                    let mut notes_to_play = self.held_notes.clone();
                    notes_to_play.sort(); // Up Mode Default

                    if arp_mode == 1 {
                        // Down
                        notes_to_play.reverse();
                    } else if arp_mode == 3 { // Random
                         // simplistic shuffle? just pick random index
                         // actually sort is fine for determinism, we pick index.
                    }

                    // Octave expansion
                    let base_len = notes_to_play.len();
                    let total_len = base_len * (arp_oct as usize);

                    self.arp_step_idx = (self.arp_step_idx + 1) % total_len;

                    let mut note_idx = self.arp_step_idx % base_len;
                    let oct_offset = (self.arp_step_idx / base_len) as i32;

                    if arp_mode == 3 {
                        note_idx = rand::random::<usize>() % base_len;
                    }

                    let raw_note = notes_to_play[note_idx];
                    let final_note = (raw_note as i32 + oct_offset * 12).clamp(0, 127) as u8;

                    // Trigger Voice
                    self.trigger_internal(final_note, 0.8); // Fixed velocity for now
                    self.arp_triggered_note = Some(final_note);
                }
            }

            // Gate Logic: Note Off
            if beat_time > arp_gate_len && self.arp_triggered_note.is_some() {
                // Kill the arp note
                let n = self.arp_triggered_note.unwrap();
                for v in &mut self.voices {
                    if v.active && v.note == n {
                        v.release();
                    }
                }
                self.arp_triggered_note = None;
            }

            self.arp_phase = beat_time;
        }

        for i in 0..frames {
            // LFO Calculation (Global Sine for now)
            let time = (playhead + i as u64) as f64 / sample_rate;
            let lfo_val = (time * lfo_rate * 2.0 * PI).sin() * lfo_depth;

            // Global noise floor - scales with AGE
            // Vintage mode behavior is now absorbed into AGE
            let noise_floor = if m_age > 0.1 {
                (rand::random::<f64>() * 2.0 - 1.0) * 0.0001 * m_age // Up to -80dB roughly
            } else {
                0.0
            };

            let mut out_l = noise_floor;
            let mut out_r = noise_floor;
            // let unison_spread = self.spread.get_value_at(playhead); // Unused variable removed

            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }

                // Per-Voice Sources
                let src_env1 = voice.amp_env.level;
                let src_env2 = voice.filt_env.level;
                let src_vel = voice.velocity;
                let src_key = (voice.note as f64) / 127.0;

                // Copy Global Mods
                let mut v_mods = mods;

                // Apply Per-Voice Matrix Slots
                for slot in &self.mod_matrix {
                    if !slot.active || slot.amount.abs() < 0.001 {
                        continue;
                    }

                    let src_val = match slot.src {
                        ModSource::Env1 => src_env1,
                        ModSource::Env2 => src_env2,
                        ModSource::Vel => src_vel,
                        ModSource::Key => src_key,
                        _ => 0.0,
                    };

                    if src_val != 0.0 {
                        let dest_idx = match slot.dest {
                            ModDest::Pitch1 => 0,
                            ModDest::Shape1 => 1,
                            ModDest::Pitch2 => 2,
                            ModDest::Shape2 => 3,
                            ModDest::Cutoff => 4,
                            ModDest::Res => 5,
                            ModDest::Drive => 6,
                            ModDest::LfoRate => 7,
                            ModDest::LfoAmt => 8,
                            ModDest::FxMixDelay => 9,
                            ModDest::FxMixReverb => 10,
                            ModDest::MasterVol => 11,
                            ModDest::FmAmt => 12,
                            _ => 999,
                        };
                        if dest_idx < 14 {
                            v_mods[dest_idx] += src_val * slot.amount;
                        }
                    }
                }

                // Apply Mods to Voice Parameters
                // Base values + mods
                let note_freq = voice.frequency * 2.0f64.powf(v_mods[0]);

                let o1_shape_v = (o1_shape + v_mods[1]).clamp(0.0, 1.0);
                let o2_shape_v = (o2_shape + v_mods[3]).clamp(0.0, 1.0);

                let v_cutoff = cutoff_base * 2.0f64.powf(v_mods[4] * 5.0 + env_amt * src_env2);
                let v_res = (res_base + v_mods[5]).clamp(0.0, 1.0);
                let v_drive = (drive_base + v_mods[6]).clamp(0.0, 10.0);
                let v_fm = (fm_amt + v_mods[12]).clamp(0.0, 1.0);

                // 1. Envelopes
                // Modify Envelope rates based on Age (Sluggish caps)
                let age_slew = 1.0 + m_age * 0.5; // up to 1.5x slower
                let amp = voice.amp_env.process(
                    dt,
                    a_atk * age_slew,
                    a_dec * age_slew,
                    a_sus,
                    a_rel * age_slew,
                );

                // Add manufacturing tolerance (env_tolerance)
                let f_atk_eff = f_atk * (1.0 + voice.env_tolerance * m_age * 10.0);

                let _f_env = voice.filt_env.process(dt, f_atk_eff, f_dec, f_sus, f_rel);

                if voice.amp_env.stage == EnvStage::Idle {
                    voice.active = false;
                    continue;
                }

                // Update Drift
                // Drifts more with Age
                let drift_speed = 0.002 + m_age * 0.005;
                voice.update_drift(drift_speed);

                // Drift Modulation (Pitch and Cutoff)
                let drift_pitch = voice.drift_val * (20.0 * m_warmth + 100.0 * m_age); // +/- 1 semitone at max age
                let drift_cutoff = voice.drift_val * (400.0 * m_warmth + 1000.0 * m_age);

                // 2. Oscillators
                // Apply LFO to Pitch (Vibrato)
                let f1_bend = o1_detune + lfo_val * 20.0 + voice.pitch_bend * 100.0 + drift_pitch;
                // USE note_freq from matrix calculation
                let f1_base = note_freq * o1_freq_mult * 2.0f64.powf(f1_bend / 1200.0);

                // --- OSC 1 Generation (With Unison/Shape) ---
                voice.osc_phases[0] += f1_base * dt;
                voice.osc_phases[0] -= voice.osc_phases[0].floor();

                let mut osc1_out =
                    poly_blep_osc(o1_type, voice.osc_phases[0], f1_base * dt, o1_shape_v);

                if unison_on && o1_type == OscType::Saw {
                    // Hyper-Stack: Add 7 detuned saws
                    let spread_amt = 0.02 * (1.0 + m_spread); // Detune amount
                    let mut stack = 0.0;
                    for u in 0..UNISON_COUNT {
                        let u_detune = (u as f64 - (UNISON_COUNT as f64 / 2.0)) * spread_amt;
                        let f_u = f1_base * (1.0 + u_detune);
                        voice.unison_phases[u] += f_u * dt;
                        voice.unison_phases[u] -= voice.unison_phases[u].floor();
                        stack += poly_blep_osc(OscType::Saw, voice.unison_phases[u], f_u * dt, 0.0);
                    }
                    // Blend in stack
                    osc1_out = osc1_out * 0.5 + stack * 0.15;
                }

                // --- OSC 2 Generation ---
                // Apply FM from Osc 1 -> Osc 2 freq
                let fm_mod = osc1_out * v_fm * 2000.0; // 2kHz mod range
                let f2_bend = o2_detune + drift_pitch;
                let f2_base = note_freq * o2_freq_mult * 2.0f64.powf(f2_bend / 1200.0) + fm_mod;

                voice.osc_phases[1] += f2_base * dt;
                voice.osc_phases[1] -= voice.osc_phases[1].floor();

                let osc2_out =
                    poly_blep_osc(o2_type, voice.osc_phases[1], f2_base * dt, o2_shape_v);

                // --- OSC 3 (Sub) ---
                let f3_base = note_freq * 0.5; // Sub Octave
                voice.osc_phases[2] += f3_base * dt;
                voice.osc_phases[2] -= voice.osc_phases[2].floor();

                let mut osc3_out = poly_blep_osc(o3_type, voice.osc_phases[2], f3_base * dt, 0.0);

                // If noise selected
                if o3_type == OscType::Noise {
                    osc3_out = rand::random::<f64>() * 2.0 - 1.0;
                }

                // Mix Oscillators
                let pre_filter = osc1_out * o1_gain + osc2_out * o2_gain + osc3_out * o3_gain * 0.6;

                // 3. Filter
                // Apply Envelope + Keytracking + Drift + Cutoff Knob
                let kt_val = (voice.note as f64 - 60.0) * 0.01 * filt_kb;
                // One-pole IIR smoothing: alpha=0.0005 gives ~1ms at 48kHz.
                // Prevents zipper noise when LFO modulates cutoff at fast/audio rates.
                let cutoff_target = (v_cutoff
                    * 2.0f64.powf(kt_val + drift_cutoff / 1200.0)
                    * voice.filter_tolerance)
                    .clamp(20.0, 20000.0);
                voice.cutoff_smooth += 0.0005 * (cutoff_target - voice.cutoff_smooth);
                let f_hz = voice.cutoff_smooth;

                // Tube Drive (Pre-Filter)
                let driven_input = saturate_tube(pre_filter, v_drive, voice.drift_val);

                // Ladder Filter
                // Apply Age to Resonance
                let eff_res = (v_res * (1.0 - m_age * 0.3)).clamp(0.0, 0.99);

                let g = (PI * f_hz * dt).tan();
                let k = eff_res * 4.0;
                let h = 1.0 / (1.0 + g);

                let v0 = driven_input;
                let feedback = (v0 - k * voice.s4) * h;

                let v1 = (feedback + voice.s1) * g * h;
                let res1 = v1 + voice.s1;
                voice.s1 = flush_denormal_f64(res1 + v1);

                let v2 = (v1 + voice.s2) * g * h;
                let res2 = v2 + voice.s2;
                voice.s2 = flush_denormal_f64(res2 + v2);

                let v3 = (v2 + voice.s3) * g * h;
                let res3 = v3 + voice.s3;
                voice.s3 = flush_denormal_f64(res3 + v3);

                let v4 = (v3 + voice.s4) * g * h;
                let res4 = v4 + voice.s4;
                voice.s4 = flush_denormal_f64(res4 + v4);

                let mut filtered = res4;
                if (1.0..2.0).contains(&f_type) {
                    filtered = driven_input - res4; // HP
                } else if f_type >= 2.0 {
                    filtered = res2 - res4; // BP
                }

                // 4. Output
                let voice_out = filtered * amp * voice.velocity * (1.0 + voice.pressure * 0.5);

                // Stereo Voice Spreading (Age adds randomness to pan)
                let mut pan = 0.0;
                if m_spread > 0.0 || m_age > 0.0 {
                    let note_factor = ((voice.note as f64 - 48.0) / 48.0).clamp(0.0, 1.0);
                    pan = voice.drift_val * (0.5 + m_age) // Age makes pan unstable
                        + ((voice.note % 2) as f64 * 2.0 - 1.0) * 0.3 * note_factor;
                    pan *= m_spread + m_age * 0.2;
                }

                let pan_l = (1.0 - pan).clamp(0.0, 2.0) * 0.5;
                let pan_r = (1.0 + pan).clamp(0.0, 2.0) * 0.5;

                out_l += voice_out * pan_l;
                out_r += voice_out * pan_r;
            }

            // --- FX ---

            // Chorus
            if c_mix > 0.001 {
                self.chorus_buffer_l[self.chorus_write_pos] = out_l;
                self.chorus_buffer_r[self.chorus_write_pos] = out_r;

                self.chorus_phase += c_rate * dt;
                if self.chorus_phase > 1.0 {
                    self.chorus_phase -= 1.0;
                }
                let lfo = (self.chorus_phase * 2.0 * PI).sin();
                let delay_mod = 480.0 + lfo * 200.0 * c_depth;

                let buf_len = self.chorus_buffer_l.len();
                let read_idx_float =
                    (self.chorus_write_pos as f64 - delay_mod + buf_len as f64) % buf_len as f64;
                let idx_i = read_idx_float.floor() as usize;
                let frac = read_idx_float - read_idx_float.floor();
                let idx_next = (idx_i + 1) % buf_len;

                let wet_l = self.chorus_buffer_l[idx_i] * (1.0 - frac)
                    + self.chorus_buffer_l[idx_next] * frac;
                let wet_r = self.chorus_buffer_r[idx_i] * (1.0 - frac)
                    + self.chorus_buffer_r[idx_next] * frac;

                out_l = out_l * (1.0 - c_mix) + wet_l * c_mix;
                out_r = out_r * (1.0 - c_mix) + wet_r * c_mix;

                self.chorus_write_pos = (self.chorus_write_pos + 1) % buf_len;
            }

            // Phaser
            if p_mix > 0.001 {
                self.phaser_phase += p_rate * dt;
                self.phaser_phase -= self.phaser_phase.floor();
                let lfo_val = (self.phaser_phase * 2.0 * PI).sin() * 0.5 + 0.5;

                let phaser_freq = 200.0 * 20.0f64.powf(lfo_val);
                let g = (PI * phaser_freq * dt).tan();
                let coeff = (g - 1.0) / (g + 1.0);

                let mut pl = out_l + self.phaser_stages_l[3] * p_feed;
                for stage in &mut self.phaser_stages_l {
                    let y = coeff * pl + *stage;
                    *stage = pl - coeff * y;
                    pl = y;
                }
                out_l = out_l * (1.0 - p_mix) + pl * p_mix;

                let mut pr = out_r + self.phaser_stages_r[3] * p_feed;
                for stage in &mut self.phaser_stages_r {
                    let y = coeff * pr + *stage;
                    *stage = pr - coeff * y;
                    pr = y;
                }
                out_r = out_r * (1.0 - p_mix) + pr * p_mix;
            }

            // Delay
            if d_mix > 0.001 {
                let delay_samps = (d_time * sample_rate).round() as usize;
                let buf_len = self.delay_buffer.len();
                let delay_samps = delay_samps.clamp(1, buf_len - 1);

                let read_pos = if self.delay_write_pos >= delay_samps {
                    self.delay_write_pos - delay_samps
                } else {
                    self.delay_write_pos + buf_len - delay_samps
                };

                let delayed = self.delay_buffer[read_pos];
                let input_sum = (out_l + out_r) * 0.5;
                let feed_val = (input_sum + delayed * d_feed).tanh();
                self.delay_buffer[self.delay_write_pos] = feed_val;
                self.delay_write_pos = (self.delay_write_pos + 1) % buf_len;

                out_l += delayed * d_mix;
                out_r += delayed * d_mix;
            }

            // Reverb
            if r_mix > 0.001 {
                let input = (out_l + out_r) * 0.5 * 0.2;
                let mut comb_out = 0.0;
                for (j, buf) in self.comb_buffers.iter_mut().enumerate() {
                    let pos = self.comb_pos[j];
                    let delayed = buf[pos];
                    buf[pos] = input + delayed * (r_size * (1.0 - r_damp * 0.2));
                    self.comb_pos[j] = (pos + 1) % buf.len();
                    comb_out += delayed;
                }
                let mut ap_out = comb_out;
                for (j, buf) in self.allpass_buffers.iter_mut().enumerate() {
                    let pos = self.allpass_pos[j];
                    let delayed = buf[pos];
                    let g = 0.5;
                    let v = ap_out + delayed * g;
                    let out = delayed - v * g;
                    buf[pos] = v;
                    self.allpass_pos[j] = (pos + 1) % buf.len();
                    ap_out = out;
                }
                out_l += ap_out * r_mix;
                out_r += ap_out * r_mix;
            }

            // Distortion
            if dist_mix > 0.001 {
                let mut dl = out_l * dist_drive;
                let mut dr = out_r * dist_drive;
                if dist_type < 1.0 {
                    dl = dl.tanh();
                    dr = dr.tanh();
                } else if dist_type < 2.0 {
                    dl = dl.clamp(-1.0, 1.0);
                    dr = dr.clamp(-1.0, 1.0);
                } else {
                    // Foldback
                    if dl > 1.0 {
                        dl = 2.0 - dl;
                    } else if dl < -1.0 {
                        dl = -2.0 - dl;
                    }
                    if dr > 1.0 {
                        dr = 2.0 - dr;
                    } else if dr < -1.0 {
                        dr = -2.0 - dr;
                    }
                    dl = dl.clamp(-1.0, 1.0);
                    dr = dr.clamp(-1.0, 1.0);
                }
                out_l = out_l * (1.0 - dist_mix) + dl * dist_mix;
                out_r = out_r * (1.0 - dist_mix) + dr * dist_mix;
            }

            // Air EQ
            if m_warmth > 0.01 || m_age > 0.01 {
                let air_freq = 12000.0;
                let g_air = (PI * air_freq * dt).tan();
                let h_air = 1.0 / (1.0 + g_air);

                let v_l = (out_l - self.air_eq_s1_l) * g_air * h_air;
                let lp_l = v_l + self.air_eq_s1_l;
                self.air_eq_s1_l = lp_l + v_l;
                let highs_l = out_l - lp_l;

                let v_r = (out_r - self.air_eq_s1_r) * g_air * h_air;
                let lp_r = v_r + self.air_eq_s1_r;
                self.air_eq_s1_r = lp_r + v_r;
                let highs_r = out_r - lp_r;

                let boost = 1.0 + m_warmth * 0.5;
                out_l += highs_l * (boost - 1.0);
                out_r += highs_r * (boost - 1.0);
            }

            buffer.channels_data[0][i] += out_l.clamp(-5.0, 5.0) * master;
            buffer.channels_data[1][i] += out_r.clamp(-5.0, 5.0) * master;
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    // MIDI Event handling (Simplified for brevity, same as before)
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_midi_event(&mut self, status: u8, data1: u16, data2: u32) {
        let cmd = status & 0xF0;
        let channel = status & 0x0F;
        match cmd {
            0x90 => {
                let vel = data2 as f64 / 127.0;
                let note = data1 as u8;

                if vel > 0.0 {
                    // Check Arp State (Hack: Read param current value - inefficient but works for this context)
                    // Better: check a cached bool updated in process?
                    // process() runs AFTER midi event usually? Or parallel.
                    // For safety, let's treat Arp as "Always capable" and check param value?
                    // No, parameters are time-variant.
                    // Let's assume Arp is handled in process() exclusively IF active,
                    // BUT we need to know if it's active here to inhibit standard triggering.
                    // Let's check the parameter value at time 0? Or use a cached flag.

                    // Allow enabling ARP via param. For now, let's check holding logic.
                    // If we hold notes, process() will see them.
                    if !self.held_notes.contains(&note) {
                        self.held_notes.push(note);
                    }

                    // If Arp is OFF (we can't easily check param here without time context),
                    // we trigger normally.
                    // To solve this properly: We always trigger normally here.
                    // AND update held_notes.
                    // IF Arp is ON in process(), it will trigger its own notes.
                    // PROBLEM: We get double notes (Pad + Arp).
                    // SOLUTION for MVP: We always trigger. "Arp" is an ADDITIVE layer (Sequenced Arp).
                    // OR: We check a flag set by process().

                    // Let's make Arp purely additive for now to avoid "Silence" bug if param reading is hard.
                    // Actually, Vibe's params are efficient.
                    if self.arp_active.get_value_at(0) < 0.5 {
                        self.trigger_internal(note, vel);
                    }
                } else {
                    // Note Off
                    if let Some(pos) = self.held_notes.iter().position(|&x| x == note) {
                        self.held_notes.remove(pos);
                    }

                    if self.arp_active.get_value_at(0) < 0.5 {
                        for v in &mut self.voices {
                            if v.active && v.note == note && v.channel == channel {
                                v.release();
                            }
                        }
                    } else {
                        // Arp is On: if held_notes empty, kill all voices?
                        if self.held_notes.is_empty() {
                            for v in &mut self.voices {
                                v.release();
                            }
                        }
                    }
                }
            }
            0x80 => {
                for v in &mut self.voices {
                    if v.active && v.note == data1 as u8 && v.channel == channel {
                        v.release();
                    }
                }
            }
            0xE0 => {
                let bend_raw = (data1 as u32 | (data2 << 7)) as f64;
                let bend_semi = (bend_raw - 8192.0) / 8192.0 * 48.0;
                for v in &mut self.voices {
                    if v.active && v.channel == channel {
                        v.pitch_bend = bend_semi;
                    }
                }
            }
            _ => {}
        }
    }

    fn on_mpe_event(&mut self, event: crate::engine::mpe_handler::MpeOutputEvent) {
        use crate::engine::mpe_handler::MpeOutputEvent;
        match event {
            MpeOutputEvent::NoteOn(chan, note, vel) => {
                self.trigger_internal(note, vel as f64 / 127.0);
                // Assign channel to the voice
                for v in &mut self.voices {
                    if v.active && v.note == note {
                        v.channel = chan;
                    }
                }
            }
            MpeOutputEvent::NoteOff(chan, note, _vel) => {
                for v in &mut self.voices {
                    if v.active && v.note == note && v.channel == chan {
                        v.release();
                    }
                }
            }
            MpeOutputEvent::PitchBend(chan, note, bend) => {
                let bend_semi = (bend as f64) / 8192.0 * 48.0; // Assume 48 semi range
                for v in &mut self.voices {
                    if v.active && v.note == note && v.channel == chan {
                        v.pitch_bend = bend_semi;
                    }
                }
            }
            MpeOutputEvent::Pressure(chan, note, pressure) => {
                let p = pressure as f64 / 127.0;
                for v in &mut self.voices {
                    if v.active && v.note == note && v.channel == chan {
                        v.pressure = p;
                    }
                }
            }
            MpeOutputEvent::Timbre(chan, note, timbre) => {
                let t = timbre as f64 / 127.0;
                for v in &mut self.voices {
                    if v.active && v.note == note && v.channel == chan {
                        v.timbre = t;
                    }
                }
            }
        }
    }

    fn on_midi2_event(&mut self, event: crate::engine::midi2_support::Midi2Output) {
        use crate::engine::midi2_support::Midi2Output;
        match event {
            Midi2Output::Note {
                on,
                channel,
                note,
                velocity,
                ..
            } => {
                if on {
                    // 16-bit velocity support
                    let vel_f = velocity as f64 / 65535.0;
                    self.trigger_internal(note, vel_f);
                    for v in &mut self.voices {
                        if v.active && v.note == note {
                            v.channel = channel;
                        }
                    }
                } else {
                    for v in &mut self.voices {
                        if v.active && v.note == note && v.channel == channel {
                            v.release();
                        }
                    }
                }
            }
            Midi2Output::PitchBend { channel, value } => {
                // 32-bit pitch bend
                let bend_semi = (value as f64 - 2147483648.0) / 2147483648.0 * 48.0;
                for v in &mut self.voices {
                    if v.active && v.channel == channel {
                        v.pitch_bend = bend_semi;
                    }
                }
            }
            Midi2Output::ControlChange {
                channel: _,
                index: _,
                value: _,
            } => {
                // High-res CC handling could be mapped to parameters
                // For now, let's just log or ignore
            }
            _ => {}
        }
    }

    fn name(&self) -> String {
        "VOne Synth".to_string()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.osc1_type,
            &mut self.osc1_oct,
            &mut self.osc1_semi,
            &mut self.osc1_detune,
            &mut self.osc1_gain,
            &mut self.osc1_shape,
            &mut self.osc2_type,
            &mut self.osc2_oct,
            &mut self.osc2_semi,
            &mut self.osc2_detune,
            &mut self.osc2_gain,
            &mut self.osc2_shape,
            &mut self.fm_amount,
            &mut self.osc3_type,
            &mut self.osc3_gain,
            &mut self.filt_type,
            &mut self.cutoff,
            &mut self.res,
            &mut self.drive,
            &mut self.filt_env_amt,
            &mut self.filt_kb,
            &mut self.amp_atk,
            &mut self.amp_dec,
            &mut self.amp_sus,
            &mut self.amp_rel,
            &mut self.filt_atk,
            &mut self.filt_dec,
            &mut self.filt_sus,
            &mut self.filt_rel,
            &mut self.lfo_rate,
            &mut self.lfo_depth,
            &mut self.master_vol,
            &mut self.delay_mix,
            &mut self.delay_time,
            &mut self.delay_feedback,
            &mut self.chorus_mix,
            &mut self.chorus_rate,
            &mut self.chorus_depth,
            &mut self.reverb_mix,
            &mut self.reverb_size,
            &mut self.reverb_damping,
            &mut self.dist_mix,
            &mut self.dist_drive,
            &mut self.dist_type,
            &mut self.phaser_mix,
            &mut self.phaser_rate,
            &mut self.phaser_feedback,
            &mut self.warmth,
            &mut self.spread,
            &mut self.age,
            &mut self.unison_active,
            &mut self.macro_x,
            &mut self.macro_y,
            &mut self.step_1,
            &mut self.step_2,
            &mut self.step_3,
            &mut self.step_4,
            &mut self.step_5,
            &mut self.step_6,
            &mut self.step_7,
            &mut self.step_8,
            &mut self.seq_target,
            &mut self.arp_active,
            &mut self.arp_mode,
            &mut self.arp_rate,
            &mut self.arp_oct,
            &mut self.arp_gate,
        ]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        let mut new_synth = VOneSynth::new();
        new_synth.id = self.id;
        let patch = self.get_patch("Clone");
        new_synth.set_patch(&patch);
        Box::new(new_synth)
    }
}

impl VOneSynth {
    // Helper to trigger a voice allocator logic
    fn trigger_internal(&mut self, note: u8, vel: f64) {
        let mut chosen_idx = 0;
        let mut found = false;
        // 1. Find inactive voice
        for (i, v) in self.voices.iter().enumerate() {
            if !v.active {
                chosen_idx = i;
                found = true;
                break;
            }
        }
        // 2. Steal if necessary (oldest active? just round robin for now/random)
        if !found {
            // Simple stealing: just take 0.
            // Better: Find oldest voice (not tracked yet).
            chosen_idx = 0;
        }
        self.voices[chosen_idx].trigger(note, vel, 0);
    }
}

// PolyBLEP Oscillator with SHAPE parameter
fn poly_blep_osc(w: OscType, phase: f64, dt: f64, shape: f64) -> f64 {
    match w {
        OscType::Sine => {
            let raw = (phase * 2.0 * PI).sin();
            if shape > 0.01 {
                // Shape adds Tube-like saturation/squaring
                (raw * (1.0 + shape * 5.0)).tanh()
            } else {
                raw
            }
        }
        OscType::Saw => {
            let mut val = 2.0 * phase - 1.0;
            val -= poly_blep(phase, dt);

            if shape > 0.01 {
                // Shape adds a second "Double Saw"
                let shift = shape * 0.5; // Phase offset
                let p2 = (phase + shift) % 1.0;
                let mut val2 = 2.0 * p2 - 1.0;
                val2 -= poly_blep(p2, dt);
                val = (val + val2) * 0.5;
            }
            val
        }
        OscType::Square => {
            // Pulse Width = 0.5 + shape * 0.45 (limit to 0.95)
            let pw = 0.5 + shape * 0.45;
            let mut val = if phase < pw { 1.0 } else { -1.0 };
            val += poly_blep(phase, dt);
            val -= poly_blep((phase + pw) % 1.0, dt);
            val
        }
        OscType::Triangle => {
            // Shape morphs triangle to... something? Let's leave triangle pure or morph to sine?
            // User didn't specify. Left as is.
            let p2 = phase * 2.0;
            if p2 < 1.0 {
                p2 * 2.0 - 1.0
            } else {
                3.0 - p2 * 2.0
            }
        }
        OscType::Noise => rand::random::<f64>() * 2.0 - 1.0,
    }
}

fn poly_blep(t: f64, dt: f64) -> f64 {
    let mut t = t;
    if t < dt {
        t /= dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

impl VOneSynth {
    pub fn get_patch(&self, name: &str) -> SynthPreset {
        let mut params = HashMap::new();
        // Helpers
        let mut add = |key: &str, p: &Parameter| {
            params.insert(key.to_string(), p.get_current_value());
        };

        // Osc 1
        add("osc1_type", &self.osc1_type);
        add("osc1_oct", &self.osc1_oct);
        add("osc1_semi", &self.osc1_semi);
        add("osc1_detune", &self.osc1_detune);
        add("osc1_gain", &self.osc1_gain);
        add("osc1_shape", &self.osc1_shape);

        // Osc 2
        add("osc2_type", &self.osc2_type);
        add("osc2_oct", &self.osc2_oct);
        add("osc2_semi", &self.osc2_semi);
        add("osc2_detune", &self.osc2_detune);
        add("osc2_gain", &self.osc2_gain);
        add("osc2_shape", &self.osc2_shape);

        // FM
        add("fm_amount", &self.fm_amount);

        // Osc 3
        add("osc3_type", &self.osc3_type);
        add("osc3_gain", &self.osc3_gain);

        // Filter
        add("filt_type", &self.filt_type);
        add("cutoff", &self.cutoff);
        add("res", &self.res);
        add("drive", &self.drive);
        add("filt_env_amt", &self.filt_env_amt);
        add("filt_kb", &self.filt_kb);

        // Amp Env
        add("amp_atk", &self.amp_atk);
        add("amp_dec", &self.amp_dec);
        add("amp_sus", &self.amp_sus);
        add("amp_rel", &self.amp_rel);

        // Filt Env
        add("filt_atk", &self.filt_atk);
        add("filt_dec", &self.filt_dec);
        add("filt_sus", &self.filt_sus);
        add("filt_rel", &self.filt_rel);

        // LFO
        add("lfo_rate", &self.lfo_rate);
        add("lfo_depth", &self.lfo_depth);

        // Master
        add("master_vol", &self.master_vol);

        // FX
        add("delay_mix", &self.delay_mix);
        add("delay_time", &self.delay_time);
        add("delay_feedback", &self.delay_feedback);
        add("chorus_mix", &self.chorus_mix);
        add("chorus_rate", &self.chorus_rate);
        add("chorus_depth", &self.chorus_depth);
        add("reverb_mix", &self.reverb_mix);
        add("reverb_size", &self.reverb_size);
        add("reverb_damping", &self.reverb_damping);
        add("dist_mix", &self.dist_mix);
        add("dist_drive", &self.dist_drive);
        add("dist_type", &self.dist_type);
        add("phaser_mix", &self.phaser_mix);
        add("phaser_rate", &self.phaser_rate);
        add("phaser_feedback", &self.phaser_feedback);

        // Mojo
        add("warmth", &self.warmth);
        add("spread", &self.spread);
        add("age", &self.age);
        add("unison_active", &self.unison_active);
        add("macro_x", &self.macro_x);
        add("macro_y", &self.macro_y);

        // Sequencer
        add("step_1", &self.step_1);
        add("step_2", &self.step_2);
        add("step_3", &self.step_3);
        add("step_4", &self.step_4);
        add("step_5", &self.step_5);
        add("step_6", &self.step_6);
        add("step_7", &self.step_7);
        add("step_8", &self.step_8);
        add("seq_target", &self.seq_target);

        // Arp
        add("arp_active", &self.arp_active);
        add("arp_mode", &self.arp_mode);
        add("arp_rate", &self.arp_rate);
        add("arp_oct", &self.arp_oct);
        add("arp_gate", &self.arp_gate);

        let mod_matrix = self.mod_matrix.to_vec();

        SynthPreset {
            name: name.to_string(),
            params,
            mod_matrix,
        }
    }

    pub fn set_patch(&mut self, preset: &SynthPreset) {
        for (name, val) in &preset.params {
            let v = *val;
            match name.as_str() {
                "osc1_type" => {
                    self.osc1_type.set_value(v);
                    self.osc1_type.value = v;
                }
                "osc1_oct" => {
                    self.osc1_oct.set_value(v);
                    self.osc1_oct.value = v;
                }
                "osc1_semi" => {
                    self.osc1_semi.set_value(v);
                    self.osc1_semi.value = v;
                }
                "osc1_detune" => {
                    self.osc1_detune.set_value(v);
                    self.osc1_detune.value = v;
                }
                "osc1_gain" => {
                    self.osc1_gain.set_value(v);
                    self.osc1_gain.value = v;
                }
                "osc1_shape" => {
                    self.osc1_shape.set_value(v);
                    self.osc1_shape.value = v;
                }

                "osc2_type" => {
                    self.osc2_type.set_value(v);
                    self.osc2_type.value = v;
                }
                "osc2_oct" => {
                    self.osc2_oct.set_value(v);
                    self.osc2_oct.value = v;
                }
                "osc2_semi" => {
                    self.osc2_semi.set_value(v);
                    self.osc2_semi.value = v;
                }
                "osc2_detune" => {
                    self.osc2_detune.set_value(v);
                    self.osc2_detune.value = v;
                }
                "osc2_gain" => {
                    self.osc2_gain.set_value(v);
                    self.osc2_gain.value = v;
                }
                "osc2_shape" => {
                    self.osc2_shape.set_value(v);
                    self.osc2_shape.value = v;
                }

                "fm_amount" => {
                    self.fm_amount.set_value(v);
                    self.fm_amount.value = v;
                }

                "osc3_type" => {
                    self.osc3_type.set_value(v);
                    self.osc3_type.value = v;
                }
                "osc3_gain" => {
                    self.osc3_gain.set_value(v);
                    self.osc3_gain.value = v;
                }

                "filt_type" => {
                    self.filt_type.set_value(v);
                    self.filt_type.value = v;
                }
                "cutoff" => {
                    self.cutoff.set_value(v);
                    self.cutoff.value = v;
                }
                "res" => {
                    self.res.set_value(v);
                    self.res.value = v;
                }
                "drive" => {
                    self.drive.set_value(v);
                    self.drive.value = v;
                }
                "filt_env_amt" => {
                    self.filt_env_amt.set_value(v);
                    self.filt_env_amt.value = v;
                }
                "filt_kb" => {
                    self.filt_kb.set_value(v);
                    self.filt_kb.value = v;
                }

                "amp_atk" => {
                    self.amp_atk.set_value(v);
                    self.amp_atk.value = v;
                }
                "amp_dec" => {
                    self.amp_dec.set_value(v);
                    self.amp_dec.value = v;
                }
                "amp_sus" => {
                    self.amp_sus.set_value(v);
                    self.amp_sus.value = v;
                }
                "amp_rel" => {
                    self.amp_rel.set_value(v);
                    self.amp_rel.value = v;
                }

                "filt_atk" => {
                    self.filt_atk.set_value(v);
                    self.filt_atk.value = v;
                }
                "filt_dec" => {
                    self.filt_dec.set_value(v);
                    self.filt_dec.value = v;
                }
                "filt_sus" => {
                    self.filt_sus.set_value(v);
                    self.filt_sus.value = v;
                }
                "filt_rel" => {
                    self.filt_rel.set_value(v);
                    self.filt_rel.value = v;
                }

                "lfo_rate" => {
                    self.lfo_rate.set_value(v);
                    self.lfo_rate.value = v;
                }
                "lfo_depth" => {
                    self.lfo_depth.set_value(v);
                    self.lfo_depth.value = v;
                }

                "master_vol" => {
                    self.master_vol.set_value(v);
                    self.master_vol.value = v;
                }

                "delay_mix" => {
                    self.delay_mix.set_value(v);
                    self.delay_mix.value = v;
                }
                "delay_time" => {
                    self.delay_time.set_value(v);
                    self.delay_time.value = v;
                }
                "delay_feedback" => {
                    self.delay_feedback.set_value(v);
                    self.delay_feedback.value = v;
                }

                "chorus_mix" => {
                    self.chorus_mix.set_value(v);
                    self.chorus_mix.value = v;
                }
                "chorus_rate" => {
                    self.chorus_rate.set_value(v);
                    self.chorus_rate.value = v;
                }
                "chorus_depth" => {
                    self.chorus_depth.set_value(v);
                    self.chorus_depth.value = v;
                }

                "reverb_mix" => {
                    self.reverb_mix.set_value(v);
                    self.reverb_mix.value = v;
                }
                "reverb_size" => {
                    self.reverb_size.set_value(v);
                    self.reverb_size.value = v;
                }
                "reverb_damping" => {
                    self.reverb_damping.set_value(v);
                    self.reverb_damping.value = v;
                }

                "dist_mix" => {
                    self.dist_mix.set_value(v);
                    self.dist_mix.value = v;
                }
                "dist_drive" => {
                    self.dist_drive.set_value(v);
                    self.dist_drive.value = v;
                }
                "dist_type" => {
                    self.dist_type.set_value(v);
                    self.dist_type.value = v;
                }

                "phaser_mix" => {
                    self.phaser_mix.set_value(v);
                    self.phaser_mix.value = v;
                }
                "phaser_rate" => {
                    self.phaser_rate.set_value(v);
                    self.phaser_rate.value = v;
                }
                "phaser_feedback" => {
                    self.phaser_feedback.set_value(v);
                    self.phaser_feedback.value = v;
                }

                "warmth" => {
                    self.warmth.set_value(v);
                    self.warmth.value = v;
                }
                "spread" => {
                    self.spread.set_value(v);
                    self.spread.value = v;
                }
                "age" => {
                    self.age.set_value(v);
                    self.age.value = v;
                }
                "unison_active" => {
                    self.unison_active.set_value(v);
                    self.unison_active.value = v;
                }
                "macro_x" => {
                    self.macro_x.set_value(v);
                    self.macro_x.value = v;
                }
                "macro_y" => {
                    self.macro_y.set_value(v);
                    self.macro_y.value = v;
                }

                "step_1" => {
                    self.step_1.set_value(v);
                    self.step_1.value = v;
                }
                "step_2" => {
                    self.step_2.set_value(v);
                    self.step_2.value = v;
                }
                "step_3" => {
                    self.step_3.set_value(v);
                    self.step_3.value = v;
                }
                "step_4" => {
                    self.step_4.set_value(v);
                    self.step_4.value = v;
                }
                "step_5" => {
                    self.step_5.set_value(v);
                    self.step_5.value = v;
                }
                "step_6" => {
                    self.step_6.set_value(v);
                    self.step_6.value = v;
                }
                "step_7" => {
                    self.step_7.set_value(v);
                    self.step_7.value = v;
                }
                "step_8" => {
                    self.step_8.set_value(v);
                    self.step_8.value = v;
                }
                "seq_target" => {
                    self.seq_target.set_value(v);
                    self.seq_target.value = v;
                }

                "arp_active" => {
                    self.arp_active.set_value(v);
                    self.arp_active.value = v;
                }
                "arp_mode" => {
                    self.arp_mode.set_value(v);
                    self.arp_mode.value = v;
                }
                "arp_rate" => {
                    self.arp_rate.set_value(v);
                    self.arp_rate.value = v;
                }
                "arp_oct" => {
                    self.arp_oct.set_value(v);
                    self.arp_oct.value = v;
                }
                "arp_gate" => {
                    self.arp_gate.set_value(v);
                    self.arp_gate.value = v;
                }

                _ => {}
            }
        }

        // Mod Matrix
        for (i, slot) in preset.mod_matrix.iter().enumerate() {
            if i < 8 {
                self.mod_matrix[i] = *slot;
            }
        }
    }

    pub fn save_to_json(&self, path: &str) -> std::io::Result<()> {
        let preset = self.get_patch("User Preset");
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &preset)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_from_json(&mut self, path: &str) -> std::io::Result<()> {
        let file = std::fs::File::open(path)?;
        let preset: SynthPreset = serde_json::from_reader(file)?;
        self.set_patch(&preset);
        Ok(())
    }
}
