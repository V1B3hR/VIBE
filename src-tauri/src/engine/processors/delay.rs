use super::super::graph::{
    flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext,
};
use std::f64::consts::PI;
use uuid::Uuid;

/// Feature-rich Vibe Echo Delay
/// Capabilities: This is a professional-grade delay unit.
/// - Stereo/Ping-Pong logic
/// - Tape Warble (LFO modulation of read heads)
/// - Diffusion (Blurring repeats using Allpass)
/// - Ducker (Internal Sidechain to duck wet signal when input is present)
/// - Tone Shaping (LPF)
pub struct StereoDelay {
    id: Uuid,
    buffer_l: Vec<f64>,
    buffer_r: Vec<f64>,
    write_pos: usize,

    // Parameters
    pub time_l: Parameter,
    pub time_r: Parameter,
    pub feedback: Parameter,
    pub low_pass: Parameter, // Cutoff for feedback loop
    pub mix: Parameter,

    // Switch Parameters
    pub ping_pong: Parameter, // 0.0 = Off, 1.0 = On
    pub sync: Parameter,      // 0.0 = Off, 1.0 = On
    pub sync_note: Parameter, // 0=1/1, 1=1/2, 2=1/4, 3=1/8, 4=1/16

    // "Texture" Params
    pub diffusion: Parameter, // 0.0 to 1.0 (Amount of smear)
    pub warble: Parameter,    // 0.0 to 1.0 (Tape flutter depth)
    pub ducking: Parameter,   // 0.0 to 1.0 (Compression amt)

    // Internal State
    sample_rate: f64,

    // Filter state for feedback LP
    prev_feedback_l: f64,
    prev_feedback_r: f64,

    // Diffusion State: 2 Allpass per channel
    diff_l1: AllpassDelay,
    diff_l2: AllpassDelay,
    diff_r1: AllpassDelay,
    diff_r2: AllpassDelay,

    // LFO State for Warble
    lfo_phase: f64,

    // Ducker State (Envelope Follower)
    duck_env: f64,
}

impl StereoDelay {
    pub fn new() -> Self {
        let max_delay_secs = 2.0;
        let sample_rate = 48000.0;
        let buffer_size = (max_delay_secs * sample_rate) as usize;

        Self {
            id: Uuid::new_v4(),
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            write_pos: 0,

            time_l: Parameter::new("Time L", 0.375, 0.0, 2.0), // Dotted 8th-ish default
            time_r: Parameter::new("Time R", 0.5, 0.0, 2.0),
            feedback: Parameter::new("Feedback", 0.4, 0.0, 1.1), // >1.0 for explosion? limited in logic
            low_pass: Parameter::new("LP Color", 8000.0, 100.0, 20000.0),
            mix: Parameter::new("Mix", 0.3, 0.0, 1.0),

            ping_pong: Parameter::new("PingPong", 1.0, 0.0, 1.0),
            sync: Parameter::new("Sync", 1.0, 0.0, 1.0),
            sync_note: Parameter::new("Sync Note", 2.0, 0.0, 4.0),

            // New Texture Params
            diffusion: Parameter::new("Diffusion", 0.0, 0.0, 1.0),
            warble: Parameter::new("Tape Warble", 0.0, 0.0, 1.0),
            ducking: Parameter::new("Ducker", 0.0, 0.0, 1.0),

            sample_rate,
            prev_feedback_l: 0.0,
            prev_feedback_r: 0.0,

            // Short allpasses for diffusion (2-50ms range usually)
            diff_l1: AllpassDelay::new(220), // ~5ms at 48k
            diff_l2: AllpassDelay::new(740), // ~15ms
            diff_r1: AllpassDelay::new(220),
            diff_r2: AllpassDelay::new(740),

            lfo_phase: 0.0,
            duck_env: 0.0,
        }
    }
}

impl AudioProcessor for StereoDelay {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        let playhead = context.playhead;
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            // Ideally re-alloc buffer if size changes drastically
        }

        let frames = buffer.frames;
        let channels = buffer.num_channels;

        let feedback = self.feedback.get_value_at(playhead);
        let lp_freq = self.low_pass.get_value_at(playhead);
        let mix_val = self.mix.get_value_at(playhead);
        let ping_pong_on = self.ping_pong.get_value_at(playhead) > 0.5;

        let diff_amt = self.diffusion.get_value_at(playhead);
        let warble_amt = self.warble.get_value_at(playhead);
        let duck_amt = self.ducking.get_value_at(playhead);

        // Pre-calcs for LP
        let alpha = 1.0 - (-2.0 * PI * lp_freq / sample_rate).exp();

        let buf_len_l = self.buffer_l.len();
        let buf_len_r = self.buffer_r.len(); // Should be same

        // Ducker Attack/Release coeffs (Fast Attack, Med Release)
        // Attack 10ms, Release 300ms
        let duck_att = (-1.0 / (sample_rate * 0.010)).exp();
        let duck_rel = (-1.0 / (sample_rate * 0.300)).exp();

        // Warble LFO parameters (Rate ~ 0.5Hz to 3Hz)
        let lfo_rate = 2.0; // Hz
        let lfo_inc = (2.0 * PI * lfo_rate) / sample_rate;

        for i in 0..frames {
            // Update LFO
            self.lfo_phase += lfo_inc;
            if self.lfo_phase > 2.0 * PI {
                self.lfo_phase -= 2.0 * PI;
            }
            let lfo_val = self.lfo_phase.sin();

            // Calculate modulated delay times
            let current_playhead = playhead + i as u64;
            let raw_time_l = self.time_l.get_value_at(current_playhead);
            let raw_time_r = self.time_r.get_value_at(current_playhead);

            // Tape Warble: Modulate delay time slightly (e.g. +/- 2ms max)
            // warble_amt 1.0 = 5ms jitter
            let jitter_sec = warble_amt * 0.005 * lfo_val;

            let time_l = (raw_time_l + jitter_sec).max(0.001);
            let time_r = (raw_time_r + jitter_sec).max(0.001); // Sync modulation for stereo tape effect usually

            let delay_samples_l = time_l * sample_rate;
            let delay_samples_r = time_r * sample_rate;

            // --- Interpolated Read L ---
            // Concept: read_ptr = write_pos - delay
            let read_ptr_l = self.write_pos as f64 - delay_samples_l;
            // Wrap logic for ring buffer
            let read_ptr_l = if read_ptr_l < 0.0 {
                read_ptr_l + buf_len_l as f64
            } else {
                read_ptr_l
            };

            let idx_l = read_ptr_l as usize;
            let frac_l = read_ptr_l - idx_l as f64;
            let idx_l_next = (idx_l + 1) % buf_len_l;
            let wet_l_raw =
                self.buffer_l[idx_l] * (1.0 - frac_l) + self.buffer_l[idx_l_next] * frac_l;

            // --- Interpolated Read R ---
            let read_ptr_r = self.write_pos as f64 - delay_samples_r;
            let read_ptr_r = if read_ptr_r < 0.0 {
                read_ptr_r + buf_len_r as f64
            } else {
                read_ptr_r
            };

            let idx_r = read_ptr_r as usize;
            let frac_r = read_ptr_r - idx_r as f64;
            let idx_r_next = (idx_r + 1) % buf_len_r;
            let wet_r_raw =
                self.buffer_r[idx_r] * (1.0 - frac_r) + self.buffer_r[idx_r_next] * frac_r;

            // --- Diffusion Step ---
            // If diffusion > 0, run taps through allpasses
            // We only process if diff_amt is substantial relative to 0 to save CPU? No, always run or branch.
            // Branch inside loop is fine.

            let mut wet_l = wet_l_raw;
            let mut wet_r = wet_r_raw;

            if diff_amt > 0.01 {
                // Set allpass coefficient based on amount. 0.6 is good max.
                let coeff = diff_amt * 0.6;
                wet_l = self.diff_l1.process(wet_l, coeff);
                wet_l = self.diff_l2.process(wet_l, coeff);

                wet_r = self.diff_r1.process(wet_r, coeff);
                wet_r = self.diff_r2.process(wet_r, coeff);
            }

            // --- Input Handling & Ducker ---
            let in_l = buffer.channels_data[0][i];
            let in_r = if channels > 1 {
                buffer.channels_data[1][i]
            } else {
                in_l
            };

            // Ducker Envelope Follower (Input Peak)
            let in_sum = (in_l.abs() + in_r.abs()) * 0.5;
            let coef = if in_sum > self.duck_env {
                duck_att
            } else {
                duck_rel
            };
            self.duck_env = coef * self.duck_env + (1.0 - coef) * in_sum;

            // Calculate Duck Gain (1.0 = full volume, 0.0 = silenced)
            // If duck_amt is 1.0, and envelope is 1.0 (0dB), gain drops to 0.
            // Simple logic: gain = 1.0 - (env * duck_amt * compression_ratio)
            // Let's make it strong:
            let duck_gain = (1.0 - (self.duck_env * duck_amt * 4.0)).clamp(0.0, 1.0);

            // --- Feedback Logic ---
            // Apply Ducker gain to WET signal going to OUTPUT, but typically duckers affect output mix.
            // Or feedback loop? Usually Output Mix.
            // "When vocals sing, delay quiets down".
            // So we apply duck_gain to wet_l/wet_r relative to MIX, or just scale them?
            // Let's scale the Wet signal used for Output.
            let wet_l_ducked = wet_l * duck_gain;
            let wet_r_ducked = wet_r * duck_gain;

            // Feedback loop takes the UNDUCKED signal?
            // Often Ducker is on output only so trails come back up.
            // Yes. Feedback loop should sustain.

            let fb_l = flush_denormal_f64(wet_l * feedback);
            let fb_r = flush_denormal_f64(wet_r * feedback);

            // Tone Filter (LP on feedback)
            self.prev_feedback_l += alpha * (fb_l - self.prev_feedback_l);
            self.prev_feedback_r += alpha * (fb_r - self.prev_feedback_r);

            let filtered_fb_l = flush_denormal_f64(self.prev_feedback_l);
            let filtered_fb_r = flush_denormal_f64(self.prev_feedback_r);

            // Write back to buffer
            if ping_pong_on {
                // Swap channels in feedback
                self.buffer_l[self.write_pos] = in_l + filtered_fb_r;
                self.buffer_r[self.write_pos] = in_r + filtered_fb_l;
            } else {
                self.buffer_l[self.write_pos] = in_l + filtered_fb_l;
                self.buffer_r[self.write_pos] = in_r + filtered_fb_r;
            }

            // --- Output Mix ---
            buffer.channels_data[0][i] = in_l * (1.0 - mix_val) + wet_l_ducked * mix_val;
            if channels > 1 {
                buffer.channels_data[1][i] = in_r * (1.0 - mix_val) + wet_r_ducked * mix_val;
            }

            self.write_pos = (self.write_pos + 1) % self.buffer_l.len();
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }
    fn name(&self) -> String {
        "VIBE Echo Delay".to_string()
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: vec![
                self.time_l.clone(),
                self.time_r.clone(),
                self.feedback.clone(),
                self.low_pass.clone(),
                self.mix.clone(),
                self.ping_pong.clone(),
                self.sync.clone(),
                self.sync_note.clone(),
                self.diffusion.clone(),
                self.warble.clone(),
                self.ducking.clone(),
            ],
        })
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.time_l,
            &mut self.time_r,
            &mut self.feedback,
            &mut self.low_pass,
            &mut self.mix,
            &mut self.ping_pong,
            &mut self.sync,
            &mut self.sync_note,
            &mut self.diffusion,
            &mut self.warble,
            &mut self.ducking,
        ]
    }
}

// --- Components ---

// Simple Allpass Delay for Diffusion
struct AllpassDelay {
    buffer: Vec<f64>,
    pos: usize,
}

impl AllpassDelay {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len],
            pos: 0,
        }
    }

    fn process(&mut self, input: f64, coeff: f64) -> f64 {
        // y[n] = -coeff * x[n] + x[n-D] + coeff * y[n-D]
        // Standard Schroeder/Moorer implementation
        let buf_out = self.buffer[self.pos];

        let delayed_part = buf_out;
        let output = -coeff * input + delayed_part;

        // Update buffer: x[n] + coeff * y[n-D] ??
        // Or simpler:
        // out = delayed - k * input
        // new_delayed = input + k * out

        let new_val = input + coeff * output;

        self.buffer[self.pos] = flush_denormal_f64(new_val);
        self.pos = (self.pos + 1) % self.buffer.len();

        output
    }
}
