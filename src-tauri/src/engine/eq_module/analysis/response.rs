use super::super::dsp::eq_band::EqBand;
use super::super::dsp::filters::TptSvf;

pub struct ResponseCurveGenerator {
    dummy_filter: TptSvf,
}

impl ResponseCurveGenerator {
    pub fn new() -> Self {
        Self {
            dummy_filter: TptSvf::new(),
        }
    }

    pub fn generate_curve(
        &self,
        bands: &[EqBand],
        _sample_rate: f64,
        num_points: usize,
    ) -> Vec<(f64, f64)> {
        let mut curve = Vec::with_capacity(num_points);

        // Logarithmic frequency scale from 20Hz to 20kHz
        let min_f: f64 = 20.0;
        let max_f: f64 = 20000.0;
        let log_min = min_f.log10();
        let log_max = max_f.log10();
        let step = (log_max - log_min) / (num_points - 1) as f64;

        for i in 0..num_points {
            let f_log = log_min + i as f64 * step;
            let f = 10.0f64.powf(f_log);

            let mut total_mag = 1.0;
            for band in bands {
                if !band.enabled {
                    continue;
                }
                total_mag *= self.dummy_filter.get_magnitude(
                    f,
                    band.freq,
                    band.q,
                    band.gain_db,
                    band.filter_type,
                );
            }

            let db = 20.0f64 * total_mag.log10();
            curve.push((f, db));
        }

        curve
    }
}
