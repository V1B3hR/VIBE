#![allow(dead_code)]
use wide::f64x4;

/// Platform-agnostic SIMD abstraction for f64 audio processing.
/// Currently maps to AVX2/NEON 256-bit (4x f64) via the `wide` crate.
/// Future-proof readiness: 2026+
pub const SIMD_WIDTH: usize = 4;
pub type V64 = f64x4;

/// Mix source buffer into destination buffer with gain.
/// Uses 4-wide SIMD instructions.
#[inline(always)]
pub fn mix_buffer_simd(dest: &mut [f64], src: &[f64]) {
    let mut dest_chunks = dest.chunks_exact_mut(4);
    let mut src_chunks = src.chunks_exact(4);

    while let (Some(d), Some(s)) = (dest_chunks.next(), src_chunks.next()) {
        let mut d_arr = [0.0; 4];
        d_arr.copy_from_slice(d);
        let mut s_arr = [0.0; 4];
        s_arr.copy_from_slice(s);

        let dv = V64::from(d_arr);
        let sv = V64::from(s_arr);
        let res = dv + sv;
        d.copy_from_slice(&res.to_array());
    }

    let dest_rem = dest_chunks.into_remainder();
    let src_rem = src_chunks.remainder();

    for (d, s) in dest_rem.iter_mut().zip(src_rem.iter()) {
        *d += *s;
    }
}

/// Apply scalar gain to buffer using SIMD.
#[inline(always)]
pub fn apply_gain_simd(buffer: &mut [f64], gain: f64) {
    let g = V64::splat(gain);
    let mut chunks = buffer.chunks_exact_mut(4);

    for chunk in chunks.by_ref() {
        let mut arr = [0.0; 4];
        arr.copy_from_slice(chunk);

        let v = V64::from(arr);
        let res = v * g;
        chunk.copy_from_slice(&res.to_array());
    }

    for sample in chunks.into_remainder().iter_mut() {
        *sample *= gain;
    }
}

/// The "Maybach" Saturation Kernel.
/// Applies quadratic asymmetry and soft-clipping.
/// Currently uses a high-precision scalar tanh fallback,
/// but structure allows for easy swap to vectorized tanh approximation.
#[inline(always)]
pub fn apply_saturation_kernel(buffer: &mut [f64], warmth: f64) {
    let one_plus_warmth = V64::splat(1.0 + warmth);
    let asym = V64::splat(0.05 * warmth);
    let correction = V64::splat(0.05 * warmth * 0.1);

    let mut chunks = buffer.chunks_exact_mut(4);

    for chunk in chunks.by_ref() {
        let mut arr = [0.0; 4];
        arr.copy_from_slice(chunk);
        let v_in = V64::from(arr);

        // Asymmetry: x + (0.05 * warmth * x^2)
        let v_squared = v_in * v_in;
        let v_asym = v_in + (v_squared * asym);

        let v_driven = v_asym * one_plus_warmth;

        // Vector-to-Scalar Tanh Fallback
        let arr_driven = v_driven.to_array();
        let mut arr_out = [0.0; 4];

        arr_out[0] = arr_driven[0].tanh();
        arr_out[1] = arr_driven[1].tanh();
        arr_out[2] = arr_driven[2].tanh();
        arr_out[3] = arr_driven[3].tanh();

        let v_out = V64::from(arr_out) - correction;
        chunk.copy_from_slice(&v_out.to_array());
    }

    // Scalar fallback
    let scalar_asym = 0.05 * warmth;
    let scalar_correction = scalar_asym * 0.1;
    let scalar_drive = 1.0 + warmth;

    for sample in chunks.into_remainder().iter_mut() {
        let x = *sample;
        let x_asym = x + (scalar_asym * x * x);
        *sample = (x_asym * scalar_drive).tanh() - scalar_correction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_buffer_simd() {
        // Test aligned 4-block + remainder
        let mut dest = vec![0.5; 7];
        let src = vec![0.25; 7];
        mix_buffer_simd(&mut dest, &src);
        for (i, x) in dest.iter().enumerate() {
            assert!((x - 0.75).abs() < 1e-9, "Mismatch at index {}", i);
        }
    }

    #[test]
    fn test_apply_gain_simd() {
        // Test aligned 4-block + remainder
        let mut buf = vec![1.0; 9];
        apply_gain_simd(&mut buf, 0.5);
        for (i, x) in buf.iter().enumerate() {
            assert!((x - 0.5).abs() < 1e-9, "Mismatch at index {}", i);
        }
    }

    #[test]
    fn test_saturation_kernel() {
        // Just ensure it doesn't crash and outputs sensible range
        let mut buf = vec![0.5; 10];
        // Apply heavy warmth
        apply_saturation_kernel(&mut buf, 1.0);

        // Output should be somewhat compressed but valid
        for (i, x) in buf.iter().enumerate() {
            assert!(x.is_finite(), "NaN at index {}", i);
            assert!(x.abs() < 2.0, "Value exploded at index {}", i);
        }
    }
}
