#![allow(dead_code)]
use super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;
use uuid::Uuid;

pub struct SpectralGate {
    id: Uuid,
    pub threshold: Parameter, // in dB
    pub reduction: Parameter, // 0.0 to 1.0 (multiplier)
    pub attack: Parameter,
    pub release: Parameter,

    fft_forward: Arc<dyn Fft<f64>>,
    fft_inverse: Arc<dyn Fft<f64>>,

    // Per-bin gain states for smoothing
    bin_gain: Vec<f64>,

    // Buffers for FFT
    fft_buffer: Vec<Complex<f64>>,
    overlap_buffer: Vec<f64>,

    window: Vec<f64>,
    block_size: usize,
}

impl SpectralGate {
    pub fn new(block_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft_len = block_size;

        let mut window = Vec::with_capacity(fft_len);
        for i in 0..fft_len {
            // Hann window
            let w = 0.5
                * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (fft_len as f64 - 1.0)).cos());
            window.push(w);
        }

        Self {
            id: Uuid::new_v4(),
            threshold: Parameter::new("Sp. Thr", -40.0, -100.0, 0.0),
            reduction: Parameter::new("Sp. Red", 0.1, 0.0, 1.0),
            attack: Parameter::new("Sp. Att", 10.0, 0.1, 100.0),
            release: Parameter::new("Sp. Rel", 50.0, 1.0, 500.0),
            fft_forward: planner.plan_fft_forward(fft_len),
            fft_inverse: planner.plan_fft_inverse(fft_len),
            bin_gain: vec![1.0; fft_len],
            fft_buffer: vec![Complex::default(); fft_len],
            overlap_buffer: vec![0.0; fft_len],
            window,
            block_size,
        }
    }
}

impl AudioProcessor for SpectralGate {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Spectral Gate".to_string()
    }

    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let frames = buffer.frames;
        if frames != self.block_size {
            return;
        }

        let sr = context.sample_rate;
        let threshold_db = self.threshold.get_current_value();
        let reduction = self.reduction.get_current_value();

        // Attack/Release factors
        let att = (-1.0 / (sr * self.attack.get_current_value() / 1000.0)).exp();
        let rel = (-1.0 / (sr * self.release.get_current_value() / 1000.0)).exp();

        for c in 0..2 {
            // 1. Fill buffer and Apply Window
            for i in 0..self.block_size {
                self.fft_buffer[i] = Complex::new(buffer.channels_data[c][i] * self.window[i], 0.0);
            }

            // 2. Forward FFT
            self.fft_forward.process(&mut self.fft_buffer);

            // 3. Process each bin
            for i in 0..(self.block_size / 2 + 1) {
                let magnitude = (self.fft_buffer[i].norm_sqr() + 1e-12).sqrt();
                let mag_db = 20.0 * magnitude.log10();

                let target_gain = if mag_db > threshold_db {
                    1.0
                } else {
                    reduction
                };

                // Smoothing
                let coef = if target_gain < self.bin_gain[i] {
                    att
                } else {
                    rel
                };
                self.bin_gain[i] = coef * self.bin_gain[i] + (1.0 - coef) * target_gain;

                // Apply gain
                self.fft_buffer[i] *= self.bin_gain[i];

                // Symmetric conjugate for real IFFT
                if i > 0 && i < self.block_size / 2 {
                    self.fft_buffer[self.block_size - i] = self.fft_buffer[i].conj();
                }
            }

            // 4. Inverse FFT
            self.fft_inverse.process(&mut self.fft_buffer);

            // 5. Apply Window again and output
            let norm = 1.0 / (self.block_size as f64);
            for i in 0..self.block_size {
                let sample = self.fft_buffer[i].re * norm * self.window[i];
                // Overlap-add (simplified 50% overlap logic would go here,
                // but for non-partitioned block-processing, we just write back)
                buffer.channels_data[c][i] = sample;
            }
        }
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.threshold,
            &mut self.reduction,
            &mut self.attack,
            &mut self.release,
        ]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }
}
