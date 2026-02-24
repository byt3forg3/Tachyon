//! Bytes-per-Cycle (bpC) Benchmark: Tachyon
//!
//! Measures true algorithmic efficiency using hardware cycle counters (RDTSC).

#![allow(unsafe_code)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::unwrap_used)]

use rayon::prelude::*;
use std::arch::x86_64::_rdtsc;
use std::hint::black_box;

// =============================================================================
// UTILITIES
// =============================================================================

/// Measure RDTSC overhead to subtract from measurements.
fn measure_overhead(iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        black_box(0);
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

// =============================================================================
// MEASUREMENT FUNCTIONS
// =============================================================================

/// Sequential: pinned to 1 thread (Rayon ThreadPool with num_threads=1).
/// This isolates the hash from any parallel speedup.
fn measure_tachyon_seq(input: &[u8], iterations: u64) -> f64 {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    pool.install(|| {
        let start = unsafe { _rdtsc() };
        for _ in 0..iterations {
            black_box(tachyon::hash(black_box(input)));
        }
        let end = unsafe { _rdtsc() };
        (end - start) as f64 / iterations as f64
    })
}

fn measure_tachyon_zero_seq(input: &[u8], iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        black_box(tachyon_zero::hash(black_box(input)));
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

/// Parallel: uses the global Rayon thread pool (all cores).
/// tachyon::hash() internally spawns Rayon tasks above the parallel threshold.
fn measure_tachyon_par(input: &[u8], iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        black_box(tachyon::hash(black_box(input)));
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

fn measure_blake3_seq(input: &[u8], iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        black_box(blake3::hash(black_box(input)));
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

fn measure_blake3_par(input: &[u8], iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(black_box(input));
        black_box(hasher.finalize());
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

fn measure_sha256_seq(input: &[u8], iterations: u64) -> f64 {
    use sha2::Digest;
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        let mut hasher = sha2::Sha256::new();
        hasher.update(black_box(input));
        black_box(hasher.finalize());
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

fn measure_gxhash(input: &[u8], iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        black_box(gxhash::gxhash128(black_box(input), 0));
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

fn measure_xxh3(input: &[u8], iterations: u64) -> f64 {
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        black_box(xxhash_rust::xxh3::xxh3_128(black_box(input)));
    }
    let end = unsafe { _rdtsc() };
    (end - start) as f64 / iterations as f64
}

// =============================================================================
// MAIN BENCHMARK
// =============================================================================

fn main() {
    println!("BENCHMARK: Single-Core (SEQ) vs Multi-Core (PAR)");
    println!("=======================================================================================================================");
    println!(
        "{:<10} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
        "Size",
        "Tachyon(SEQ)",
        "Tachyon(PAR)",
        "Zero(SEQ)",
        "BLAKE3(SEQ)",
        "BLAKE3(PAR)",
        "SHA256(SEQ)",
        "GxHash(SEQ)",
        "XXH3(SEQ)"
    );
    println!(
        "{:-<10}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", "", "", "", "", ""
    );

    let overhead = measure_overhead(1_000_000);

    let sizes = [
        1024,
        64 * 1024,
        1024 * 1024,
        10 * 1024 * 1024,
        100 * 1024 * 1024,
    ];

    for &size in &sizes {
        let input = vec![0u8; size];
        let iterations = if size < 4096 {
            500_000
        } else if size < 1024 * 1024 {
            10_000
        } else if size < 10 * 1024 * 1024 {
            200
        } else {
            50
        };

        let t_seq = size as f64 / (measure_tachyon_seq(&input, iterations) - overhead).max(1.0);
        let t_par = size as f64 / (measure_tachyon_par(&input, iterations) - overhead).max(1.0);
        let z_seq =
            size as f64 / (measure_tachyon_zero_seq(&input, iterations) - overhead).max(1.0);
        let b_seq = size as f64 / (measure_blake3_seq(&input, iterations) - overhead).max(1.0);
        let b_par = size as f64 / (measure_blake3_par(&input, iterations) - overhead).max(1.0);
        let s_seq = size as f64 / (measure_sha256_seq(&input, iterations) - overhead).max(1.0);
        let g_seq = size as f64 / (measure_gxhash(&input, iterations) - overhead).max(1.0);
        let x_seq = size as f64 / (measure_xxh3(&input, iterations) - overhead).max(1.0);

        println!(
            "{:<10} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2} | {:<12.2}",
            format!("{} B", size),
            t_seq,
            t_par,
            z_seq,
            b_seq,
            b_par,
            s_seq,
            g_seq,
            x_seq
        );
    }
    println!("=======================================================================================================================");
    println!("* Values in Bytes/Cycle (Higher is Better)");
    println!("* SEQ = Single Thread | PAR = All Cores (Rayon)");
    println!("* SHA-256 cannot be parallelized.");

    measure_cpu_limit();
}

// =============================================================================
// THEORETICAL LIMIT (L1 CACHE)
// =============================================================================

fn measure_cpu_limit() {
    println!("\nCPU SCALING LIMIT (L1 Cache Test - 32KB per Thread)");
    println!("===========================================================");
    println!(
        "{:<15} | {:<12} | {:<12}",
        "Hash", "Peak bpC", "Est. GB/s @ 4GHz"
    );
    println!("{:-<15}-+-{:-<12}-+-{:-<12}", "", "", "");

    let num_threads = rayon::current_num_threads();
    let chunk_size = 32 * 1024;
    let iterations = 100_000;
    let total_bytes = num_threads as f64 * chunk_size as f64 * iterations as f64;

    // Helper
    let run_bench = |name: &str, func: fn(&[u8])| {
        let start = unsafe { _rdtsc() };
        (0..num_threads).into_par_iter().for_each(|_| {
            let buf = vec![0u8; chunk_size]; // Thread-local
            for _ in 0..iterations {
                func(black_box(&buf));
            }
        });
        let end = unsafe { _rdtsc() };
        let cycles = (end - start) as f64;
        let bpc = total_bytes / cycles;
        println!("{:<15} | {:<12.2} | {:<12.0}", name, bpc, bpc * 4.0);
    };

    run_bench("Tachyon", |d| {
        black_box(tachyon::hash(d));
    });
    run_bench("Tachyon Zero", |d| {
        black_box(tachyon_zero::hash(d));
    });
    run_bench("BLAKE3", |d| {
        black_box(blake3::hash(d));
    });
    run_bench("SHA-256", |d| {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(d);
        black_box(h.finalize());
    });
    run_bench("GxHash", |d| {
        black_box(gxhash::gxhash128(d, 0));
    });
    run_bench("XXH3", |d| {
        black_box(xxhash_rust::xxh3::xxh3_128(d));
    });

    println!("===========================================================");
    println!("* This test fits in CPU Cache. RAM Bandwidth is NOT a factor.");
    println!("* Shows theoretical maximum if memory were infinite speed.");
}
