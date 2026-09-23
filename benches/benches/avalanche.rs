//! Avalanche Test Benchmark: Tachyon Core
//!
//! Measures Strict Avalanche Criterion (SAC) and Bit Independence Criterion (BIC)
//! using a Chi-Squared test.
//!
//! Every input bit flip should ideally flip every output bit with a probability of exactly 50%.

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rand::prelude::*;
use std::io::{self, Write};

const HASH_SIZE_BYTES: usize = 32;
const HASH_SIZE_BITS: usize = HASH_SIZE_BYTES * 8;
const INPUT_SIZE_BYTES: usize = 64;
const INPUT_SIZE_BITS: usize = INPUT_SIZE_BYTES * 8;
const NUM_SAMPLES: usize = 10_000;

// =============================================================================
// TYPES & ENUMS
// =============================================================================

#[derive(Clone, Copy)]
enum Algorithm {
    Tachyon,
}

impl Algorithm {
    fn name(&self) -> &'static str {
        match self {
            Self::Tachyon => "Tachyon",
        }
    }

    fn hash(&self, input: &[u8]) -> [u8; HASH_SIZE_BYTES] {
        match self {
            Self::Tachyon => tachyon::hash(input),
        }
    }
}

// =============================================================================
// STATISTICS STATE
// =============================================================================

/// Statistics for the Avalanche Test.
struct AvalancheStats {
    /// counts[i][j] = number of times flipping input bit `i` causes output bit `j` to flip.
    counts: Vec<Vec<u64>>,
    /// bic_counts[i][flat_idx] = number of times flipping input bit `i` causes BOTH output bits `j` and `k` to flip.
    bic_counts: Vec<Vec<u32>>,
    samples: u64,
}

impl AvalancheStats {
    fn new() -> Self {
        Self {
            counts: vec![vec![0; HASH_SIZE_BITS]; INPUT_SIZE_BITS],
            // 256 * 255 / 2 = 32640 pairs of output bits
            bic_counts: vec![vec![0; 32640]; INPUT_SIZE_BITS],
            samples: 0,
        }
    }

    fn add_sample(
        &mut self,
        _base_input: &[u8],
        base_hash: &[u8],
        mod_hash: &[u8],
        flipped_input_bit: usize,
    ) {
        let mut diff_bits = Vec::with_capacity(128);
        for j in 0..HASH_SIZE_BITS {
            let byte_idx = j / 8;
            let bit_idx = j % 8;
            let base_bit = (base_hash[byte_idx] >> bit_idx) & 1;
            let mod_bit = (mod_hash[byte_idx] >> bit_idx) & 1;

            if base_bit != mod_bit {
                self.counts[flipped_input_bit][j] += 1;
                diff_bits.push(j);
            }
        }

        for (idx, &j) in diff_bits.iter().enumerate() {
            for &k in diff_bits.iter().skip(idx + 1) {
                let pairs_before_j = j * 255 - j * j.saturating_sub(1) / 2;
                let flat_idx = pairs_before_j + (k - j - 1);
                self.bic_counts[flipped_input_bit][flat_idx] += 1;
            }
        }
    }

    fn increment_samples(&mut self) {
        self.samples += 1;
    }

    /// Calculates the SAC (Strict Avalanche Criterion) score.
    /// Ideally, this should be 0.5 (50%).
    fn sac_score(&self) -> f64 {
        let mut total_flips = 0;
        for i in 0..INPUT_SIZE_BITS {
            for j in 0..HASH_SIZE_BITS {
                total_flips += self.counts[i][j];
            }
        }
        let total_possible =
            (self.samples as f64) * (INPUT_SIZE_BITS as f64) * (HASH_SIZE_BITS as f64);
        (total_flips as f64) / total_possible
    }

    /// Calculates Maximum Bias
    fn max_bias(&self) -> f64 {
        let mut max_dev = 0.0;
        let expected = (self.samples as f64) * 0.5;
        for i in 0..INPUT_SIZE_BITS {
            for j in 0..HASH_SIZE_BITS {
                let p = self.counts[i][j] as f64;
                let dev = (p - expected).abs() / (self.samples as f64);
                if dev > max_dev {
                    max_dev = dev;
                }
            }
        }
        max_dev
    }

    /// Perform a basic Chi-Squared test for uniformity (Expected value = samples / 2).
    /// Degrees of freedom = 1 (two categories: flipped or not flipped).
    fn chi_squared_test(&self) -> f64 {
        let expected = (self.samples as f64) * 0.5;
        let mut chi_sq = 0.0;

        let mut n_tests = 0;

        for i in 0..INPUT_SIZE_BITS {
            for j in 0..HASH_SIZE_BITS {
                let observed_flips = self.counts[i][j] as f64;
                let observed_non_flips = (self.samples as f64) - observed_flips;

                let df = (observed_flips - expected).powi(2) / expected;
                let dnf = (observed_non_flips - expected).powi(2) / expected;

                chi_sq += df + dnf;
                n_tests += 1;
            }
        }

        // Average chi-squared statistic across all (input bit, output bit) pairs.
        // For df=1, a critical value of ~3.84 corresponds to p=0.05.
        // A value close to 1.0 is expected for perfectly uniform noise.
        chi_sq / (n_tests as f64)
    }

    /// Calculates the Bit Independence Criterion (BIC) score parameters
    /// Returns (mean_absolute_correlation, max_absolute_correlation)
    fn bic_score(&self) -> (f64, f64) {
        let mut total_corr = 0.0;
        let mut max_corr = 0.0;
        let mut count = 0;
        let n = self.samples as f64;

        if n == 0.0 {
            return (0.0, 0.0);
        }

        for i in 0..INPUT_SIZE_BITS {
            for j in 0..HASH_SIZE_BITS {
                for k in (j + 1)..HASH_SIZE_BITS {
                    let pairs_before_j = j * 255 - j * j.saturating_sub(1) / 2;
                    let flat_idx = pairs_before_j + (k - j - 1);

                    let p_j = (self.counts[i][j] as f64) / n;
                    let p_k = (self.counts[i][k] as f64) / n;
                    let p_jk = (self.bic_counts[i][flat_idx] as f64) / n;

                    let var_j = p_j * (1.0 - p_j);
                    let var_k = p_k * (1.0 - p_k);

                    let corr = if var_j > 0.0 && var_k > 0.0 {
                        (p_jk - p_j * p_k) / f64::sqrt(var_j * var_k)
                    } else {
                        0.0
                    };

                    let abs_corr = corr.abs();
                    total_corr += abs_corr;
                    if abs_corr > max_corr {
                        max_corr = abs_corr;
                    }
                    count += 1;
                }
            }
        }

        (total_corr / (count as f64), max_corr)
    }
}

// =============================================================================
// MEASUREMENT FUNCTIONS
// =============================================================================

/// Runs the 1-bit Strict Avalanche Criterion setup for the specified algorithm.
fn run_avalanche_test(algorithm: Algorithm) {
    println!("Running Avalanche Test for {}...", algorithm.name());
    println!(
        "Inputs: {} samples of {} bytes",
        NUM_SAMPLES, INPUT_SIZE_BYTES
    );

    let mut stats = AvalancheStats::new();
    let mut rng = rand::rng();

    let progress_steps = 50;
    let samples_per_step = NUM_SAMPLES / progress_steps;

    let mut current_progress = 0;
    print!("\rProgress: [{:<50}]", "");
    io::stdout().flush().unwrap();

    for s in 0..NUM_SAMPLES {
        if s > 0 && s % samples_per_step == 0 {
            current_progress += 1;
            let filled = "=".repeat(current_progress);
            print!("\rProgress: [{:<50}]", filled);
            io::stdout().flush().unwrap();
        }

        // Generate random base input
        let mut base_input = vec![0u8; INPUT_SIZE_BYTES];
        rng.fill(&mut base_input[..]);

        let base_hash = algorithm.hash(&base_input);

        // Flip each bit and measure
        for i in 0..INPUT_SIZE_BITS {
            let byte_idx = i / 8;
            let bit_idx = i % 8;

            let mut mod_input = base_input.clone();
            mod_input[byte_idx] ^= 1 << bit_idx;

            let mod_hash = algorithm.hash(&mod_input);
            stats.add_sample(&base_input, &base_hash, &mod_hash, i);
        }
        stats.increment_samples();
    }

    print!("\rProgress: [{:=<50}] Done!\n\n", "");
    io::stdout().flush().unwrap();

    let sac = stats.sac_score();
    let max_bias = stats.max_bias();
    let chi_sq = stats.chi_squared_test();
    let (bic_mean, bic_max) = stats.bic_score();

    println!("--------------------------------------------------");
    println!("Results for {}", algorithm.name());
    println!("--------------------------------------------------");
    println!("SAC Score (ideal ~0.5):        {:.6}", sac);
    println!("Max Bias (ideal ~0.0):         {:.6}", max_bias);
    println!("Mean Chi-Squared (ideal ~1.0): {:.6}", chi_sq);
    println!("Mean BIC Corr (ideal ~0.0):    {:.6}", bic_mean);
    println!("Max BIC Corr (ideal ~0.0):     {:.6}", bic_max);
    println!("--------------------------------------------------\n");

    // Simple pass/fail heuristic
    let mut passed = true;
    if (sac - 0.5).abs() > 0.01 {
        println!("FAIL: SAC score deviates significantly from 0.5.");
        passed = false;
    }
    if chi_sq > 3.84 {
        println!("WARN: Chi-Squared stat indicates statistically significant bias.");
    }

    if passed {
        println!("Avalanche Test: PASS");
    } else {
        println!("Avalanche Test: FAIL");
    }
    println!("==================================================\n");
}

// =============================================================================
// MAIN BENCHMARK
// =============================================================================

fn main() {
    println!("==================================================");
    println!("Tachyon Core - Avalanche & Chi-Squared Test");
    println!("==================================================\n");

    run_avalanche_test(Algorithm::Tachyon);
    run_multibit_test(Algorithm::Tachyon);
}

// =============================================================================
// MULTI-BIT BENCHMARK
// =============================================================================

/// Systematically runs the Avalanche Criterion logic for multi-bit flips
fn run_multibit_test(algorithm: Algorithm) {
    println!(
        "Running Multi-Bit Perturbation Test for {}...",
        algorithm.name()
    );
    let flip_counts = [2, 5, 10, 20, 30, 60, 128, 256];
    let samples_per_test = NUM_SAMPLES;
    let expected = (samples_per_test as f64) * 0.5;

    println!(
        "{:<10} | {:<12} | {:<12} | {:<12}",
        "Bits Flipped", "SAC Score", "Max Bias", "Chi-Squared"
    );
    println!("{:-<10}-+-{:-<12}-+-{:-<12}-+-{:-<12}", "", "", "", "");

    let mut rng = rand::rng();

    for &k in &flip_counts {
        print!("{:<10} | ", k);
        io::stdout().flush().unwrap();

        // We track the raw count of how many times each output bit flipped
        // across all `samples_per_test` samples for a given `k`.
        let mut flip_freqs = vec![0u64; HASH_SIZE_BITS];

        let progress_steps = 10;
        let samples_per_step = samples_per_test / progress_steps;
        let mut current_progress = 0;

        print!("\r{:<10} | [{:<10}]", k, "");
        io::stdout().flush().unwrap();

        for s in 0..samples_per_test {
            if s > 0 && s % samples_per_step == 0 {
                current_progress += 1;
                let filled = "=".repeat(current_progress);
                print!("\r{:<10} | [{:<10}]", k, filled);
                io::stdout().flush().unwrap();
            }

            // ── 1. Generate random base input ────────────────────────────────
            let mut base_input = vec![0u8; INPUT_SIZE_BYTES];
            rng.fill(&mut base_input[..]);
            let base_hash = algorithm.hash(&base_input);

            // ── 2. Select random distinct bit indices ────────────────────────
            let indices = rand::seq::index::sample(&mut rng, INPUT_SIZE_BITS, k);

            // ── 3. Flip selected bits ────────────────────────────────────────
            let mut mod_input = base_input.clone();
            for idx in indices {
                let byte_idx = idx / 8;
                let bit_idx = idx % 8;
                mod_input[byte_idx] ^= 1 << bit_idx;
            }

            // ── 4. Hash modified input and compare ───────────────────────────
            let mod_hash = algorithm.hash(&mod_input);
            for (j, flip_frequency) in flip_freqs.iter_mut().enumerate() {
                let byte_idx = j / 8;
                let bit_idx = j % 8;
                let base_bit = (base_hash[byte_idx] >> bit_idx) & 1;
                let mod_bit = (mod_hash[byte_idx] >> bit_idx) & 1;

                if base_bit != mod_bit {
                    *flip_frequency += 1;
                }
            }
        }

        // --- Calculate Stats for this `k` ---
        let mut total_flips = 0;
        let mut max_dev = 0.0;
        let mut chi_sq_sum = 0.0;

        for &flips in &flip_freqs {
            total_flips += flips;

            let p = flips as f64;
            // Max Bias
            let dev = (p - expected).abs() / (samples_per_test as f64);
            if dev > max_dev {
                max_dev = dev;
            }

            // Chi-Squared
            let observed_flips = p;
            let observed_non_flips = (samples_per_test as f64) - p;

            let df = (observed_flips - expected).powi(2) / expected;
            let dnf = (observed_non_flips - expected).powi(2) / expected;
            chi_sq_sum += df + dnf;
        }

        let sac = (total_flips as f64) / ((samples_per_test as f64) * (HASH_SIZE_BITS as f64));
        let mean_chi_sq = chi_sq_sum / (HASH_SIZE_BITS as f64);

        // Erase the progress dots and print the actual stats row
        print!(
            "\r{:<10} | {:<12.6} | {:<12.6} | {:<12.6}\n",
            k, sac, max_dev, mean_chi_sq
        );
    }
    println!("--------------------------------------------------\n");
}
