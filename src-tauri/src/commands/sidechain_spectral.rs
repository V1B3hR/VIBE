use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidechainMaskingFrame {
    pub track_a_fft: Vec<f32>,
    pub track_b_fft: Vec<f32>,
    pub collision_mask: Vec<bool>,
}

#[tauri::command]
pub fn get_sidechain_spectrum_comparison(
    track_a_idx: usize,
    track_b_idx: usize,
) -> Result<SidechainMaskingFrame, String> {
    let bands = 128;
    let mut track_a_fft = vec![-60.0f32; bands];
    let mut track_b_fft = vec![-60.0f32; bands];
    let mut collision_mask = vec![false; bands];

    // Simulated 128-band FFT data comparison
    for b in 20..60 {
        let val_a = -12.0 + (b as f32 * 0.1).sin() * 5.0;
        let val_b = -14.0 + (b as f32 * 0.1).cos() * 5.0;
        track_a_fft[b] = val_a;
        track_b_fft[b] = val_b;

        // Flag collision if both signals > -24dB and difference < 3dB
        if val_a > -24.0 && val_b > -24.0 && (val_a - val_b).abs() < 3.0 {
            collision_mask[b] = true;
        }
    }

    let _ = (track_a_idx, track_b_idx);

    Ok(SidechainMaskingFrame {
        track_a_fft,
        track_b_fft,
        collision_mask,
    })
}
