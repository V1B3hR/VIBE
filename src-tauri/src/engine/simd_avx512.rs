#![allow(dead_code)]

use std::arch::x86_64::*;
use wide::f64x4;

/// SIMD-optimized audio summing using AVX-512
/// Processes 8 f64 samples per cycle
pub struct SimdSummer {
    supports_avx512: bool,
}

impl SimdSummer {
    pub fn new() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                supports_avx512: is_x86_feature_detected!("avx512f"),
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {
                supports_avx512: false,
            }
        }
    }

    /// Sum two audio buffers with SIMD acceleration
    /// dest += src
    pub fn sum_buffers(&self, dest: &mut [f64], src: &[f64]) {
        let len = dest.len().min(src.len());

        #[cfg(target_arch = "x86_64")]
        if self.supports_avx512 && len >= 8 {
            unsafe {
                self.sum_buffers_avx512(dest, src, len);
                return;
            }
        }

        // Fallback to wide crate (AVX2/NEON/SSE depending on target)
        let simd_len = (len / 4) * 4;
        let mut i = 0;
        while i < simd_len {
            unsafe {
                let dest_ptr = dest.as_mut_ptr().add(i);
                let src_ptr = src.as_ptr().add(i);
                
                let d_vec = f64x4::new([*dest_ptr, *dest_ptr.add(1), *dest_ptr.add(2), *dest_ptr.add(3)]);
                let s_vec = f64x4::new([*src_ptr, *src_ptr.add(1), *src_ptr.add(2), *src_ptr.add(3)]);
                
                let res = (d_vec + s_vec).to_array();
                *dest_ptr = res[0];
                *dest_ptr.add(1) = res[1];
                *dest_ptr.add(2) = res[2];
                *dest_ptr.add(3) = res[3];
            }
            i += 4;
        }

        for j in simd_len..len {
            dest[j] += src[j];
        }
    }

    /// Sum two audio buffers with gain: dest += src * gain
    pub fn sum_buffers_with_gain(&self, dest: &mut [f64], src: &[f64], gain: f64) {
        let len = dest.len().min(src.len());

        #[cfg(target_arch = "x86_64")]
        if self.supports_avx512 && len >= 8 {
            unsafe {
                self.sum_buffers_with_gain_avx512(dest, src, gain, len);
                return;
            }
        }

        // Fallback to wide crate
        let simd_len = (len / 4) * 4;
        let g_vec = f64x4::splat(gain);
        let mut i = 0;
        while i < simd_len {
            unsafe {
                let dest_ptr = dest.as_mut_ptr().add(i);
                let src_ptr = src.as_ptr().add(i);
                
                let d_vec = f64x4::new([*dest_ptr, *dest_ptr.add(1), *dest_ptr.add(2), *dest_ptr.add(3)]);
                let s_vec = f64x4::new([*src_ptr, *src_ptr.add(1), *src_ptr.add(2), *src_ptr.add(3)]);
                
                let res = (d_vec + (s_vec * g_vec)).to_array();
                *dest_ptr = res[0];
                *dest_ptr.add(1) = res[1];
                *dest_ptr.add(2) = res[2];
                *dest_ptr.add(3) = res[3];
            }
            i += 4;
        }

        for j in simd_len..len {
            dest[j] += src[j] * gain;
        }
    }

    /// AVX-512 implementation: 8 f64 per iteration
    #[target_feature(enable = "avx512f")]
    unsafe fn sum_buffers_avx512(&self, dest: &mut [f64], src: &[f64], len: usize) {
        let mut i = 0;
        let simd_end = (len / 8) * 8;

        // Process 8 samples at a time
        while i < simd_end {
            // Load 8 f64 from dest
            let dest_vec = _mm512_loadu_pd(dest.as_ptr().add(i));

            // Load 8 f64 from src
            let src_vec = _mm512_loadu_pd(src.as_ptr().add(i));

            // Add
            let result = _mm512_add_pd(dest_vec, src_vec);

            // Store back to dest
            _mm512_storeu_pd(dest.as_mut_ptr().add(i), result);

            i += 8;
        }

        // Handle remaining samples (scalar)
        for j in i..len {
            dest[j] += src[j];
        }
    }

    #[target_feature(enable = "avx512f")]
    unsafe fn sum_buffers_with_gain_avx512(&self, dest: &mut [f64], src: &[f64], gain: f64, len: usize) {
        let mut i = 0;
        let simd_end = (len / 8) * 8;
        let gain_vec = _mm512_set1_pd(gain);

        while i < simd_end {
            let dest_vec = _mm512_loadu_pd(dest.as_ptr().add(i));
            let src_vec = _mm512_loadu_pd(src.as_ptr().add(i));
            
            // FMA: dest = dest + (src * gain)
            let result = _mm512_fmadd_pd(src_vec, gain_vec, dest_vec);
            
            _mm512_storeu_pd(dest.as_mut_ptr().add(i), result);
            i += 8;
        }

        for j in i..len {
            dest[j] += src[j] * gain;
        }
    }

    /// Multiply buffer by scalar with SIMD
    pub fn multiply_scalar(&self, buffer: &mut [f64], scalar: f64) {
        let len = buffer.len();

        #[cfg(target_arch = "x86_64")]
        if self.supports_avx512 && len >= 8 {
            unsafe {
                self.multiply_scalar_avx512(buffer, scalar, len);
                return;
            }
        }

        // Fallback to wide crate
        let simd_len = (len / 4) * 4;
        let s_vec = f64x4::splat(scalar);
        let mut i = 0;
        while i < simd_len {
            unsafe {
                let ptr = buffer.as_mut_ptr().add(i);
                let b_vec = f64x4::new([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]);
                
                let res = (b_vec * s_vec).to_array();
                *ptr = res[0];
                *ptr.add(1) = res[1];
                *ptr.add(2) = res[2];
                *ptr.add(3) = res[3];
            }
            i += 4;
        }

        for j in simd_len..len {
            buffer[j] *= scalar;
        }
    }

    #[target_feature(enable = "avx512f")]
    unsafe fn multiply_scalar_avx512(&self, buffer: &mut [f64], scalar: f64, len: usize) {
        let mut i = 0;
        let simd_end = (len / 8) * 8;

        // Broadcast scalar to all 8 lanes
        let scalar_vec = _mm512_set1_pd(scalar);

        while i < simd_end {
            let buf_vec = _mm512_loadu_pd(buffer.as_ptr().add(i));
            let result = _mm512_mul_pd(buf_vec, scalar_vec);
            _mm512_storeu_pd(buffer.as_mut_ptr().add(i), result);
            i += 8;
        }

        for j in i..len {
            buffer[j] *= scalar;
        }
    }

    /// Clear buffer with SIMD
    pub fn clear(&self, buffer: &mut [f64]) {
        let len = buffer.len();

        #[cfg(target_arch = "x86_64")]
        if self.supports_avx512 && len >= 8 {
            unsafe {
                self.clear_avx512(buffer, len);
                return;
            }
        }

        // Fallback to wide crate
        let simd_len = (len / 4) * 4;
        let zero_vec = f64x4::splat(0.0);
        let mut i = 0;
        let res = zero_vec.to_array();
        while i < simd_len {
            unsafe {
                let ptr = buffer.as_mut_ptr().add(i);
                *ptr = res[0];
                *ptr.add(1) = res[1];
                *ptr.add(2) = res[2];
                *ptr.add(3) = res[3];
            }
            i += 4;
        }

        for j in simd_len..len {
            buffer[j] = 0.0;
        }
    }

    #[target_feature(enable = "avx512f")]
    unsafe fn clear_avx512(&self, buffer: &mut [f64], len: usize) {
        let mut i = 0;
        let simd_end = (len / 8) * 8;
        let zero = _mm512_setzero_pd();

        while i < simd_end {
            _mm512_storeu_pd(buffer.as_mut_ptr().add(i), zero);
            i += 8;
        }

        for j in i..len {
            buffer[j] = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_buffers() {
        let summer = SimdSummer::new();
        let mut dest = vec![1.0; 16];
        let src = vec![2.0; 16];

        summer.sum_buffers(&mut dest, &src);

        for &val in &dest {
            assert_eq!(val, 3.0);
        }
    }

    #[test]
    fn test_multiply_scalar() {
        let summer = SimdSummer::new();
        let mut buffer = vec![2.0; 16];

        summer.multiply_scalar(&mut buffer, 3.0);

        for &val in &buffer {
            assert_eq!(val, 6.0);
        }
    }
}
