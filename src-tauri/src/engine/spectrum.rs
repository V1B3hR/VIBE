#![allow(dead_code)]
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const FFT_SIZE: usize = 2048;
const NUM_BANDS: usize = 1024; // Increased for higher resolution since it's off-threaded

/// Thread-safe Spectrum Data storage
pub struct SpectrumData {
    pub magnitudes: Vec<AtomicU64>,
}

impl SpectrumData {
    pub fn new() -> Self {
        let mut magnitudes = Vec::with_capacity(FFT_SIZE / 2);
        for _ in 0..(FFT_SIZE / 2) {
            magnitudes.push(AtomicU64::new(0));
        }
        Self { magnitudes }
    }

    pub fn store(&self, data: &[f64]) {
        for (i, &val) in data.iter().enumerate() {
            if i < self.magnitudes.len() {
                self.magnitudes[i].store(val.to_bits(), Ordering::Relaxed);
            }
        }
    }

    pub fn load_to_vec(&self) -> Vec<f32> {
        self.magnitudes
            .iter()
            .map(|a| f64::from_bits(a.load(Ordering::Relaxed)) as f32)
            .collect()
    }
}

/// Real-Time Analyzer
pub struct SpectrumAnalyzer {
    fft_planner: FftPlanner<f64>,
    fft_buffer: Vec<Complex<f64>>,
    magnitude_buffer: Vec<f64>,
    window: Vec<f64>,
    sample_accumulator: Vec<f64>,
    samples_collected: usize,
    pub shared_data: Arc<SpectrumData>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        let mut window = vec![0.0; FFT_SIZE];
        for i in 0..FFT_SIZE {
            window[i] =
                0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / FFT_SIZE as f64).cos());
        }

        Self {
            fft_planner: FftPlanner::new(),
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            magnitude_buffer: vec![0.0; FFT_SIZE / 2],
            window,
            sample_accumulator: vec![0.0; FFT_SIZE],
            samples_collected: 0,
            shared_data: Arc::new(SpectrumData::new()),
        }
    }

    pub fn process(&mut self, samples: &[f64]) {
        for &sample in samples {
            self.sample_accumulator[self.samples_collected] = sample;
            self.samples_collected += 1;
            if self.samples_collected >= FFT_SIZE {
                self.compute_spectrum();
                self.samples_collected = 0;
            }
        }
    }

    pub fn process_stereo(&mut self, l: &[f64], r: &[f64]) {
        let frames = l.len().min(r.len());
        for i in 0..frames {
            let mono = (l[i] + r[i]) * 0.5;
            self.sample_accumulator[self.samples_collected] = mono;
            self.samples_collected += 1;

            if self.samples_collected >= FFT_SIZE {
                self.compute_spectrum();
                self.samples_collected = 0;
            }
        }
    }

    fn compute_spectrum(&mut self) {
        for i in 0..FFT_SIZE {
            self.fft_buffer[i] = Complex::new(self.sample_accumulator[i] * self.window[i], 0.0);
        }

        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);
        fft.process(&mut self.fft_buffer);

        for i in 0..(FFT_SIZE / 2) {
            let real = self.fft_buffer[i].re;
            let imag = self.fft_buffer[i].im;
            let mag = (real * real + imag * imag).sqrt();
            let db = if mag > 1e-5 {
                20.0 * mag.log10()
            } else {
                -100.0
            };
            self.magnitude_buffer[i] = db;
        }

        self.shared_data.store(&self.magnitude_buffer);
    }

    pub fn get_bands(&self) -> Vec<f64> {
        let mut bands = vec![0.0; 32];
        let magnitudes = self.shared_data.load_to_vec();
        let bins_per_band = magnitudes.len() / 32;

        for (band_idx, band) in bands.iter_mut().enumerate() {
            let start = band_idx * bins_per_band;
            let end = (start + bins_per_band).min(magnitudes.len());
            let sum: f32 = magnitudes[start..end].iter().sum();
            *band = (sum / bins_per_band as f32) as f64;
        }
        bands
    }

    pub fn get_scope_data(&self) -> (Vec<f32>, Vec<f32>) {
        // Placeholder for scope data until re-implemented with atomics if needed
        (vec![0.0; 2048], vec![0.0; 2048])
    }
}
