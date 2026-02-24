//! Consistency & Regression Tests
//!
//! Verifies internal logic consistency, boundary conditions, and architectural invariants.
//! - Streaming vs One-shot consistency
//! - Padding & Length Injection
//! - Lane Isolation (AVX-512)
//! - Bulk Processing Boundaries
//! - Parallel API Fallback & Consistency

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used)]

use tachyon::{hash, hash_parallel, Hasher};

// =============================================================================
// STREAMING CONSISTENCY
// =============================================================================

#[test]
fn test_streaming_consistency() {
    // Verify that one-shot and streaming (parallel) produce SAME results for various sizes.
    // Critical for fixing the "128KB - 1MB" inconsistency reported.
    let sizes = [
        64,              // Kernel Short Path
        1024,            // Kernel Bulk Path (Serial)
        128 * 1024,      // Exactly CHUNK_SIZE (Transition point)
        512 * 1024,      // Previously inconsistent range
        2 * 1024 * 1024, // Definitely parallel
    ];

    for &size in &sizes {
        let input = vec![0u8; size];
        let h_oneshot = hash(&input);

        let mut hasher = Hasher::new().unwrap();
        hasher.update(&input);
        let h_streaming = hasher.finalize();

        assert_eq!(
            h_oneshot, h_streaming,
            "CONSISTENCY FAILURE at size {size}: one-shot and streaming produced different hashes!",
        );
    }
}

// =============================================================================
// PARALLEL API CONSISTENCY
// =============================================================================

#[test]
fn test_parallel_fallback() {
    // 64 KB input (smaller than 128 KB chunk size)
    // Verify that hash_parallel correctly falls back to serial hash logic
    let data = vec![0x11u8; 64 * 1024];

    let h_serial = hash(&data);
    let h_parallel = hash_parallel(&data);

    assert_eq!(
        h_serial, h_parallel,
        "Small inputs should fallback to serial hash (Same Logic)"
    );
}

#[test]
fn test_hash_consistency_with_parallel() {
    // 200 KB input (larger than 128 KB chunk size to trigger parallel mode)
    let data = vec![0xABu8; 200 * 1024];

    let h_serial = hash(&data);
    let h_parallel = hash_parallel(&data);

    assert_eq!(
        h_serial, h_parallel,
        "hash() (Oneshot) and hash_parallel() (Parallel) MUST be consistent"
    );
}

#[test]
fn test_parallel_determinism() {
    let data = vec![0x42u8; 1024 * 1024]; // 1 MB
    let h1 = hash_parallel(&data);
    let h2 = hash_parallel(&data);
    assert_eq!(h1, h2, "Parallel hash must be deterministic");
}

// =============================================================================
// BOUNDARY CONDITIONS & PADDING
// =============================================================================

#[test]
fn test_exact_boundary_conditions() {
    // The dispatcher switches at 64 bytes (Short Path < 64).
    // Let's test right around that boundary.
    let sizes = [0, 1, 15, 16, 31, 32, 63, 64, 127, 128];

    for size in sizes {
        let input = vec![0u8; size];
        let h1 = hash(&input);
        let h2 = hash(&input);

        // Determinism Check
        assert_eq!(h1, h2, "Hash not deterministic for size {size}");

        // Basic Quality Check: Output should not be all zeros
        assert_ne!(h1, [0u8; 32], "Hash is all zeros for size {size}");
    }
}

#[test]
fn test_padding_correctness() {
    // Ensure that "A" and "A\0" produce different hashes.
    // This verifies that the length injection works properly.
    let h1 = hash(b"A");
    let h2 = hash(b"A\0");

    assert_ne!(
        h1, h2,
        "Collision between 'A' and 'A\\0' - Length injection failed!"
    );
}

#[test]
fn test_length_commitment_security() {
    // Verify that the hash depends on the total length.
    // This prevents Merkle Tree extension attacks.
    let input = vec![0u8; 128 * 1024]; // 1 full chunk
    let h1 = hash(&input);
    let h2 = hash(&input[..64 * 1024]); // Half chunk

    assert_ne!(h1, h2);
}

// =============================================================================
// ARCHITECTURAL ISOLATION (AVX-512 / AES-NI)
// =============================================================================

#[test]
fn test_bulk_track_isolation() {
    // Test if changing one track in a large block affects the entire hash.
    // 512-byte input triggers bulk path.
    let input_a = [0u8; 512];
    let mut input_b = [0u8; 512];
    input_b[0] ^= 1; // Flip 1st bit in 1st track

    let h_a = hash(&input_a);
    let h_b = hash(&input_b);

    // Count flipped bits in each 64-bit word of the 256-bit hash
    let mut word_flips = [0u32; 4];
    for i in 0..32 {
        word_flips[i / 8] += (h_a[i] ^ h_b[i]).count_ones();
    }

    for (i, flips) in word_flips.iter().enumerate() {
        assert!(
            *flips > 0,
            "BULK TRACK ISOLATION DETECTED: Word {i} of the hash was not affected by input in Track 0!",
        );
    }
}

#[test]
fn test_lane_avalanche() {
    // Specifically test if bits from different 128-bit lanes (AVX-512)
    // affect the same parts of the hash (Lane Mixing).
    let mut input_a = vec![0u8; 128 * 1024]; // 128 KB
    let mut input_b = input_a.clone();

    // Data in Lane 0 (bits 0..127) vs Lane 1 (bits 128..255)
    input_a[0] ^= 1;
    input_b[16] ^= 1; // 128th bit

    let h_a = hash(&input_a);
    let h_b = hash(&input_b);

    assert_ne!(h_a, h_b, "Hash must differ for changes in different lanes!");

    // Check diffusion
    let h_base = hash(&vec![0u8; 128 * 1024]);
    let mut flips = 0;
    for i in 0..32 {
        flips += (h_base[i] ^ h_b[i]).count_ones();
    }
    assert!(
        flips > 100,
        "Lane change did not diffuse enough! Flips: {flips}",
    );
}
