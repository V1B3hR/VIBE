use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MelSpectrogramConfig {
    pub fft_size: usize,  // e.g., 2048
    pub hop_size: usize,  // e.g., 512
    pub n_mels: usize,    // e.g., 128
    pub sample_rate: u32, // e.g., 48000
    pub f_min: f32,       // e.g., 20.0
    pub f_max: f32,       // e.g., 20000.0
    pub power: f32,       // 2.0 for power, 1.0 for magnitude
}

impl Default for MelSpectrogramConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 512,
            n_mels: 128,
            sample_rate: 48000,
            f_min: 0.0,
            f_max: 24000.0,
            power: 2.0,
        }
    }
}
