use super::simd_avx512::SimdSummer;
use lazy_static::lazy_static;
use wide::f64x4;

lazy_static! {
    static ref SUMMER: SimdSummer = SimdSummer::new();
}

/// Optimized SIMD mixing with zero-copy operations.
/// Dynamically switches between AVX-512 and AVX2/NEON.
#[allow(dead_code)]
#[inline(always)]
pub fn mix_buffer_simd_optimized(dest: &mut [f64], src: &[f64]) {
    SUMMER.sum_buffers(dest, src);
}

/// Optimized SIMD mixing with per-buffer gain.
#[inline(always)]
pub fn mix_buffer_with_gain_simd_optimized(dest: &mut [f64], src: &[f64], gain: f64) {
    SUMMER.sum_buffers_with_gain(dest, src, gain);
}

/// Apply scalar gain to buffer using SIMD.
#[inline(always)]
pub fn apply_gain_simd_optimized(buffer: &mut [f64], gain: f64) {
    SUMMER.multiply_scalar(buffer, gain);
}

/// Optimized hard limiter with SIMD support.
/// Includes NaN/Inf protection and hard clipping.
#[inline(always)]
pub fn apply_limiter_optimized(buffer: &mut [f64]) {
    let len = buffer.len();
    let simd_len = len / 4 * 4;
    let one = f64x4::splat(1.0);
    let neg_one = f64x4::splat(-1.0);

    for i in (0..simd_len).step_by(4) {
        unsafe {
            let ptr = buffer.as_mut_ptr().add(i);
            let v = f64x4::new([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]);

            let limited = v.min(one).max(neg_one);
            let res_arr = limited.to_array();

            // Sanity check for NaNs during store
            *ptr = if res_arr[0].is_finite() {
                res_arr[0]
            } else {
                0.0
            };
            *ptr.add(1) = if res_arr[1].is_finite() {
                res_arr[1]
            } else {
                0.0
            };
            *ptr.add(2) = if res_arr[2].is_finite() {
                res_arr[2]
            } else {
                0.0
            };
            *ptr.add(3) = if res_arr[3].is_finite() {
                res_arr[3]
            } else {
                0.0
            };
        }
    }

    for i in simd_len..len {
        let mut x = buffer[i];
        if !x.is_finite() {
            x = 0.0;
        }
        buffer[i] = x.clamp(-1.0, 1.0);
    }
}

/// Fast tanh approximation using Padé approximant.
/// Accurate to ~0.1% for |x| < 3, which covers 99.9% of audio signals.
/// 3-5x faster than std::tanh.
#[inline(always)]
fn fast_tanh(x: f64) -> f64 {
    if x.abs() > 3.0 {
        return x.signum(); // Hard clip for extreme values
    }

    // Padé [3/3] approximation
    let x2 = x * x;
    let num = x * (135135.0 + x2 * (17325.0 + x2 * (378.0 + x2)));
    let den = 135135.0 + x2 * (62370.0 + x2 * (3150.0 + x2 * 28.0));
    num / den
}

/// Optimized saturation with fast tanh approximation.
#[inline(always)]
pub fn apply_saturation_optimized(buffer: &mut [f64], warmth: f64) {
    let one_plus_warmth = 1.0 + warmth;
    let asym = 0.05 * warmth;
    let correction = asym * 0.1;

    let len = buffer.len();
    let simd_len = len / 4 * 4;

    // SIMD path
    for i in (0..simd_len).step_by(4) {
        unsafe {
            let ptr = buffer.as_mut_ptr().add(i);

            // Load
            let v_in = f64x4::new([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]);

            // Asymmetry
            let v_squared = v_in * v_in;
            let v_asym = v_in + (v_squared * f64x4::splat(asym));
            let v_driven = v_asym * f64x4::splat(one_plus_warmth);

            // Fast tanh (scalar fallback, but with fast approximation)
            let arr_driven = v_driven.to_array();
            let arr_out = [
                fast_tanh(arr_driven[0]),
                fast_tanh(arr_driven[1]),
                fast_tanh(arr_driven[2]),
                fast_tanh(arr_driven[3]),
            ];

            let v_out = f64x4::from(arr_out) - f64x4::splat(correction);
            let res = v_out.to_array();

            // Store
            *ptr = res[0];
            *ptr.add(1) = res[1];
            *ptr.add(2) = res[2];
            *ptr.add(3) = res[3];
        }
    }

    // Scalar fallback
    for i in simd_len..len {
        let x = buffer[i];
        let x_asym = x + (asym * x * x);
        buffer[i] = fast_tanh(x_asym * one_plus_warmth) - correction;
    }
}

/// Sums stereo L/R buffers into a mono buffer with 0.5 scaling.
#[inline(always)]
pub fn sum_stereo_to_mono_optimized(dest: &mut [f64], src_l: &[f64], src_r: &[f64]) {
    let len = dest.len();
    let simd_len = len / 4 * 4;
    let half = f64x4::splat(0.5);

    for i in (0..simd_len).step_by(4) {
        let l = f64x4::new([src_l[i], src_l[i + 1], src_l[i + 2], src_l[i + 3]]);
        let r = f64x4::new([src_r[i], src_r[i + 1], src_r[i + 2], src_r[i + 3]]);
        let mono = (l + r) * half;
        let res = mono.to_array();
        dest[i] = res[0];
        dest[i + 1] = res[1];
        dest[i + 2] = res[2];
        dest[i + 3] = res[3];
    }

    for i in simd_len..len {
        dest[i] = (src_l[i] + src_r[i]) * 0.5;
    }
}

/// Apply width (MS) processing to stereo buffers.
/// Width 1.0 = No change, 0.0 = Mono, 2.0 = Extra Wide.
#[inline(always)]
pub fn apply_width_optimized(l: &mut [f64], r: &mut [f64], width: f64) {
    if (width - 1.0).abs() < 0.001 {
        return;
    }

    let len = l.len();
    let simd_len = len / 4 * 4;
    let w = f64x4::splat(width);
    let half = f64x4::splat(0.5);

    for i in (0..simd_len).step_by(4) {
        let vl = f64x4::new([l[i], l[i + 1], l[i + 2], l[i + 3]]);
        let vr = f64x4::new([r[i], r[i + 1], r[i + 2], r[i + 3]]);

        let m = (vl + vr) * half;
        let s = (vl - vr) * half * w;

        let res_l = (m + s).to_array();
        let res_r = (m - s).to_array();

        l[i] = res_l[0];
        l[i + 1] = res_l[1];
        l[i + 2] = res_l[2];
        l[i + 3] = res_l[3];

        r[i] = res_r[0];
        r[i + 1] = res_r[1];
        r[i + 2] = res_r[2];
        r[i + 3] = res_r[3];
    }

    for i in simd_len..len {
        let m = (l[i] + r[i]) * 0.5;
        let s = (l[i] - r[i]) * 0.5 * width;
        l[i] = m + s;
        r[i] = m - s;
    }
}

/// Apply pan (stereo balance) to stereo buffers.
/// Pan: -1.0 (Left) to 1.0 (Right), 0.0 (Center).
#[inline(always)]
pub fn apply_pan_optimized(l: &mut [f64], r: &mut [f64], pan: f64) {
    if pan.abs() < 0.001 {
        return;
    }

    let len = l.len();
    let simd_len = len / 4 * 4;

    // Linear pan Law
    let pan_l = (1.0 - pan).min(1.0);
    let pan_r = (1.0 + pan).min(1.0);

    let vl_gain = f64x4::splat(pan_l);
    let vr_gain = f64x4::splat(pan_r);

    for i in (0..simd_len).step_by(4) {
        let vl = f64x4::new([l[i], l[i + 1], l[i + 2], l[i + 3]]);
        let vr = f64x4::new([r[i], r[i + 1], r[i + 2], r[i + 3]]);

        let res_l = (vl * vl_gain).to_array();
        let res_r = (vr * vr_gain).to_array();

        l[i] = res_l[0];
        l[i + 1] = res_l[1];
        l[i + 2] = res_l[2];
        l[i + 3] = res_l[3];

        r[i] = res_r[0];
        r[i + 1] = res_r[1];
        r[i + 2] = res_r[2];
        r[i + 3] = res_r[3];
    }

    for i in simd_len..len {
        l[i] *= pan_l;
        r[i] *= pan_r;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_tanh_accuracy() {
        // Test fast_tanh against std::tanh
        for i in -30..=30 {
            let x = i as f64 * 0.1;
            let fast = fast_tanh(x);
            let exact = x.tanh();
            let error = (fast - exact).abs();

            // Should be accurate to 0.1% for |x| < 3
            if x.abs() < 3.0 {
                assert!(
                    error < 0.001,
                    "Error too large at x={}: {} vs {}",
                    x,
                    fast,
                    exact
                );
            }
        }
    }

    #[test]
    fn test_mix_buffer_optimized() {
        let mut dest = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let src = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];

        mix_buffer_simd_optimized(&mut dest, &src);

        for i in 0..8 {
            assert!((dest[i] - (i as f64 + 1.5)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_saturation_optimized() {
        let mut buffer = vec![0.5, 1.0, 1.5, 2.0, -0.5, -1.0, -1.5, -2.0];
        apply_saturation_optimized(&mut buffer, 0.05);

        // Should apply saturation without clipping
        for &sample in &buffer {
            assert!(sample.abs() <= 1.0, "Saturation should prevent clipping");
        }
    }

    #[test]
    fn test_non_multiple_of_4() {
        // Test with buffer size not divisible by 4
        let mut dest = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 5 samples
        let src = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        mix_buffer_simd_optimized(&mut dest, &src);

        assert!((dest[0] - 1.1).abs() < 1e-10);
        assert!((dest[4] - 5.5).abs() < 1e-10);
    }
}
