use half::f16;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter};

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct WaveformPoint {
    pub min: f16,
    pub max: f16,
    pub rms: f16,
}

impl WaveformPoint {
    pub fn new(min: f32, max: f32, rms: f32) -> Self {
        Self {
            min: f16::from_f32(min),
            max: f16::from_f32(max),
            rms: f16::from_f32(rms),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WaveformLOD {
    pub level: u8,
    pub samples_per_point: u32,
    pub data: Vec<WaveformPoint>,
}

#[derive(Serialize, Deserialize)]
pub struct PyramidCache {
    pub lods: Vec<WaveformLOD>,
}
impl PyramidCache {
    fn process_small_chunk(chunk: &[f32]) -> WaveformPoint {
        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;
        let mut sum_sq = 0.0;

        for &v in chunk {
            if v < min_val { min_val = v; }
            if v > max_val { max_val = v; }
            sum_sq += v * v;
        }

        if min_val == f32::MAX {
            min_val = 0.0;
            max_val = 0.0;
        }

        let rms = (sum_sq / chunk.len().max(1) as f32).sqrt();
        WaveformPoint::new(min_val, max_val, rms)
    }

    pub fn generate(samples: &[f32], _sample_rate: u32) -> Self {
        let lod0 = Self::generate_lod(samples, 16);   // NEW: 16 samples/pt for 10-50ms zoom
        let lod1 = Self::generate_lod(samples, 128);
        let lod2 = Self::generate_lod_from_lod(&lod1, 2048 / 128);
        let lod3 = Self::generate_lod_from_lod(&lod2, 65536 / 2048);
        Self { lods: vec![lod0, lod1, lod2, lod3] }
    }

    /// Find the nearest zero-crossing sample position at or after `start_sample`.
    /// Used by snap_loop_to_zero to prevent clicks/cracks at loop boundaries.
    pub fn find_zero_crossing(samples: &[f32], start_sample: usize, search_window: usize) -> usize {
        let end = (start_sample + search_window).min(samples.len().saturating_sub(1));
        // Search forward for sign change
        for i in start_sample..end {
            if i + 1 < samples.len() && samples[i] * samples[i + 1] <= 0.0 {
                // Prefer the sample closer to zero
                if samples[i].abs() <= samples[i + 1].abs() {
                    return i;
                } else {
                    return i + 1;
                }
            }
        }
        // Search backward as fallback
        let search_start = start_sample.saturating_sub(search_window);
        for i in (search_start..start_sample).rev() {
            if i + 1 < samples.len() && samples[i] * samples[i + 1] <= 0.0 {
                return i;
            }
        }
        start_sample
    }


    pub fn save_cache(&self, path: &std::path::Path) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        bincode::serialize_into(&mut writer, self).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_cache(path: &std::path::Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let cache: Self = bincode::deserialize_from(reader).map_err(|e| e.to_string())?;
        Ok(cache)
    }

    fn generate_lod(samples: &[f32], chunk_size: usize) -> WaveformLOD {
        let chunk_count = samples.len().div_ceil(chunk_size);
        let data = (0..chunk_count).into_par_iter().map(|i| {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(samples.len());
            Self::process_small_chunk(&samples[start..end])
        }).collect();

        WaveformLOD { level: 1, samples_per_point: chunk_size as u32, data }
    }

    fn generate_lod_from_lod(source: &WaveformLOD, reduction_factor: usize) -> WaveformLOD {
        let chunk_count = source.data.len().div_ceil(reduction_factor);
        let data = (0..chunk_count).into_par_iter().map(|i| {
            let start = i * reduction_factor;
            let end = (start + reduction_factor).min(source.data.len());
            let chunk = &source.data[start..end];
            
            let mut min_val = f32::MAX;
            let mut max_val = f32::MIN;
            let mut sum_rms_sq = 0.0;

            for pt in chunk {
                let min = pt.min.to_f32();
                let max = pt.max.to_f32();
                let rms = pt.rms.to_f32();
                if min < min_val { min_val = min; }
                if max > max_val { max_val = max; }
                sum_rms_sq += rms * rms;
            }
            WaveformPoint::new(min_val, max_val, (sum_rms_sq / chunk.len().max(1) as f32).sqrt())
        }).collect();

        WaveformLOD { level: source.level + 1, samples_per_point: source.samples_per_point * reduction_factor as u32, data }
    }
}
