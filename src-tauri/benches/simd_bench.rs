//! Performance benchmarks for SIMD summing engine.
//! Run with: cargo bench --bench simd_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// Mock the SIMD functions for benchmarking
mod simd_original {
    use wide::f64x4;

    pub fn mix_buffer_simd(dest: &mut [f64], src: &[f64]) {
        let mut dest_chunks = dest.chunks_exact_mut(4);
        let mut src_chunks = src.chunks_exact(4);

        while let (Some(d), Some(s)) = (dest_chunks.next(), src_chunks.next()) {
            let mut d_arr = [0.0; 4];
            d_arr.copy_from_slice(d);
            let mut s_arr = [0.0; 4];
            s_arr.copy_from_slice(s);

            let dv = f64x4::from(d_arr);
            let sv = f64x4::from(s_arr);
            let res = dv + sv;
            d.copy_from_slice(&res.to_array());
        }

        let dest_rem = dest_chunks.into_remainder();
        let src_rem = src_chunks.remainder();

        for (d, s) in dest_rem.iter_mut().zip(src_rem.iter()) {
            *d += *s;
        }
    }
}

mod simd_optimized {
    use wide::f64x4;

    pub fn mix_buffer_simd_optimized(dest: &mut [f64], src: &[f64]) {
        let len = dest.len().min(src.len());
        let simd_len = len / 4 * 4;

        for i in (0..simd_len).step_by(4) {
            unsafe {
                let d_ptr = dest.as_mut_ptr().add(i);
                let s_ptr = src.as_ptr().add(i);

                let dv = f64x4::new([*d_ptr, *d_ptr.add(1), *d_ptr.add(2), *d_ptr.add(3)]);
                let sv = f64x4::new([*s_ptr, *s_ptr.add(1), *s_ptr.add(2), *s_ptr.add(3)]);

                let res = dv + sv;
                let res_arr = res.to_array();

                *d_ptr = res_arr[0];
                *d_ptr.add(1) = res_arr[1];
                *d_ptr.add(2) = res_arr[2];
                *d_ptr.add(3) = res_arr[3];
            }
        }

        for i in simd_len..len {
            dest[i] += src[i];
        }
    }
}

fn bench_mix_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("mix_buffer");

    for size in [64, 256, 512, 1024, 4096].iter() {
        let mut dest = vec![1.0; *size];
        let src = vec![0.5; *size];

        group.bench_with_input(BenchmarkId::new("original", size), size, |b, _| {
            b.iter(|| {
                simd_original::mix_buffer_simd(black_box(&mut dest), black_box(&src));
            });
        });

        group.bench_with_input(BenchmarkId::new("optimized", size), size, |b, _| {
            b.iter(|| {
                simd_optimized::mix_buffer_simd_optimized(black_box(&mut dest), black_box(&src));
            });
        });
    }

    group.finish();
}

fn bench_tanh(c: &mut Criterion) {
    let mut group = c.benchmark_group("tanh");

    let test_values: Vec<f64> = (0..1000).map(|i| (i as f64 - 500.0) * 0.01).collect();

    group.bench_function("std_tanh", |b| {
        b.iter(|| {
            for &x in &test_values {
                black_box(x.tanh());
            }
        });
    });

    group.bench_function("fast_tanh", |b| {
        b.iter(|| {
            for &x in &test_values {
                black_box(fast_tanh(x));
            }
        });
    });

    group.finish();
}

#[inline(always)]
fn fast_tanh(x: f64) -> f64 {
    if x.abs() > 3.0 {
        return x.signum();
    }
    let x2 = x * x;
    let num = x * (135135.0 + x2 * (17325.0 + x2 * (378.0 + x2)));
    let den = 135135.0 + x2 * (62370.0 + x2 * (3150.0 + x2 * 28.0));
    num / den
}

criterion_group!(benches, bench_mix_buffer, bench_tanh);
criterion_main!(benches);
