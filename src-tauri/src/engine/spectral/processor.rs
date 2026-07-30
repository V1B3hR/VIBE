use super::config::MelSpectrogramConfig;
use super::types::MelFrame;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::Arc;

pub struct MelProcessor {
    config: MelSpectrogramConfig,
    fft: Arc<dyn rustfft::Fft<f32>>,
    window: Vec<f32>,
    filterbank: Vec<Vec<f32>>, // [n_mels][fft_size / 2 + 1]
}

impl MelProcessor {
    pub fn new(config: MelSpectrogramConfig) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(config.fft_size);

        // Hann Window
        let window = (0..config.fft_size)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (config.fft_size - 1) as f32).cos())
            })
            .collect();

        let filterbank = Self::create_filterbank(&config);

        Self {
            config,
            fft,
            window,
            filterbank,
        }
    }

    fn create_filterbank(config: &MelSpectrogramConfig) -> Vec<Vec<f32>> {
        let n_mels = config.n_mels;
        let fft_size = config.fft_size;
        let sample_rate = config.sample_rate as f32;

        // Mel scale conversion
        let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
        let mel_to_hz = |mel: f32| 700.0 * (10.0f32.powf(mel / 2595.0) - 1.0);

        let mel_min = hz_to_mel(config.f_min);
        let mel_max = hz_to_mel(config.f_max);

        // Points in Mel scale
        let mut mel_points = vec![0.0; n_mels + 2];
        for i in 0..n_mels + 2 {
            mel_points[i] = mel_min + (mel_max - mel_min) * (i as f32 / (n_mels + 1) as f32);
        }

        // Points in Hz
        let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();

        // Bin indices
        let bin_points: Vec<usize> = hz_points
            .iter()
            .map(|&hz| (hz * (fft_size as f32 + 1.0) / sample_rate).floor() as usize)
            .collect();

        let mut filters = vec![vec![0.0; (fft_size / 2) + 1]; n_mels];

        for i in 0..n_mels {
            let start = bin_points[i];
            let mid = bin_points[i + 1];
            let end = bin_points[i + 2];

            for j in start..mid {
                if j < filters[i].len() {
                    filters[i][j] = (j - start) as f32 / (mid - start) as f32;
                }
            }
            for j in mid..end {
                if j < filters[i].len() {
                    filters[i][j] = (end - j) as f32 / (end - mid) as f32;
                }
            }
        }

        filters
    }

    pub fn process_frame(&self, samples: &[f32], timestamp: u64) -> MelFrame {
        let mut input: Vec<Complex<f32>> = samples
            .iter()
            .zip(self.window.iter())
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();

        // Padding if needed
        if input.len() < self.config.fft_size {
            input.resize(self.config.fft_size, Complex::new(0.0, 0.0));
        }

        self.fft.process(&mut input);

        let half = self.config.fft_size / 2 + 1;
        let mut magnitude = vec![0.0; half];
        for (i, complex) in input.iter().take(half).enumerate() {
            let mag = (complex.re * complex.re + complex.im * complex.im).sqrt();
            magnitude[i] = mag.powf(self.config.power);
        }

        // Apply Mel Filterbank
        let mut mel_data = vec![0.0; self.config.n_mels];
        for i in 0..self.config.n_mels {
            let mut sum = 0.0;
            for j in 0..half {
                sum += magnitude[j] * self.filterbank[i][j];
            }
            // Log-magnitude (dB)
            mel_data[i] = 10.0 * (sum + 1e-9).log10(); // small epsilon to avoid log(0)
        }

        MelFrame {
            data: mel_data,
            timestamp_samples: timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mel_processor_creation() {
        let config = MelSpectrogramConfig::default();
        let processor = MelProcessor::new(config.clone());
        assert_eq!(processor.window.len(), config.fft_size);
        assert_eq!(processor.filterbank.len(), config.n_mels);
    }

    #[test]
    fn test_process_sine_wave() {
        let config = MelSpectrogramConfig::default();
        let processor = MelProcessor::new(config.clone());

        // Generate a 1kHz sine wave
        let freq = 1000.0;
        let sample_rate = config.sample_rate as f32;
        let mut samples = vec![0.0; config.fft_size];
        for i in 0..config.fft_size {
            samples[i] = (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin();
        }

        let frame = processor.process_frame(&samples, 0);
        assert_eq!(frame.data.len(), config.n_mels);

        // Find the index of the maximum energy
        let mut max_idx = 0;
        let mut max_val = -100.0;
        for (i, &val) in frame.data.iter().enumerate() {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }

        // 1kHz should be somewhere in the middle of the mel scale
        assert!(max_idx > 0);
        assert!(max_idx < config.n_mels - 1);

        println!(
            "Max energy for 1kHz sine at mel index: {}, value: {}",
            max_idx, max_val
        );
    }

    #[test]
    fn test_process_silence() {
        let config = MelSpectrogramConfig::default();
        let processor = MelProcessor::new(config.clone());
        let samples = vec![0.0; config.fft_size];
        let frame = processor.process_frame(&samples, 0);

        // All values should be very low (near -90dB due to epsilon)
        for &val in frame.data.iter() {
            assert!(val < -80.0);
        }
    }
}
