#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// 7.1.4 Surround Channel Layout Output Gain Coefficients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurroundGains714 {
    pub left: f32,
    pub right: f32,
    pub center: f32,
    pub lfe: f32,
    pub left_surround: f32,
    pub right_surround: f32,
    pub left_top_front: f32,
    pub right_top_front: f32,
    pub left_top_back: f32,
    pub right_top_back: f32,
}

/// Headphone Binaural HRTF Stereo Gain Pair (with ITD / ILD simulation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinauralHrtfGains {
    pub left_gain: f32,
    pub right_gain: f32,
    pub itd_samples_left: f32,
    pub itd_samples_right: f32,
}

/// 3D Spatial Panner node operating in Cartesian coordinates (-1.0 <= x, y, z <= 1.0)
pub struct SpatialPanner3D;

impl SpatialPanner3D {
    /// Calculate 7.1.4 Surround Channel Gains using 3D VBAP amplitude panning
    /// x: Left (-1.0) to Right (+1.0)
    /// y: Back (-1.0) to Front (+1.0)
    /// z: Floor (-1.0) to Ceiling (+1.0)
    pub fn calculate_714_gains(x: f32, y: f32, z: f32) -> SurroundGains714 {
        let x = x.clamp(-1.0, 1.0);
        let y = y.clamp(-1.0, 1.0);
        let z = z.clamp(-1.0, 1.0);

        // Horizontal panning
        let left_weight = ((1.0 - x) * 0.5).clamp(0.0, 1.0);
        let right_weight = ((1.0 + x) * 0.5).clamp(0.0, 1.0);

        // Front / Back panning
        let front_weight = ((1.0 + y) * 0.5).clamp(0.0, 1.0);
        let back_weight = ((1.0 - y) * 0.5).clamp(0.0, 1.0);

        // Height / Top panning
        let height_weight = ((1.0 + z) * 0.5).clamp(0.0, 1.0);
        let floor_weight = ((1.0 - z) * 0.5).clamp(0.0, 1.0);

        // Center channel extraction when y > 0 and |x| < 0.3
        let center_weight = if y > 0.0 && x.abs() < 0.3 {
            (1.0 - (x.abs() / 0.3)) * front_weight
        } else {
            0.0
        };

        // LFE sub channel (non-directional low-pass weight)
        let lfe = 0.1;

        let left = left_weight * front_weight * floor_weight;
        let right = right_weight * front_weight * floor_weight;
        let left_surround = left_weight * back_weight * floor_weight;
        let right_surround = right_weight * back_weight * floor_weight;

        let left_top_front = left_weight * front_weight * height_weight;
        let right_top_front = right_weight * front_weight * height_weight;
        let left_top_back = left_weight * back_weight * height_weight;
        let right_top_back = right_weight * back_weight * height_weight;

        // Energy normalization (sum of squares = 1.0)
        let total_sq = left * left
            + right * right
            + center_weight * center_weight
            + left_surround * left_surround
            + right_surround * right_surround
            + left_top_front * left_top_front
            + right_top_front * right_top_front
            + left_top_back * left_top_back
            + right_top_back * right_top_back
            + 1e-6;

        let norm = 1.0 / total_sq.sqrt();

        SurroundGains714 {
            left: left * norm,
            right: right * norm,
            center: center_weight * norm,
            lfe,
            left_surround: left_surround * norm,
            right_surround: right_surround * norm,
            left_top_front: left_top_front * norm,
            right_top_front: right_top_front * norm,
            left_top_back: left_top_back * norm,
            right_top_back: right_top_back * norm,
        }
    }

    /// Calculate HRTF Binaural Headphone Gains with Interaural Time Difference (ITD) and Level Difference (ILD)
    pub fn calculate_binaural_hrtf(x: f32, y: f32, z: f32) -> BinauralHrtfGains {
        let x = x.clamp(-1.0, 1.0);
        let distance = (x * x + y * y + z * z).sqrt().clamp(0.1, 2.0);

        // ILD (Interaural Level Difference)
        let left_gain = (((1.0 - x) * 0.5) / distance).clamp(0.0, 1.0);
        let right_gain = (((1.0 + x) * 0.5) / distance).clamp(0.0, 1.0);

        // ITD (Interaural Time Difference: ~0.65ms max head delay = ~31 samples @ 48kHz)
        let itd_max_samples = 31.0f32;
        let itd_left = if x > 0.0 { x * itd_max_samples } else { 0.0 };
        let itd_right = if x < 0.0 { -x * itd_max_samples } else { 0.0 };

        BinauralHrtfGains {
            left_gain,
            right_gain,
            itd_samples_left: itd_left,
            itd_samples_right: itd_right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_714_surround_energy_normalization() {
        let gains = SpatialPanner3D::calculate_714_gains(0.5, 0.5, 0.5);
        let sum_sq = gains.left * gains.left
            + gains.right * gains.right
            + gains.center * gains.center
            + gains.left_surround * gains.left_surround
            + gains.right_surround * gains.right_surround
            + gains.left_top_front * gains.left_top_front
            + gains.right_top_front * gains.right_top_front
            + gains.left_top_back * gains.left_top_back
            + gains.right_top_back * gains.right_top_back;

        assert!((sum_sq - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_binaural_hrtf_itd_ild() {
        // Hard Left Panning (-1.0)
        let binaural = SpatialPanner3D::calculate_binaural_hrtf(-1.0, 0.0, 0.0);
        assert!(binaural.left_gain > binaural.right_gain);
        assert!(binaural.itd_samples_right > 0.0);
        assert_eq!(binaural.itd_samples_left, 0.0);
    }
}
