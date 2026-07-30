use crate::engine::graph::{
    flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext,
};
use crate::engine::oversampling::PolyphaseFir;
use std::f64::consts::PI;
use uuid::Uuid;

pub struct TubeLimiter {
    id: Uuid,
    input_gain: Parameter,
    ceiling: Parameter,
    drive: Parameter,
    release: Parameter,
    true_peak: Parameter,

    // Internal state
    envelope: f64,
    sample_rate: f64,

    // Oversampling
    upsampler_l: PolyphaseFir,
    upsampler_r: PolyphaseFir,
    downsampler_l: PolyphaseFir,
    downsampler_r: PolyphaseFir,
    os_buffer_l: Vec<f64>,
    os_buffer_r: Vec<f64>,
}

impl TubeLimiter {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            input_gain: Parameter::new("Input Gain", 0.0, -24.0, 24.0), // dB
            ceiling: Parameter::new("Ceiling", -0.1, -24.0, 0.0),       // dB
            drive: Parameter::new("Tube Drive", 0.5, 0.0, 1.0),
            release: Parameter::new("Release", 100.0, 1.0, 1000.0), // ms
            true_peak: Parameter::new("True Peak", 0.0, 0.0, 1.0),  // 0: Off, 1: On
            envelope: 0.0,
            sample_rate: 44100.0,

            // Init with 4x factor (2 chans needed? no, FIR is mono)
            upsampler_l: PolyphaseFir::new(4, 1),
            upsampler_r: PolyphaseFir::new(4, 1),
            downsampler_l: PolyphaseFir::new(4, 1),
            downsampler_r: PolyphaseFir::new(4, 1),

            os_buffer_l: vec![0.0; 4096 * 4],
            os_buffer_r: vec![0.0; 4096 * 4],
        }
    }

    fn db_to_linear(db: f64) -> f64 {
        (10.0f64).powf(db / 20.0)
    }

    fn soft_clip_tube(x: f64, drive: f64) -> f64 {
        // Asymmetric saturation for tube-like character (more 2nd harmonic)
        let x = x * (1.0 + drive * 2.0);
        if x > 0.0 {
            (2.0 / PI) * (x * PI / 2.0).atan()
        } else {
            // Slightly different curve for negative side
            0.9 * (2.0 / PI) * (x * 1.1 * PI / 2.0).atan()
        }
    }

    #[inline(always)]
    fn process_pair(
        &mut self,
        l_in: f64,
        r_in: f64,
        input_gain: f64,
        ceiling: f64,
        drive: f64,
        rel_coeff: f64,
    ) -> (f64, f64) {
        let mut left = l_in * input_gain;
        let mut right = r_in * input_gain;

        // Simple peak detection
        let input_peak = left.abs().max(right.abs());

        // Envelope follower
        if input_peak > self.envelope {
            self.envelope = input_peak;
        } else {
            self.envelope = input_peak + rel_coeff * (self.envelope - input_peak);
        }
        self.envelope = flush_denormal_f64(self.envelope);

        // Gain reduction
        let mut reduction = 1.0;
        if self.envelope > ceiling {
            reduction = ceiling / self.envelope;
        }

        left *= reduction;
        right *= reduction;

        left = Self::soft_clip_tube(left, drive);
        right = Self::soft_clip_tube(right, drive);

        // Ceiling clamp
        (
            flush_denormal_f64(left.clamp(-ceiling, ceiling)),
            flush_denormal_f64(right.clamp(-ceiling, ceiling)),
        )
    }
}

impl AudioProcessor for TubeLimiter {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Magneto-Tube Limiter".to_string()
    }

    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        self.sample_rate = sample_rate;
        let input_gain_lin = Self::db_to_linear(self.input_gain.get_current_value());
        let ceiling_lin = Self::db_to_linear(self.ceiling.get_current_value());
        let drive_val = self.drive.get_current_value();
        let release_ms = self.release.get_current_value();
        let use_true_peak = self.true_peak.get_current_value() > 0.5;

        // Effective Sample Rate for coefficients
        let sr = if use_true_peak {
            sample_rate * 4.0
        } else {
            sample_rate
        };

        let release_samples = release_ms * sr / 1000.0;
        let release_coeff = (-1.0f64 / release_samples).exp();

        let frames = buffer.frames;

        if use_true_peak {
            // Resize buffers if needed
            if self.os_buffer_l.len() < frames * 4 {
                self.os_buffer_l.resize(frames * 4, 0.0);
                self.os_buffer_r.resize(frames * 4, 0.0);
            }

            // Upsample
            self.upsampler_l.upsample(
                &buffer.channels_data[0][..frames],
                &mut self.os_buffer_l[..frames * 4],
                0,
            );
            self.upsampler_r.upsample(
                &buffer.channels_data[1][..frames],
                &mut self.os_buffer_r[..frames * 4],
                0,
            );

            let num_samples = frames * 4;
            for i in 0..num_samples {
                let (l, r) = self.process_pair(
                    self.os_buffer_l[i],
                    self.os_buffer_r[i],
                    input_gain_lin,
                    ceiling_lin,
                    drive_val,
                    release_coeff,
                );
                self.os_buffer_l[i] = l;
                self.os_buffer_r[i] = r;
            }

            // Downsample
            self.downsampler_l.downsample(
                &self.os_buffer_l[..frames * 4],
                &mut buffer.channels_data[0][..frames],
                0,
            );
            self.downsampler_r.downsample(
                &self.os_buffer_r[..frames * 4],
                &mut buffer.channels_data[1][..frames],
                0,
            );
        } else {
            for i in 0..frames {
                let (l, r) = self.process_pair(
                    buffer.channels_data[0][i],
                    buffer.channels_data[1][i],
                    input_gain_lin,
                    ceiling_lin,
                    drive_val,
                    release_coeff,
                );
                buffer.channels_data[0][i] = l;
                buffer.channels_data[1][i] = r;
            }
        }
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.input_gain,
            &mut self.ceiling,
            &mut self.drive,
            &mut self.release,
            &mut self.true_peak,
        ]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            input_gain: self.input_gain.clone(),
            ceiling: self.ceiling.clone(),
            drive: self.drive.clone(),
            release: self.release.clone(),
            true_peak: self.true_peak.clone(),
            envelope: 0.0,
            sample_rate: self.sample_rate,
            upsampler_l: self.upsampler_l.clone(),
            upsampler_r: self.upsampler_r.clone(),
            downsampler_l: self.downsampler_l.clone(),
            downsampler_r: self.downsampler_r.clone(),
            os_buffer_l: self.os_buffer_l.clone(),
            os_buffer_r: self.os_buffer_r.clone(),
        })
    }
}
