//! Backend Comparison Benchmark
//!
//! Compares Tachyon Core hardware backends across path boundaries.

#![allow(missing_docs)]
#![allow(unsafe_code)]
#![allow(clippy::unwrap_used)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use tachyon::kernels as tachyon_kernels;

// Path boundaries:
//   ≤  32 B  →  short   (AES-NI short path)
//      128 B  →  medium  (AVX2 / AES-NI medium path)
//   ≥  256 B  →  bulk   (8-accumulator streaming Hasher)
const SIZES: &[(usize, &str)] = &[
    (7, "7B"),
    (32, "32B"),
    (128, "128B"),
    (256, "256B"),
    (1024, "1KB"),
    (256 * 1024, "256KB"),
];

// =============================================================================
// TACHYON
// =============================================================================

fn bench_backends_tachyon(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tachyon Backends");

    for &(size, label) in SIZES {
        let input = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("Auto", label), &input, |b, data| {
            b.iter(|| tachyon::hash(black_box(data)));
        });

        if is_x86_feature_detected!("avx512f") {
            group.bench_with_input(BenchmarkId::new("AVX-512", label), &input, |b, data| {
                b.iter(|| unsafe { tachyon_kernels::avx512::oneshot(black_box(data), 0, 0, None) });
            });
        }

        if is_x86_feature_detected!("aes") {
            group.bench_with_input(BenchmarkId::new("AES-NI", label), &input, |b, data| {
                b.iter(|| unsafe { tachyon_kernels::aesni::oneshot(black_box(data), 0, 0, None) });
            });
        }

        group.bench_with_input(BenchmarkId::new("Portable", label), &input, |b, data| {
            b.iter(|| tachyon_kernels::portable::oneshot(black_box(data), 0, 0, None));
        });
    }

    group.finish();
}

// =============================================================================
// MAIN
// =============================================================================

criterion_group!(benches, bench_backends_tachyon);
criterion_main!(benches);
