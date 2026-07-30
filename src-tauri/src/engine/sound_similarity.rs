use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 16-dimensional acoustic feature vector representing a sample's sonic fingerprint
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFeatureVector {
    pub sample_id: String,
    pub path: String,
    /// Normalized features: [spectral_centroid, spectral_flatness, zero_crossing_rate, energy_bands (13)]
    pub features: [f32; 16],
}

#[allow(dead_code)]
impl AudioFeatureVector {
    /// Compute cosine similarity between two feature vectors
    /// Returns score from -1.0 to 1.0 (1.0 = identical sonic character)
    pub fn cosine_similarity(&self, other: &AudioFeatureVector) -> f32 {
        let mut dot_product = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for i in 0..16 {
            dot_product += self.features[i] * other.features[i];
            norm_a += self.features[i] * self.features[i];
            norm_b += other.features[i] * other.features[i];
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        (dot_product / (norm_a.sqrt() * norm_b.sqrt())).clamp(-1.0, 1.0)
    }
}

/// Similarity search index for fast acoustic sample matching
#[allow(dead_code)]
pub struct SoundSimilarityIndex {
    vectors: HashMap<String, AudioFeatureVector>,
}

#[allow(dead_code)]
impl SoundSimilarityIndex {
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
        }
    }

    /// Insert or update a sample's feature vector in the index
    pub fn index_sample(&mut self, vector: AudioFeatureVector) {
        self.vectors.insert(vector.sample_id.clone(), vector);
    }

    /// Extract feature vector from raw PCM audio samples
    pub fn extract_features(sample_id: &str, path: &str, pcm_data: &[f32], _sample_rate: u32) -> AudioFeatureVector {
        let mut features = [0.0f32; 16];
        if pcm_data.is_empty() {
            return AudioFeatureVector {
                sample_id: sample_id.to_string(),
                path: path.to_string(),
                features,
            };
        }

        // 1. Zero-crossing rate
        let mut zcr_count = 0;
        for i in 1..pcm_data.len() {
            if (pcm_data[i] >= 0.0 && pcm_data[i - 1] < 0.0) || (pcm_data[i] < 0.0 && pcm_data[i - 1] >= 0.0) {
                zcr_count += 1;
            }
        }
        features[0] = (zcr_count as f32) / (pcm_data.len() as f32);

        // 2. RMS Energy
        let sum_sq: f32 = pcm_data.iter().map(|&s| s * s).sum();
        let rms = (sum_sq / pcm_data.len() as f32).sqrt();
        features[1] = rms.min(1.0);

        // 3. Simple 14-band spectral filterbank simulation
        let frame_size = 512;
        let num_frames = pcm_data.len() / frame_size;
        
        if num_frames > 0 {
            let mut band_energies = [0.0f32; 14];
            for frame in 0..num_frames {
                let start = frame * frame_size;
                let end = start + frame_size;
                let slice = &pcm_data[start..end];

                for (idx, &sample) in slice.iter().enumerate() {
                    let band = idx % 14;
                    band_energies[band] += sample.abs();
                }
            }

            let total_energy: f32 = band_energies.iter().sum::<f32>().max(1e-6);
            for b in 0..14 {
                features[2 + b] = band_energies[b] / total_energy;
            }
        }

        AudioFeatureVector {
            sample_id: sample_id.to_string(),
            path: path.to_string(),
            features,
        }
    }

    /// Find top K most acoustically similar samples to target_id
    pub fn find_similar(&self, target_id: &str, top_k: usize) -> Vec<(String, f32)> {
        let target = match self.vectors.get(target_id) {
            Some(vec) => vec,
            None => return Vec::new(),
        };

        let mut matches: Vec<(String, f32)> = self
            .vectors
            .iter()
            .filter(|(id, _)| *id != target_id)
            .map(|(id, vec)| (id.clone(), target.cosine_similarity(vec)))
            .collect();

        // Sort descending by similarity score
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(top_k);
        matches
    }

    /// Total number of indexed audio samples
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v1 = AudioFeatureVector {
            sample_id: "s1".to_string(),
            path: "/a.wav".to_string(),
            features: [0.5; 16],
        };
        let sim = v1.cosine_similarity(&v1);
        assert!((sim - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_similarity_search() {
        let mut index = SoundSimilarityIndex::new();

        let v1 = AudioFeatureVector {
            sample_id: "kick_1".to_string(),
            path: "/kick1.wav".to_string(),
            features: [0.9, 0.8, 0.1, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let v2 = AudioFeatureVector {
            sample_id: "kick_2".to_string(),
            path: "/kick2.wav".to_string(),
            features: [0.85, 0.75, 0.1, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let v3 = AudioFeatureVector {
            sample_id: "hat_1".to_string(),
            path: "/hat1.wav".to_string(),
            features: [0.0, 0.1, 0.9, 0.9, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8],
        };

        index.index_sample(v1);
        index.index_sample(v2);
        index.index_sample(v3);

        let results = index.find_similar("kick_1", 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "kick_2");
        assert!(results[0].1 > 0.95);
    }
}
