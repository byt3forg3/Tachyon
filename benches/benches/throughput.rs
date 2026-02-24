//! Tachyon Comprehensive Criterion Benchmark
//!
//! Statistically rigorous performance measurements across all scenarios.

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rand::prelude::*;
use std::hint::black_box;

const KB: usize = 1024;
const MB: usize = 1024 * 1024;

// =============================================================================
// BENCHMARK 1: LATENCY
// =============================================================================

/// Hot path latency for small inputs (Hash Map keys, IDs).
fn bench_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("1-Latency");

    let sizes = [
        (16, "16B"),
        (64, "64B"),
        (256, "256B"),
        (KB, "1KB"),
        (4 * KB, "4KB"),
    ];

    for (size, name) in sizes {
        let mut input = vec![0u8; size];
        rand::rng().fill(&mut input[..]);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(name),
            &input,
            |b, data| b.iter(|| tachyon::hash(black_box(data))),
        );
    }
    group.finish();
}

// =============================================================================
// BENCHMARK 2: SMALL FILES
// =============================================================================

/// Throughput for small files (Git objects, database chunks).
fn bench_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("2-Small-Files");

    let sizes = [
        (8 * KB, "8KB"),
        (16 * KB, "16KB"),
        (32 * KB, "32KB"),
        (64 * KB, "64KB"),
        (128 * KB, "128KB"),
        (256 * KB, "256KB"),
    ];

    for (size, name) in sizes {
        let mut input = vec![0u8; size];
        rand::rng().fill(&mut input[..]);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(name),
            &input,
            |b, data| b.iter(|| tachyon::hash(black_box(data))),
        );
    }
    group.finish();
}

// =============================================================================
// BENCHMARK 3: MEDIUM FILES
// =============================================================================

/// Throughput for medium files (Documents, images, source bundles).
fn bench_medium_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("3-Medium-Files");
    group.sample_size(50); // Reduced samples for larger inputs

    let sizes = [
        (512 * KB, "512KB"),
        (MB, "1MB"),
        (4 * MB, "4MB"),
        (8 * MB, "8MB"),
        (16 * MB, "16MB"),
    ];

    for (size, name) in sizes {
        let mut input = vec![0u8; size];
        rand::rng().fill(&mut input[..]);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(name),
            &input,
            |b, data| b.iter(|| tachyon::hash(black_box(data))),
        );
    }
    group.finish();
}

// =============================================================================
// BENCHMARK 4: LARGE FILES
// =============================================================================

/// Throughput for large files (ISOs, Videos). Includes 1GB RAM saturation test.
fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("4-Large-Files");
    group.sample_size(20); // Minimal samples for heavy I/O simulation

    let sizes = [
        (32 * MB, "32MB"),
        (64 * MB, "64MB"),
        (100 * MB, "100MB"),
        (256 * MB, "256MB"),
        (1024 * MB, "1GB-RAM-Saturation"),
    ];

    for (size, name) in sizes {
        let mut input = vec![0u8; size];
        rand::rng().fill(&mut input[..]);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(name),
            &input,
            |b, data| b.iter(|| tachyon::hash(black_box(data))),
        );
    }
    group.finish();
}

// =============================================================================
// BENCHMARK 5: STREAMING
// =============================================================================

/// Throughput for incremental updates (Network streams, large file hashing).
#[cfg(feature = "multithread")]
fn bench_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("5-Streaming");
    group.sample_size(50);

    let test_cases = [
        (MB, 4 * KB, "1MB-4KB-chunks"),
        (MB, 64 * KB, "1MB-64KB-chunks"),
        (16 * MB, 64 * KB, "16MB-64KB-chunks"),
        (16 * MB, 256 * KB, "16MB-256KB-chunks"),
        (100 * MB, MB, "100MB-1MB-chunks"),
    ];

    for (total_size, chunk_size, name) in test_cases {
        let mut input = vec![0u8; total_size];
        rand::rng().fill(&mut input[..]);
        group.throughput(Throughput::Bytes(total_size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(name),
            &(input, chunk_size),
            |b, (data, chunk_sz)| {
                b.iter(|| {
                    let mut hasher = tachyon::Hasher::new().unwrap();
                    for chunk in data.chunks(*chunk_sz) {
                        hasher.update(black_box(chunk));
                    }
                    hasher.finalize()
                })
            },
        );
    }
    group.finish();
}

// =============================================================================
// BENCHMARK 6: THREAD SCALING
// =============================================================================

/// Multi-core scaling efficiency using Rayon (1 to N threads).
#[cfg(feature = "multithread")]
fn bench_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("6-Thread-Scaling");
    group.sample_size(50);

    let size = 16 * MB;
    let mut input = vec![0u8; size];
    rand::rng().fill(&mut input[..]);
    group.throughput(Throughput::Bytes(size as u64));

    let max_threads = num_cpus::get();
    let thread_counts: Vec<usize> = [1, 2, 4, 8, 16, 32]
        .iter()
        .copied()
        .filter(|&t| t <= max_threads)
        .collect();

    for threads in thread_counts {
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(format!("{}threads", threads)),
            &threads,
            |b, &t| {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(t)
                    .build()
                    .unwrap();
                pool.install(|| b.iter(|| tachyon::hash(black_box(&input))));
            },
        );
    }
    group.finish();
}

// =============================================================================
// BENCHMARK 7: SPECIAL OPERATIONS
// =============================================================================

/// Latency/Throughput for secondary features (Keyed Hash, Domain Separation).
fn bench_special_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("7-Special-Operations");

    let size = 64 * KB;
    let mut input = vec![0u8; size];
    rand::rng().fill(&mut input[..]);
    let key = [42u8; 32];
    group.throughput(Throughput::Bytes(size as u64));

    // Regular hash
    group.bench_function("regular-hash", |b| {
        b.iter(|| tachyon::hash(black_box(&input)))
    });

    // Keyed hash (MAC)
    group.bench_function("keyed-hash", |b| {
        b.iter(|| tachyon::hash_keyed(black_box(&input), black_box(&key)))
    });

    // Domain-separated hash
    group.bench_function("domain-separated", |b| {
        b.iter(|| {
            tachyon::hash_with_domain(black_box(&input), tachyon::TachyonDomain::FileChecksum)
        })
    });

    // Verification (constant-time)
    let hash = tachyon::hash(&input);
    group.bench_function("verify", |b| {
        b.iter(|| tachyon::verify(black_box(&input), black_box(&hash)))
    });

    // MAC verification
    let mac = tachyon::hash_keyed(&input, &key);
    group.bench_function("verify-mac", |b| {
        b.iter(|| tachyon::verify_mac(black_box(&input), black_box(&key), black_box(&mac)))
    });

    group.finish();
}

// =============================================================================
// BENCHMARK 8: CACHE EFFECTS
// =============================================================================

/// Performance at various cache hierarchy levels (L1/L2/L3/RAM).
fn bench_cache_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("8-Cache-Effects");

    let sizes = [
        (8 * KB, "8KB-L1"),     // Fits in L1 cache
        (64 * KB, "64KB-L2"),   // Fits in L2 cache
        (512 * KB, "512KB-L3"), // Fits in L3 cache
        (8 * MB, "8MB-RAM"),    // RAM access
        (64 * MB, "64MB-RAM"),  // Heavy RAM access
    ];

    for (size, name) in sizes {
        let mut input = vec![0u8; size];
        rand::rng().fill(&mut input[..]);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(name),
            &input,
            |b, data| b.iter(|| tachyon::hash(black_box(data))),
        );
    }
    group.finish();
}

// =============================================================================
// MAIN
// =============================================================================

criterion_group!(
    benches,
    bench_latency,
    bench_small_files,
    bench_medium_files,
    bench_large_files,
    bench_special_operations,
    bench_cache_effects,
);

#[cfg(feature = "multithread")]
criterion_group!(benches_multithread, bench_streaming, bench_thread_scaling,);

#[cfg(feature = "multithread")]
criterion_main!(benches, benches_multithread);

#[cfg(not(feature = "multithread"))]
criterion_main!(benches);
