use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct SpectrumAnalyzer {
    input_buffer: Vec<f32>,
    write_pos: usize,
    fft_size: usize,
    // Use AtomicU32 for storage (f32 bits) to avoid locks during transfer
    spectrum_storage: Arc<Vec<AtomicU32>>,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize) -> Self {
        let mut storage = Vec::with_capacity(fft_size);
        for _ in 0..fft_size {
            storage.push(AtomicU32::new(0.0f32.to_bits()));
        }

        Self {
            input_buffer: vec![0.0; fft_size],
            write_pos: 0,
            fft_size,
            spectrum_storage: Arc::new(storage),
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        for &s in samples {
            self.input_buffer[self.write_pos] = s;
            self.write_pos = (self.write_pos + 1) % self.fft_size;
        }
    }

    pub fn analyze(&mut self) {
        // GPU Spectral Engine: CPU no longer computes FFT.
        // We simply flush the circular buffer to the atomic storage for the front-end to read.
        for i in 0..self.fft_size {
            // Read from circular buffer starting from the oldest sample
            let idx = (self.write_pos + i) % self.fft_size;
            let val = self.input_buffer[idx];
            self.spectrum_storage[i].store(val.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn get_data(&self) -> Vec<f32> {
        // Return raw time-domain data
        self.spectrum_storage
            .iter()
            .map(|a| f32::from_bits(a.load(Ordering::Relaxed)))
            .collect()
    }
}

