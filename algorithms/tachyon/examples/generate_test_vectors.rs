//! Generator for Tachyon test vectors
//!
//! Generates the canonical JSON test vectors used by `tests/test_vectors.json`.
//! Includes Empty, Small, Large, and specific boundary conditions.
#![allow(clippy::unwrap_used, clippy::print_stdout)]
use serde_json::json;

fn main() {
    let mut vectors = Vec::new();

    // ── 1. Basic vectors ─────────────────────────────────────────────────────

    // Validates standard ASCII input
    let input_basic = b"abc";
    vectors.push(json!({
        "name": "basic",
        "input": "abc",
        "hash": hex::encode(tachyon::hash(input_basic))
    }));

    // Validates empty input handling (padding/length encoding)
    let input_empty = b"";
    vectors.push(json!({
        "name": "empty",
        "input": "",
        "hash": hex::encode(tachyon::hash(input_empty))
    }));

    // ── 2. Boundary conditions ───────────────────────────────────────────────

    // Large Input (1KB) - Triggers bulk processing
    let input_large = vec![0x41u8; 1024];
    vectors.push(json!({
        "name": "large",
        "input": "LARGE_1KB",
        "hash": hex::encode(tachyon::hash(&input_large))
    }));

    // Medium input (256 bytes), used by C/Java binding tests
    let input_medium = vec![0x41u8; 256];
    vectors.push(json!({
        "name": "medium_256",
        "input": "MEDIUM_256_A",
        "hash": hex::encode(tachyon::hash(&input_medium))
    }));

    // Small input ("Tachyon")
    let input_small = b"Tachyon";
    vectors.push(json!({
        "name": "small",
        "input": "Tachyon",
        "hash": hex::encode(tachyon::hash(input_small))
    }));

    // Exact 64-byte block (64 x 0x00)
    let input_exact_64 = vec![0x00u8; 64];
    vectors.push(json!({
        "name": "exact_block_64",
        "input": "EXACT_64_ZERO",
        "hash": hex::encode(tachyon::hash(&input_exact_64))
    }));

    // Exact 512-byte block (512 x 0x01)
    let input_exact_512 = vec![0x01u8; 512];
    vectors.push(json!({
        "name": "exact_block_512",
        "input": "EXACT_512_ONE",
        "hash": hex::encode(tachyon::hash(&input_exact_512))
    }));

    // Unaligned 63-byte input (63 x 0x02)
    let input_unaligned_63 = vec![0x02u8; 63];
    vectors.push(json!({
        "name": "unaligned_63",
        "input": "UNALIGNED_63_TWO",
        "hash": hex::encode(tachyon::hash(&input_unaligned_63))
    }));

    // ── 3. Extreme inputs ────────────────────────────────────────────────────

    // Very Large (1MB) - Stresses buffer management and parallelism
    let input_huge = vec![0x41u8; 1024 * 1024];
    vectors.push(json!({
        "name": "huge",
        "input": "HUGE_1MB",
        "hash": hex::encode(tachyon::hash(&input_huge))
    }));

    let output = json!({ "vectors": vectors });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
