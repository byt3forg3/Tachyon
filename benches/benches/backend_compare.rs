//! Backend Comparison Benchmark
//!
//! Compares performance of the hybrid runtime dispatcher vs explicit
//! AVX-512 and AES-NI kernels. Validates the cost of fallback paths.

#![allow(missing_docs)]
#![allow(unsafe_code)]
#![allow(clippy::unwrap_used)]
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use tachyon::kernels;

// =============================================================================
// BENCHMARKS
// =============================================================================

fn bench_backends(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tachyon Backends");

    // Scenarios:
    // - Small (7B): Test dispatch overhead vs short-path
    // - Medium (1KB): L1 cache hot-path
    // - Large (256KB): Bulk throughput (AVX-512 saturation)
    let sizes = [7, 1024, 256 * 1024];

    for size in sizes {
        let input = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        // 1. Hybrid (Production Path)
        // Measures cost of runtime dispatch + fastest available kernel
        group.bench_function(format!("Hybrid (Default) - {size} bytes"), |b| {
            b.iter(|| tachyon::hash(black_box(&input)));
        });

        // 2. AVX-512 - Explicit kernel (bypasses dispatcher)
        if is_x86_feature_detected!("avx512f") {
            group.bench_function(format!("AVX-512 Native - {size} bytes"), |b| {
                b.iter(|| unsafe { kernels::avx512::oneshot(black_box(&input), 0, 0, None) });
            });
        }

        // 3. AES-NI - Explicit fallback kernel
        // Forces the AES-NI path to measure performance on non-AVX-512 hardware
        if is_x86_feature_detected!("aes") {
            group.bench_function(format!("AES-NI Native - {size} bytes"), |b| {
                b.iter(|| unsafe { kernels::aesni::oneshot(black_box(&input), 0, 0, None) });
            });
        }

        // 4. Portable - Pure Rust, no SIMD
        // Baseline to quantify the speedup from hardware acceleration
        group.bench_function(format!("Portable (No SIMD) - {size} bytes"), |b| {
            b.iter(|| kernels::portable::oneshot(black_box(&input), 0, 0, None));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_backends);
criterion_main!(benches);
