use super::auto_gain::AutoGain;
use super::eq_band::{ChannelMode, EqBand};
use super::filters::TptSvfSimd;
use crate::engine::graph::{AudioBuffer, AudioProcessor, ProcessingContext};
use arc_swap::ArcSwap;
use std::sync::Arc;
use uuid::Uuid;
use wide::f64x2;

pub struct Equalizer {
    id: Uuid,
    bands: ArcSwap<Vec<EqBand>>,
    filters: Vec<TptSvfSimd>,
    solo_filter: TptSvfSimd,
    solo_band_index: Arc<std::sync::atomic::AtomicIsize>,
    auto_gain: AutoGain,
    sample_rate: f64,
}

impl Equalizer {
    pub fn new(sample_rate: f64) -> Self {
        let mut filters = Vec::new();

        for _ in 0..32 {
            filters.push(TptSvfSimd::new());
        }

        Self {
            id: Uuid::new_v4(),
            bands: ArcSwap::from_pointee(Vec::new()),
            filters,
            solo_filter: TptSvfSimd::new(),
            solo_band_index: Arc::new(std::sync::atomic::AtomicIsize::new(-1)),
            auto_gain: AutoGain::new(0.999),
            sample_rate,
        }
    }

    pub fn set_band_solo(&self, index: isize) {
        self.solo_band_index
            .store(index, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_bands(&self, bands: Vec<EqBand>) {
        self.bands.store(Arc::new(bands));
    }

    pub fn get_bands(&self) -> Vec<EqBand> {
        (**self.bands.load()).clone()
    }

    pub fn process_block(&mut self, left: &mut [f64], right: &mut [f64]) {
        let bands = self.bands.load();
        let len = std::cmp::min(left.len(), right.len());

        // 1. Pre-calculate coefficients
        let solo_idx_raw = self
            .solo_band_index
            .load(std::sync::atomic::Ordering::Relaxed);
        if solo_idx_raw >= 0 {
            let idx = solo_idx_raw as usize;
            if idx < bands.len() {
                let band = &bands[idx];
                // Configure Solo Filter as BandPass
                self.solo_filter.set_parameters(
                    band.freq,
                    band.q,
                    0.0, // Gain irrelevant for BP usually, or 0.0dB
                    super::eq_band::FilterType::BandPass,
                    self.sample_rate,
                );
            }
        }

        for (idx, band) in bands.iter().enumerate() {
            if idx < self.filters.len() {
                self.filters[idx].set_parameters(
                    band.freq,
                    band.q,
                    band.gain_db,
                    band.filter_type,
                    self.sample_rate,
                );
            }
        }

        // 2. Process samples
        for i in 0..len {
            let l_in = left[i];
            let r_in = right[i];
            let mut v = f64x2::from([l_in, r_in]);
            let original_l = l_in;
            let original_r = r_in;

            if solo_idx_raw >= 0 {
                // Solo Mode: Apply BandPass
                v = self.solo_filter.process(v);
            } else {
                // Apply active bands
                for (idx, band) in bands.iter().enumerate() {
                    if !band.enabled || idx >= self.filters.len() {
                        continue;
                    }

                    match band.mode {
                        ChannelMode::Stereo => {
                            v = self.filters[idx].process(v);
                        }
                        ChannelMode::Left => {
                            let out = self.filters[idx].process(v);
                            let arr_out: [f64; 2] = out.to_array();
                            let arr_in: [f64; 2] = v.to_array();
                            v = f64x2::from([arr_out[0], arr_in[1]]);
                        }
                        ChannelMode::Right => {
                            let out = self.filters[idx].process(v);
                            let arr_out: [f64; 2] = out.to_array();
                            let arr_in: [f64; 2] = v.to_array();
                            v = f64x2::from([arr_in[0], arr_out[1]]);
                        }
                        ChannelMode::Mid => {
                            let arr_prev: [f64; 2] = v.to_array();
                            let m = (arr_prev[0] + arr_prev[1]) * 0.5;
                            let s = (arr_prev[0] - arr_prev[1]) * 0.5;
                            let v_ms = f64x2::from([m, s]);
                            let out_ms = self.filters[idx].process(v_ms);
                            let arr_ms_out: [f64; 2] = out_ms.to_array();
                            let m_new = arr_ms_out[0];
                            let l_new = m_new + s;
                            let r_new = m_new - s;
                            v = f64x2::from([l_new, r_new]);
                        }
                        ChannelMode::Side => {
                            let arr_prev: [f64; 2] = v.to_array();
                            let m = (arr_prev[0] + arr_prev[1]) * 0.5;
                            let s = (arr_prev[0] - arr_prev[1]) * 0.5;
                            let v_ms = f64x2::from([m, s]);
                            let out_ms = self.filters[idx].process(v_ms);
                            let arr_ms_out: [f64; 2] = out_ms.to_array();
                            let s_new = arr_ms_out[1];
                            let l_new = m + s_new;
                            let r_new = m - s_new;
                            v = f64x2::from([l_new, r_new]);
                        }
                    }
                }
            }

            let res: [f64; 2] = v.to_array();
            let mut l_final = res[0];
            let mut r_final = res[1];

            // Apply auto-gain
            l_final = self.auto_gain.process(original_l, l_final);
            r_final = self.auto_gain.process(original_r, r_final);

            left[i] = l_final;
            right[i] = r_final;
        }
    }

    pub fn get_magnitude_curve(&self, frequencies: &[f64]) -> Vec<f64> {
        let bands = self.bands.load();
        let mut curve = vec![1.0; frequencies.len()];

        for (i, &f_target) in frequencies.iter().enumerate() {
            let mut total_mag = 1.0;
            for band in bands.iter() {
                if !band.enabled {
                    continue;
                }
                total_mag *= self.filters[0].get_magnitude(
                    f_target,
                    band.freq,
                    band.q,
                    band.gain_db,
                    band.filter_type,
                );
            }
            curve[i] = total_mag;
        }
        curve
    }
}

impl AudioProcessor for Equalizer {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        self.sample_rate = sample_rate;
        let (left, right) = buffer.get_stereo_mut();
        self.process_block(left, right);
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }

    fn name(&self) -> String {
        "Prisma EQ".to_string()
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
