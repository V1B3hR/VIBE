use super::eq_band::FilterType;
use num_complex::Complex;
use std::f64::consts::PI;
use wide::f64x2;

#[derive(Clone, Copy)]
pub struct TptSvfSimd {
    s1: f64x2,
    s2: f64x2,

    // Cached Coefficients (Simd)
    g: f64x2,
    g_inv: f64x2,
    r: f64x2,
    a: f64x2,
    a2: f64x2,
    filter_type: FilterType,
}

impl TptSvfSimd {
    pub fn new() -> Self {
        Self {
            s1: f64x2::splat(0.0),
            s2: f64x2::splat(0.0),
            g: f64x2::splat(0.0),
            g_inv: f64x2::splat(0.0),
            r: f64x2::splat(0.0),
            a: f64x2::splat(1.0),
            a2: f64x2::splat(1.0),
            filter_type: FilterType::LowPass,
        }
    }

    pub fn reset(&mut self) {
        self.s1 = f64x2::splat(0.0);
        self.s2 = f64x2::splat(0.0);
    }

    /// Pre-calculate coefficients (Call this once per block or when params change)
    pub fn set_parameters(
        &mut self,
        f: f64,
        q: f64,
        gain_db: f64,
        filter_type: FilterType,
        fs: f64,
    ) {
        self.filter_type = filter_type;

        let g_scalar = (PI * f / fs).tan();
        let a_scalar = (gain_db / 40.0).powf(10.0);
        let a2_scalar = a_scalar * a_scalar;

        // Broadcast common values
        self.a = f64x2::splat(a_scalar);
        self.a2 = f64x2::splat(a2_scalar);

        match filter_type {
            FilterType::Bell => {
                let r = 1.0 / (q * a_scalar);
                let g_inv = 1.0 / (1.0 + r * g_scalar + g_scalar * g_scalar);

                self.g = f64x2::splat(g_scalar);
                self.r = f64x2::splat(r);
                self.g_inv = f64x2::splat(g_inv);
            }
            FilterType::LowPass
            | FilterType::HighPass
            | FilterType::Notch
            | FilterType::BandPass => {
                let r = 1.0 / q;
                let g_inv = 1.0 / (1.0 + r * g_scalar + g_scalar * g_scalar);

                self.g = f64x2::splat(g_scalar);
                self.r = f64x2::splat(r);
                self.g_inv = f64x2::splat(g_inv);
            }
            FilterType::LowShelf => {
                let r = 1.0 / q;
                let g_scaled = g_scalar / a_scalar.sqrt();
                let g_inv_shelf = 1.0 / (1.0 + r * g_scaled + g_scaled * g_scaled);

                self.g = f64x2::splat(g_scaled); // Use scaled g
                self.r = f64x2::splat(r);
                self.g_inv = f64x2::splat(g_inv_shelf);
            }
            FilterType::HighShelf => {
                let r = 1.0 / q;
                let g_scaled = g_scalar * a_scalar.sqrt();
                let g_inv_shelf = 1.0 / (1.0 + r * g_scaled + g_scaled * g_scaled);

                self.g = f64x2::splat(g_scaled); // Use scaled g
                self.r = f64x2::splat(r);
                self.g_inv = f64x2::splat(g_inv_shelf);
            }
        }
    }

    #[inline(always)]
    pub fn process(&mut self, x: f64x2) -> f64x2 {
        // Shared Topology
        // hp = (x - (r + g) * s1 - s2) * g_inv
        let hp = (x - (self.r + self.g) * self.s1 - self.s2) * self.g_inv;
        let bp = self.g * hp + self.s1;
        let lp = self.g * bp + self.s2;

        // State Update
        self.s1 = self.g * hp + bp;
        self.s2 = self.g * bp + lp;

        // Output Mixer
        match self.filter_type {
            FilterType::Bell => {
                // x + r * (a^2 - 1) * bp
                x + self.r * (self.a2 - f64x2::splat(1.0)) * bp
            }
            FilterType::LowPass => lp,
            FilterType::HighPass => hp,
            FilterType::BandPass => bp,
            FilterType::Notch => hp + lp,
            FilterType::LowShelf => {
                // x + (a^2 - 1)*lp + r*(a - 1)*bp
                x + (self.a2 - f64x2::splat(1.0)) * lp + self.r * (self.a - f64x2::splat(1.0)) * bp
            }
            FilterType::HighShelf => {
                // a^2*x + (1 - a^2)*lp + r*a*(1 - a)*bp
                self.a2 * x
                    + (f64x2::splat(1.0) - self.a2) * lp
                    + self.r * self.a * (f64x2::splat(1.0) - self.a) * bp
            }
        }
    }

    /// Calculate magnitude response at a given frequency
    pub fn get_magnitude(
        &self,
        f_target: f64,
        f_cutoff: f64,
        q: f64,
        gain_db: f64,
        filter_type: FilterType,
    ) -> f64 {
        let w = 2.0 * PI * f_target;
        let wc = 2.0 * PI * f_cutoff;
        let s = Complex::new(0.0, w);
        let a = (gain_db / 40.0).powf(10.0);
        let a2 = a * a;

        // Analog transfer functions for SVF
        let h = match filter_type {
            FilterType::LowPass => {
                let denom = s * s + (wc / q) * s + wc * wc;
                wc * wc / denom
            }
            FilterType::HighPass => {
                let denom = s * s + (wc / q) * s + wc * wc;
                s * s / denom
            }
            FilterType::BandPass => {
                let denom = s * s + (wc / q) * s + wc * wc;
                (s * (wc / q)) / denom
            }
            FilterType::Bell => {
                let bp = (wc / (q * a)) * s / (s * s + (wc / (q * a)) * s + wc * wc);
                let gain_factor = a2 - 1.0;
                Complex::new(1.0, 0.0) + Complex::new(gain_factor, 0.0) * bp
            }
            FilterType::LowShelf => {
                let numer = s * s + (a * wc / q) * s + a2 * wc * wc;
                let denom = s * s + (wc / (q * a)) * s + wc * wc;
                numer / denom
            }
            FilterType::HighShelf => {
                let numer = a2 * s * s + (a * wc / q) * s + wc * wc;
                let denom = s * s + (wc / (q * a)) * s + wc * wc;
                numer / denom
            }
            FilterType::Notch => {
                let denom = s * s + (wc / q) * s + wc * wc;
                (s * s + wc * wc) / denom
            }
        };

        h.norm()
    }
}

pub struct TptSvf {
    s1: f64,
    s2: f64,
}

impl TptSvf {
    pub fn new() -> Self {
        Self { s1: 0.0, s2: 0.0 }
    }

    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    /// Process one sample
    /// f: Frequency in Hz
    /// q: Q factor
    /// gain_db: Gain in dB
    /// fs: Sample rate
    pub fn process(
        &mut self,
        x: f64,
        f: f64,
        q: f64,
        gain_db: f64,
        filter_type: FilterType,
        fs: f64,
    ) -> f64 {
        let g = (PI * f / fs).tan();
        let a = (gain_db / 40.0).powf(10.0); // Square root of linear gain

        match filter_type {
            FilterType::Bell => {
                let r = 1.0 / (q * a);
                let g_inv = 1.0 / (1.0 + r * g + g * g);

                let hp = (x - (r + g) * self.s1 - self.s2) * g_inv;
                let bp = g * hp + self.s1;
                let lp = g * bp + self.s2;

                self.s1 = g * hp + bp;
                self.s2 = g * bp + lp;

                x + r * (a * a - 1.0) * bp
            }
            FilterType::LowPass => {
                let r = 1.0 / q;
                let g_inv = 1.0 / (1.0 + r * g + g * g);

                let hp = (x - (r + g) * self.s1 - self.s2) * g_inv;
                let bp = g * hp + self.s1;
                let lp = g * bp + self.s2;

                self.s1 = g * hp + bp;
                self.s2 = g * bp + lp;

                lp
            }
            FilterType::BandPass => {
                let r = 1.0 / q;
                let g_inv = 1.0 / (1.0 + r * g + g * g);

                let hp = (x - (r + g) * self.s1 - self.s2) * g_inv;
                let bp = g * hp + self.s1;
                let lp = g * bp + self.s2;

                self.s1 = g * hp + bp;
                self.s2 = g * bp + lp;

                bp
            }
            FilterType::HighPass => {
                let r = 1.0 / q;
                let g_inv = 1.0 / (1.0 + r * g + g * g);

                let hp = (x - (r + g) * self.s1 - self.s2) * g_inv;
                let bp = g * hp + self.s1;
                let lp = g * bp + self.s2;

                self.s1 = g * hp + bp;
                self.s2 = g * bp + lp;

                hp
            }
            FilterType::LowShelf => {
                let r = 1.0 / q;
                // let g_inv = 1.0 / (1.0 + r * g + g * g);

                let a2 = a * a;
                let g_scaled = g / a.sqrt();
                let g_inv_shelf = 1.0 / (1.0 + r * g_scaled + g_scaled * g_scaled);

                let hp = (x - (r + g_scaled) * self.s1 - self.s2) * g_inv_shelf;
                let bp = g_scaled * hp + self.s1;
                let lp = g_scaled * bp + self.s2;

                self.s1 = g_scaled * hp + bp;
                self.s2 = g_scaled * bp + lp;

                x + (a2 - 1.0) * lp + r * (a - 1.0) * bp
            }
            FilterType::HighShelf => {
                let r = 1.0 / q;
                let a2 = a * a;
                let g_scaled = g * a.sqrt();
                let g_inv_shelf = 1.0 / (1.0 + r * g_scaled + g_scaled * g_scaled);

                let hp = (x - (r + g_scaled) * self.s1 - self.s2) * g_inv_shelf;
                let bp = g_scaled * hp + self.s1;
                let lp = g_scaled * bp + self.s2;

                self.s1 = g_scaled * hp + bp;
                self.s2 = g_scaled * bp + lp;

                a2 * x + (1.0 - a2) * lp + r * a * (1.0 - a) * bp
            }
            FilterType::Notch => {
                let r = 1.0 / q;
                let g_inv = 1.0 / (1.0 + r * g + g * g);

                let hp = (x - (r + g) * self.s1 - self.s2) * g_inv;
                let bp = g * hp + self.s1;
                let lp = g * bp + self.s2;

                self.s1 = g * hp + bp;
                self.s2 = g * bp + lp;

                hp + lp
            }
        }
    }

    /// Calculate magnitude response at a given frequency
    pub fn get_magnitude(
        &self,
        f_target: f64,
        f_cutoff: f64,
        q: f64,
        gain_db: f64,
        filter_type: FilterType,
    ) -> f64 {
        let w = 2.0 * PI * f_target;
        let wc = 2.0 * PI * f_cutoff;
        let s = Complex::new(0.0, w);
        let a = (gain_db / 40.0).powf(10.0);

        // Analog transfer functions for SVF
        let h = match filter_type {
            FilterType::LowPass => {
                let denom = s * s + (wc / q) * s + wc * wc;
                wc * wc / denom
            }
            FilterType::HighPass => {
                let denom = s * s + (wc / q) * s + wc * wc;
                s * s / denom
            }
            FilterType::BandPass => {
                let denom = s * s + (wc / q) * s + wc * wc;
                (s * (wc / q)) / denom
            }
            FilterType::Bell => {
                let bp = (wc / (q * a)) * s / (s * s + (wc / (q * a)) * s + wc * wc);
                let gain_factor = a * a - 1.0;
                Complex::new(1.0, 0.0) + Complex::new(gain_factor, 0.0) * bp
            }
            FilterType::LowShelf => {
                let a2 = a * a;
                let numer = s * s + (a * wc / q) * s + a2 * wc * wc;
                let denom = s * s + (wc / (q * a)) * s + wc * wc;
                numer / denom
            }
            FilterType::HighShelf => {
                let a2 = a * a;
                let numer = a2 * s * s + (a * wc / q) * s + wc * wc;
                let denom = s * s + (wc / (q * a)) * s + wc * wc;
                numer / denom
            }
            FilterType::Notch => {
                let denom = s * s + (wc / q) * s + wc * wc;
                (s * s + wc * wc) / denom
            }
        };

        h.norm()
    }
}
