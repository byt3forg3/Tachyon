//! Bytes-per-Cycle (bpC) Benchmark: Tachyon
//!
//! Measures true algorithmic efficiency using hardware cycle counters (RDTSC).

#![allow(unsafe_code)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::unwrap_used)]

use rayon::prelude::*;
use std::arch::x86_64::_rdtsc;
use std::env;
use std::hint::black_box;

const SAMPLE_COUNT_TINY: usize = 15;
const SAMPLE_COUNT_SMALL: usize = 11;
const SAMPLE_COUNT_LARGE: usize = 7;

#[derive(Clone, Copy)]
struct CycleStats {
    mean: f64,
    p95: f64,
    p99: f64,
}

#[derive(Clone, Copy)]
struct RowMetrics {
    bpc: f64,
    cph: f64,
    p95_cph: f64,
    p99_cph: f64,
}

struct BenchRow {
    size: usize,
    metrics: [RowMetrics; 7],
}

// =============================================================================
// UTILITIES
// =============================================================================

fn robust_stats(samples: &mut [f64]) -> CycleStats {
    samples.sort_by(f64::total_cmp);
    let p95 = percentile_from_sorted(samples, 0.95);
    let p99 = percentile_from_sorted(samples, 0.99);
    let trim = if samples.len() >= 5 {
        samples.len() / 5
    } else {
        0
    };
    let window = &samples[trim..samples.len() - trim];
    let mean = window.iter().sum::<f64>() / window.len() as f64;

    CycleStats { mean, p95, p99 }
}

fn percentile_from_sorted(sorted_samples: &[f64], percentile: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }

    let rank = (percentile.clamp(0.0, 1.0) * (sorted_samples.len() as f64 - 1.0)).ceil() as usize;
    sorted_samples[rank.min(sorted_samples.len() - 1)]
}

fn measure_cycles_per_iter<F>(mut func: F, iterations: u64, samples: usize) -> CycleStats
where
    F: FnMut(),
{
    let warmup_iterations = (iterations / 50).max(100);
    let mut cycles_per_iter = Vec::with_capacity(samples);

    for _ in 0..samples {
        for _ in 0..warmup_iterations {
            func();
        }

        let start = unsafe { _rdtsc() };
        for _ in 0..iterations {
            func();
        }
        let end = unsafe { _rdtsc() };

        cycles_per_iter.push((end - start) as f64 / iterations as f64);
    }

    robust_stats(&mut cycles_per_iter)
}

fn measure_loop_overhead(iterations: u64, samples: usize) -> f64 {
    measure_cycles_per_iter(
        || {
            black_box(());
        },
        iterations,
        samples,
    )
    .mean
}

fn sample_count_for_size(size: usize) -> usize {
    if size <= 128 {
        SAMPLE_COUNT_TINY
    } else if size <= 64 * 1024 {
        SAMPLE_COUNT_SMALL
    } else {
        SAMPLE_COUNT_LARGE
    }
}

// =============================================================================
// MEASUREMENT FUNCTIONS
// =============================================================================

/// Sequential: pinned to 1 thread (Rayon ThreadPool with num_threads=1).
/// This isolates the hash from any parallel speedup.
fn measure_tachyon_seq(
    pool: &rayon::ThreadPool,
    input: &[u8],
    iterations: u64,
    samples: usize,
) -> CycleStats {
    pool.install(|| {
        measure_cycles_per_iter(
            || {
                black_box(tachyon::hash(black_box(input)));
            },
            iterations,
            samples,
        )
    })
}

/// Parallel: uses the global Rayon thread pool (all cores).
/// tachyon::hash() internally spawns Rayon tasks above the parallel threshold.
fn measure_tachyon_par(input: &[u8], iterations: u64, samples: usize) -> CycleStats {
    measure_cycles_per_iter(
        || {
            black_box(tachyon::hash(black_box(input)));
        },
        iterations,
        samples,
    )
}

fn measure_blake3_seq(input: &[u8], iterations: u64, samples: usize) -> CycleStats {
    measure_cycles_per_iter(
        || {
            black_box(blake3::hash(black_box(input)));
        },
        iterations,
        samples,
    )
}

fn measure_blake3_par(input: &[u8], iterations: u64, samples: usize) -> CycleStats {
    measure_cycles_per_iter(
        || {
            let mut hasher = blake3::Hasher::new();
            hasher.update_rayon(black_box(input));
            black_box(hasher.finalize());
        },
        iterations,
        samples,
    )
}

fn measure_sha256_seq(input: &[u8], iterations: u64, samples: usize) -> CycleStats {
    use sha2::Digest;
    measure_cycles_per_iter(
        || {
            let mut hasher = sha2::Sha256::new();
            hasher.update(black_box(input));
            black_box(hasher.finalize());
        },
        iterations,
        samples,
    )
}

fn measure_gxhash(input: &[u8], iterations: u64, samples: usize) -> CycleStats {
    measure_cycles_per_iter(
        || {
            black_box(gxhash::gxhash128(black_box(input), 0));
        },
        iterations,
        samples,
    )
}

fn measure_xxh3(input: &[u8], iterations: u64, samples: usize) -> CycleStats {
    measure_cycles_per_iter(
        || {
            black_box(xxhash_rust::xxh3::xxh3_128(black_box(input)));
        },
        iterations,
        samples,
    )
}

// =============================================================================
// MAIN BENCHMARK
// =============================================================================

fn main() {
    let short_mode = env::args().any(|arg| arg == "--short");

    let seq_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();

    println!("BPC TABLE: Bytes per Cycle (Higher is Better)");
    println!(
        "Mode: {}",
        if short_mode {
            "short (8B..512B)"
        } else {
            "full"
        }
    );
    println!("==========================================================================================================");
    println!(
        "{:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
        "Size",
        "Tachyon(SEQ)",
        "Tachyon(PAR)",
        "BLAKE3(SEQ)",
        "BLAKE3(PAR)",
        "SHA256(SEQ)",
        "GxHash(SEQ)",
        "XXH3(SEQ)"
    );
    println!(
        "{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", "", "", "", ""
    );

    let short_sizes = [8, 16, 30, 32, 48, 64, 80, 96, 128, 256, 512];
    let full_sizes = [
        8,
        16,
        30,
        32,
        48,
        64,
        80,
        96,
        128,
        256,
        512,
        1024,
        64 * 1024,
        1024 * 1024,
        10 * 1024 * 1024,
        100 * 1024 * 1024,
    ];
    let sizes = if short_mode {
        &short_sizes[..]
    } else {
        &full_sizes[..]
    };

    let mut rows = Vec::with_capacity(sizes.len());
    for &size in sizes {
        let input = vec![0u8; size];
        let samples = sample_count_for_size(size);
        let iterations = if size < 128 {
            2_000_000
        } else if size < 4096 {
            500_000
        } else if size < 1024 * 1024 {
            10_000
        } else if size < 10 * 1024 * 1024 {
            200
        } else {
            50
        };

        let overhead = measure_loop_overhead(iterations, samples);

        let raw_stats = [
            measure_tachyon_seq(&seq_pool, &input, iterations, samples),
            measure_tachyon_par(&input, iterations, samples),
            measure_blake3_seq(&input, iterations, samples),
            measure_blake3_par(&input, iterations, samples),
            measure_sha256_seq(&input, iterations, samples),
            measure_gxhash(&input, iterations, samples),
            measure_xxh3(&input, iterations, samples),
        ];

        let metrics = raw_stats.map(|stats| {
            let adjusted_cph = (stats.mean - overhead).max(1.0);
            let bpc = size as f64 / adjusted_cph;
            let p95_cph = (stats.p95 - overhead).max(1.0);
            let p99_cph = (stats.p99 - overhead).max(1.0);

            RowMetrics {
                bpc,
                cph: adjusted_cph,
                p95_cph,
                p99_cph,
            }
        });

        let bpcs = metrics.map(|metric| metric.bpc);

        println!(
            "{:<12} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2}",
            format!("{} B", size),
            bpcs[0],
            bpcs[1],
            bpcs[2],
            bpcs[3],
            bpcs[4],
            bpcs[5],
            bpcs[6]
        );

        rows.push(BenchRow { size, metrics });
    }

    let l1 = measure_cpu_limit();
    println!(
        "{:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
        "L1 cache",
        "-",
        format!("{:.2}", l1.tachyon),
        "-",
        format!("{:.2}", l1.blake3),
        format!("{:.2}", l1.sha256),
        format!("{:.2}", l1.gxhash),
        format!("{:.2}", l1.xxh3)
    );

    println!();
    println!("LATENCY TABLE: Cycles per Hash (Lower is Better)");
    println!("==========================================================================================================");
    println!(
        "{:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
        "Size",
        "Tachyon(SEQ)",
        "Tachyon(PAR)",
        "BLAKE3(SEQ)",
        "BLAKE3(PAR)",
        "SHA256(SEQ)",
        "GxHash(SEQ)",
        "XXH3(SEQ)"
    );
    println!(
        "{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", "", "", "", ""
    );
    for row in &rows {
        let cph = row.metrics.map(|metric| metric.cph);
        println!(
            "{:<12} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2}",
            format!("{} B", row.size),
            cph[0],
            cph[1],
            cph[2],
            cph[3],
            cph[4],
            cph[5],
            cph[6]
        );
    }

    if !short_mode {
        println!();
        println!("TAIL LATENCY TABLE (Run-to-Run): p95/p99 Cycles per Hash (Lower is Better)");
        println!("==========================================================================================================");
        println!(
            "{:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
            "Size",
            "Tachyon(SEQ)",
            "Tachyon(PAR)",
            "BLAKE3(SEQ)",
            "BLAKE3(PAR)",
            "SHA256(SEQ)",
            "GxHash(SEQ)",
            "XXH3(SEQ)"
        );
        println!(
            "{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}",
            "", "", "", "", "", "", "", ""
        );
        for row in rows.iter().filter(|row| row.size <= 256) {
            let p95 = row.metrics.map(|metric| metric.p95_cph);
            let p99 = row.metrics.map(|metric| metric.p99_cph);
            let tail_cells = [
                format!("{:.1}/{:.1}", p95[0], p99[0]),
                format!("{:.1}/{:.1}", p95[1], p99[1]),
                format!("{:.1}/{:.1}", p95[2], p99[2]),
                format!("{:.1}/{:.1}", p95[3], p99[3]),
                format!("{:.1}/{:.1}", p95[4], p99[4]),
                format!("{:.1}/{:.1}", p95[5], p99[5]),
                format!("{:.1}/{:.1}", p95[6], p99[6]),
            ];

            println!(
                "{:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
                format!("{} B", row.size),
                tail_cells[0],
                tail_cells[1],
                tail_cells[2],
                tail_cells[3],
                tail_cells[4],
                tail_cells[5],
                tail_cells[6]
            );
        }
        println!("* Cell format: p95/p99 cycles per hash (run-to-run across samples, not individual requests).",);
    }
}

// =============================================================================
// CPU LIMIT RESULT (L1 CACHE)
// =============================================================================

struct CpuLimitResult {
    tachyon: f64,
    blake3: f64,
    sha256: f64,
    gxhash: f64,
    xxh3: f64,
}

fn measure_cpu_limit() -> CpuLimitResult {
    let num_threads = rayon::current_num_threads();
    let chunk_size = 32 * 1024;
    let iterations = 100_000;
    let total_bytes = num_threads as f64 * chunk_size as f64 * iterations as f64;

    // Helper
    let run_bench = |func: fn(&[u8])| {
        let mut samples = [0.0_f64; SAMPLE_COUNT_LARGE];

        for sample in &mut samples {
            let start = unsafe { _rdtsc() };
            (0..num_threads).into_par_iter().for_each(|_| {
                let buf = vec![0u8; chunk_size];
                for _ in 0..iterations {
                    func(black_box(&buf));
                }
            });
            let end = unsafe { _rdtsc() };
            let cycles = (end - start) as f64;
            *sample = total_bytes / cycles;
        }

        robust_stats(&mut samples).mean
    };

    let tachyon = run_bench(|d| {
        black_box(tachyon::hash(d));
    });
    let blake3 = run_bench(|d| {
        black_box(blake3::hash(d));
    });
    let sha256 = run_bench(|d| {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(d);
        black_box(h.finalize());
    });
    let gxhash = run_bench(|d| {
        black_box(gxhash::gxhash128(d, 0));
    });
    let xxh3 = run_bench(|d| {
        black_box(xxhash_rust::xxh3::xxh3_128(d));
    });

    CpuLimitResult {
        tachyon,
        blake3,
        sha256,
        gxhash,
        xxh3,
    }
}
