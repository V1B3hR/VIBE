use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MelFrame {
    pub data: Vec<f32>, // n_mels values (log-magnitude)
    pub timestamp_samples: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralAnalysisResult {
    pub frames: Vec<MelFrame>,
    pub duration_samples: u64,
}
