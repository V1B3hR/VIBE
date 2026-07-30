use crate::engine::graph::{
    flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext,
};
use std::any::Any;
use std::collections::VecDeque;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DetectionMode {
    Peak,
    Rms,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TopologyMode {
    FeedForward,
    FeedBack,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StereoLink {
    Linked,   // max(L, R)
    Unlinked, // separate per channel
    MidSide,  // M/S detection
}

pub struct Compressor {
    id: Uuid,
    // Parameters
    pub threshold: Parameter,
    pub ratio: Parameter,
    pub attack: Parameter,
    pub release: Parameter,
    pub knee: Parameter,
    pub makeup: Parameter,
    pub lookahead: Parameter,
    pub mix: Parameter,

    // Switch Parameters
    pub detection: Parameter,
    pub topology: Parameter,
    pub link: Parameter,

    // Extended Parameters (The Glue Update)
    pub sidechain_hpf: Parameter,

    // Internal State
    sample_rate: f64,
    env_l: f64,
    env_r: f64,

    lookahead_buf_l: VecDeque<f64>,
    lookahead_buf_r: VecDeque<f64>,

    rms_window_samples: usize,
    rms_buf_l: VecDeque<f64>,
    rms_buf_r: VecDeque<f64>,
    rms_sum_l: f64,
    rms_sum_r: f64,

    prev_out_l: f64,
    prev_out_r: f64,

    current_gr_l: f64,
    current_gr_r: f64,

    // State for High Pass Filter (Internal Sidechain)
    hpf_prev_x_l: f64,
    hpf_prev_y_l: f64,
    hpf_prev_x_r: f64,
    hpf_prev_y_r: f64,

    // State for Auto-Release (Program Dependent)
    slow_envelope_l: f64,
    slow_envelope_r: f64,

    pub sidechain_enabled: Parameter,
}

struct CompParams {
    threshold: f64,
    ratio: f64,
    knee: f64,
    makeup: f64,
    att_coeff: f64,
    fast_release_coeff: f64,
    slow_release_coeff: f64,
    lookahead_size: usize,
    mix: f64,
    hpf_cutoff: f64,
}

impl Compressor {
    pub fn new(sample_rate: f64) -> Self {
        let id = Uuid::new_v4();
        let rms_window = (sample_rate * 0.010) as usize; // 10ms

        let mut comp = Self {
            id,
            threshold: Parameter::new("Threshold", -18.0, -60.0, 0.0),
            ratio: Parameter::new("Ratio", 4.0, 1.0, 20.0),
            attack: Parameter::new("Attack", 4.0, 0.1, 500.0),
            release: Parameter::new("Release", 80.0, 1.0, 2000.0),
            knee: Parameter::new("Knee", 6.0, 0.0, 24.0),
            makeup: Parameter::new("Makeup", 0.0, -24.0, 24.0),
            lookahead: Parameter::new("Lookahead", 0.0, 0.0, 10.0),
            mix: Parameter::new("Mix", 100.0, 0.0, 100.0),

            sidechain_hpf: Parameter::new("SC HPF", 20.0, 20.0, 300.0),

            detection: Parameter::new("Detection", 0.0, 0.0, 1.0), // 0: Peak, 1: RMS
            topology: Parameter::new("Topology", 0.0, 0.0, 1.0),   // 0: FF, 1: FB
            link: Parameter::new("Stereo Link", 0.0, 0.0, 2.0),    // 0: Linked, 1: Unlinked, 2: M/S

            sample_rate,
            env_l: 0.0,
            env_r: 0.0,

            hpf_prev_x_l: 0.0,
            hpf_prev_y_l: 0.0,
            hpf_prev_x_r: 0.0,
            hpf_prev_y_r: 0.0,

            slow_envelope_l: 0.0,
            slow_envelope_r: 0.0,

            lookahead_buf_l: VecDeque::new(),
            lookahead_buf_r: VecDeque::new(),

            rms_window_samples: rms_window.max(1),
            rms_buf_l: VecDeque::with_capacity(rms_window.max(1)),
            rms_buf_r: VecDeque::with_capacity(rms_window.max(1)),
            rms_sum_l: 0.0,
            rms_sum_r: 0.0,

            prev_out_l: 0.0,
            prev_out_r: 0.0,

            current_gr_l: 0.0,
            current_gr_r: 0.0,

            sidechain_enabled: Parameter::new("Sidechain", 0.0, 0.0, 1.0),
        };
        comp.update_lookahead();
        comp
    }

    fn update_lookahead(&mut self) {
        let ms = self.lookahead.get_current_value();
        let size = ((self.sample_rate * ms / 1000.0) as usize).max(1);
        self.lookahead_buf_l = VecDeque::with_capacity(size);
        self.lookahead_buf_r = VecDeque::with_capacity(size);
        for _ in 0..size {
            self.lookahead_buf_l.push_back(0.0);
            self.lookahead_buf_r.push_back(0.0);
        }
    }

    fn detect_level(&mut self, sample_l: f64, sample_r: f64) -> (f64, f64) {
        let mode = self.detection.get_current_value().round() as usize;
        match mode {
            0 => (sample_l.abs(), sample_r.abs()), // Peak
            1 | _ => {
                // RMS
                let sq_l = sample_l * sample_l;
                let sq_r = sample_r * sample_r;

                self.rms_sum_l += sq_l;
                self.rms_sum_r += sq_r;
                self.rms_buf_l.push_back(sq_l);
                self.rms_buf_r.push_back(sq_r);

                if self.rms_buf_l.len() > self.rms_window_samples {
                    self.rms_sum_l -= self.rms_buf_l.pop_front().unwrap_or(0.0);
                }
                if self.rms_buf_r.len() > self.rms_window_samples {
                    self.rms_sum_r -= self.rms_buf_r.pop_front().unwrap_or(0.0);
                }

                let rms_l = (self.rms_sum_l / self.rms_buf_l.len() as f64).sqrt();
                let rms_r = (self.rms_sum_r / self.rms_buf_r.len() as f64).sqrt();
                (rms_l, rms_r)
            }
        }
    }

    fn apply_stereo_link(&self, level_l: f64, level_r: f64) -> (f64, f64) {
        let mode = self.link.get_current_value().round() as usize;
        match mode {
            0 => {
                // Linked
                let max = level_l.max(level_r);
                (max, max)
            }
            1 => (level_l, level_r), // Unlinked
            2 | _ => {
                // MidSide
                let mid = (level_l + level_r) * 0.5;
                let side = (level_l - level_r) * 0.5;
                let detected = mid.abs().max(side.abs());
                (detected, detected)
            }
        }
    }

    // High Pass Filter for Sidechain (Simple 1-pole for detector)
    // y[n] = x[n] - x[n-1] + a * y[n-1]
    #[inline(always)]
    fn apply_sc_hpf(
        x: f64,
        prev_x: &mut f64,
        prev_y: &mut f64,
        cutoff: f64,
        sample_rate: f64,
    ) -> f64 {
        if cutoff <= 20.0 {
            return x; // Bypass if low
        }
        let rc = 1.0 / (2.0 * std::f64::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);
        let y = alpha * (*prev_y + x - *prev_x);
        *prev_x = x;
        *prev_y = flush_denormal_f64(y);
        y
    }

    // Soft Clipper for output saturation
    #[inline(always)]
    fn apply_soft_clip(x: f64) -> f64 {
        const THRESHOLD: f64 = 0.8;
        if x > THRESHOLD {
            THRESHOLD + (x - THRESHOLD).tanh() * (1.0 - THRESHOLD)
        } else if x < -THRESHOLD {
            -THRESHOLD + (x + THRESHOLD).tanh() * (1.0 - THRESHOLD)
        } else {
            x
        }
    }

    // Calculate adaptive release based on "Smart Opto" logic
    #[inline(always)]
    fn calculate_adaptive_release(current_gr: f64, slow_env: &mut f64, params: &CompParams) -> f64 {
        // 1. Track "depth" of compression
        // If compressor is constantly reducing gain, average GR increases
        *slow_env = *slow_env * 0.999 + current_gr * 0.001; // Very long integration

        // 2. Decision: Fast or Slow return?
        // If current attenuation is close to average -> Slow return (musical)
        // If it's a short transient peak -> Fast return

        let release_mix = (current_gr - *slow_env).abs().clamp(0.0, 1.0);

        // Linear Interpolate between fast and slow coefficient

        params.fast_release_coeff
            + (params.slow_release_coeff - params.fast_release_coeff) * release_mix
    }

    fn compute_gain_reduction(
        level: f64,
        env: &mut f64,
        slow_env: &mut f64,
        params: &CompParams,
    ) -> f64 {
        let level_db = if level > 1e-10 {
            20.0 * level.log10()
        } else {
            -200.0
        };
        let mut over = level_db - params.threshold;

        if over <= 0.0 {
            over = 0.0;
        } else if params.knee > 0.0 && over < params.knee {
            over = over * over / (2.0 * params.knee);
        }

        let gr_target = over * (1.0 - 1.0 / params.ratio);

        let coeff = if gr_target > *env {
            params.att_coeff
        } else {
            // Adaptive Release Logic
            Self::calculate_adaptive_release(*env, slow_env, params)
        };

        *env = coeff * *env + (1.0 - coeff) * gr_target;

        *env
    }

    #[allow(dead_code)]
    pub fn get_metrics(&self) -> (f32, f32) {
        (self.current_gr_l as f32, self.current_gr_r as f32)
    }
}

impl AudioProcessor for Compressor {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        self.sample_rate = sample_rate;
        let (left, right) = buffer.get_stereo_mut();

        // Prepare parameters
        let release_ms = self.release.get_current_value();

        // Smart Opto setup:
        // Fast release is usually user setting * 0.2 (20% of set time for transients)
        // Slow release is user setting (for sustained body)
        let fast_rel_coeff = (-1.0 / (sample_rate * (release_ms * 0.2) / 1000.0)).exp();
        let slow_rel_coeff = (-1.0 / (sample_rate * release_ms / 1000.0)).exp();

        let params = CompParams {
            threshold: self.threshold.get_current_value(),
            ratio: self.ratio.get_current_value(),
            knee: self.knee.get_current_value(),
            makeup: self.makeup.get_current_value(),
            att_coeff: (-1.0 / (sample_rate * self.attack.get_current_value() / 1000.0)).exp(),
            fast_release_coeff: fast_rel_coeff,
            slow_release_coeff: slow_rel_coeff,
            lookahead_size: ((sample_rate * self.lookahead.get_current_value() / 1000.0) as usize)
                .max(1),
            mix: self.mix.get_current_value(),
            hpf_cutoff: self.sidechain_hpf.get_current_value(),
        };

        for i in 0..left.len() {
            let input_l = left[i];
            let input_r = right[i];

            let sc_on = self.sidechain_enabled.get_current_value() > 0.5;

            let (detect_src_l, detect_src_r) = if sc_on && context.sidechain.is_some() {
                let sc = context.sidechain.unwrap();
                (sc.channels_data[0][i], sc.channels_data[1][i])
            } else {
                let topo_mode = self.topology.get_current_value().round() as usize;
                match topo_mode {
                    0 => (input_l, input_r),                     // FF
                    1 | _ => (self.prev_out_l, self.prev_out_r), // FB
                }
            };

            // 1. Internal Sidechain HPF (The "Low-End Savior")
            let filtered_l = Self::apply_sc_hpf(
                detect_src_l,
                &mut self.hpf_prev_x_l,
                &mut self.hpf_prev_y_l,
                params.hpf_cutoff,
                sample_rate,
            );
            let filtered_r = Self::apply_sc_hpf(
                detect_src_r,
                &mut self.hpf_prev_x_r,
                &mut self.hpf_prev_y_r,
                params.hpf_cutoff,
                sample_rate,
            );

            // 2. Detect Level
            let (level_l, level_r) = self.detect_level(filtered_l, filtered_r);

            self.lookahead_buf_l.push_back(level_l);
            self.lookahead_buf_r.push_back(level_r);

            // Note: In a real lookahead compressor, we would delay the audio input here too.
            // For now, consistent with previous implementation (Control Lookahead).

            while self.lookahead_buf_l.len() > params.lookahead_size {
                self.lookahead_buf_l.pop_front();
            }
            while self.lookahead_buf_r.len() > params.lookahead_size {
                self.lookahead_buf_r.pop_front();
            }

            // Lookahead max (if buffer is filled enough)
            let control_l = self.lookahead_buf_l.iter().cloned().fold(0.0, f64::max);
            let control_r = self.lookahead_buf_r.iter().cloned().fold(0.0, f64::max);

            let (linked_l, linked_r) = self.apply_stereo_link(control_l, control_r);

            // 3. Compute GR with Smart Release
            let gr_l = Self::compute_gain_reduction(
                linked_l,
                &mut self.env_l,
                &mut self.slow_envelope_l,
                &params,
            );
            let gr_r = Self::compute_gain_reduction(
                linked_r,
                &mut self.env_r,
                &mut self.slow_envelope_r,
                &params,
            );

            self.current_gr_l = gr_l;
            self.current_gr_r = gr_r;

            let gain_l = 10f64.powf(-gr_l / 20.0);
            let gain_r = 10f64.powf(-gr_r / 20.0);
            let makeup = 10f64.powf(params.makeup / 20.0);

            let mut out_l = flush_denormal_f64(input_l * gain_l * makeup);
            let mut out_r = flush_denormal_f64(input_r * gain_r * makeup);

            // 4. Output Saturation (Soft Clipper)
            out_l = Self::apply_soft_clip(out_l);
            out_r = Self::apply_soft_clip(out_r);

            // Apply Mix (Dry/Wet)
            let mix_val = params.mix / 100.0;
            left[i] = (out_l * mix_val) + (input_l * (1.0 - mix_val));
            right[i] = (out_r * mix_val) + (input_r * (1.0 - mix_val));

            self.prev_out_l = out_l;
            self.prev_out_r = out_r;
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: vec![
                self.threshold.clone(),
                self.ratio.clone(),
                self.attack.clone(),
                self.release.clone(),
                self.knee.clone(),
                self.makeup.clone(),
                self.lookahead.clone(),
                self.mix.clone(),
                // Extended
                self.sidechain_hpf.clone(),
                self.sidechain_enabled.clone(),
                self.detection.clone(),
                self.topology.clone(),
                self.link.clone(),
            ],
        })
    }
    fn name(&self) -> String {
        "Vibe Compressor".to_string()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.threshold,
            &mut self.ratio,
            &mut self.attack,
            &mut self.release,
            &mut self.knee,
            &mut self.makeup,
            &mut self.lookahead,
            &mut self.mix,
            &mut self.sidechain_hpf,
            &mut self.sidechain_enabled,
            &mut self.detection,
            &mut self.topology,
            &mut self.link,
        ]
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
