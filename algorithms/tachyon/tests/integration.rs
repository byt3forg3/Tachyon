//! Integration Tests
//!
//! Verifies the public API of the Tachyon library.
//! Ensures determinism, correct output size, and parallel processing consistency.

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(unsafe_code)]

// =============================================================================
// BASIC TESTS
// =============================================================================

#[test]
fn test_hash_consistency() {
    let input = b"Hello, Tachyon!";
    let hash1 = tachyon::hash(input);
    let hash2 = tachyon::hash(input);

    // Determinism check
    assert_eq!(hash1, hash2, "Hash must be deterministic");

    // Smoke check (not empty)
    assert_ne!(hash1, [0u8; 32], "Hash should not be all zeros");
}

#[test]
fn test_backend_reporting() {
    let backend = tachyon::active_backend();
    println!("API Hardware detected: {backend}");
    assert!(!backend.is_empty(), "Backend name should not be empty");
}

#[test]
fn test_large_input() {
    // Check parallel threshold
    let input = vec![0x42u8; 1024 * 1024]; // 1MB
    let hash = tachyon::hash(&input);
    assert_ne!(hash, [0u8; 32]);
}

#[test]
fn test_verify() {
    let input = b"Secure Data";
    let hash = tachyon::hash(input);
    assert!(
        tachyon::verify(input, &hash),
        "Verification should succeed for correct hash"
    );

    let mut bad_hash = hash;
    bad_hash[0] ^= 0xFF;
    assert!(
        !tachyon::verify(input, &bad_hash),
        "Verification should fail for incorrect hash"
    );
}

// =============================================================================
// STREAMING TESTS
// =============================================================================

#[test]
fn test_streaming() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use tachyon::Hasher;

        let input = b"StreamingChunk1Chunk2";
        let part1 = b"Streaming";
        let part2 = b"Chunk1";
        let part3 = b"Chunk2";

        // 1. One-Shot Baseline
        let expected = tachyon::hash(input);

        // 2. Streaming w/ Chunks
        let mut hasher = Hasher::new().expect("CPU feature check failed");
        hasher.update(part1);
        hasher.update(part2);
        hasher.update(part3);
        let stream_hash = hasher.finalize();

        assert_eq!(
            expected, stream_hash,
            "Streaming hash must match one-shot hash"
        );
    }
}

#[test]
fn test_default_trait() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use tachyon::Hasher;

        // Test that Default trait works
        let mut hasher = Hasher::default();
        hasher.update(b"test data");
        let hash = hasher.finalize();

        // Should match one-shot
        let expected = tachyon::hash(b"test data");
        assert_eq!(hash, expected, "Default trait must work correctly");
    }
}

#[test]
fn test_streaming_edge_cases() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use tachyon::Hasher;

        // --- Edge Case 1: Empty Input ---
        let hasher_empty = Hasher::new().expect("CPU feature check failed");
        let hash_empty = hasher_empty.finalize();
        let expected_empty = tachyon::hash(b"");
        assert_eq!(
            hash_empty, expected_empty,
            "Empty streaming hash must match empty one-shot"
        );

        // --- Edge Case 2: Exact 512-Byte Boundary ---
        let data_512 = vec![0x42u8; 512];
        let mut hasher_512 = Hasher::new().expect("CPU feature check failed");
        hasher_512.update(&data_512);
        let hash_512 = hasher_512.finalize();
        let expected_512 = tachyon::hash(&data_512);
        assert_eq!(
            hash_512, expected_512,
            "512-byte boundary hash must match one-shot"
        );

        // --- Edge Case 3: Buffer Overflow Protection (511 + 2 bytes) ---
        let part1_511 = vec![0xAAu8; 511];
        let part2_2 = vec![0xBBu8; 2];
        let combined = [part1_511.as_slice(), part2_2.as_slice()].concat();

        let mut hasher_overflow = Hasher::new().expect("CPU feature check failed");
        hasher_overflow.update(&part1_511);
        hasher_overflow.update(&part2_2);
        let hash_overflow = hasher_overflow.finalize();
        let expected_overflow = tachyon::hash(&combined);
        assert_eq!(
            hash_overflow, expected_overflow,
            "Buffer overflow scenario must match one-shot"
        );

        // --- Edge Case 4: Multiple Small Updates ---
        let mut hasher_small = Hasher::new().expect("CPU feature check failed");
        for i in 0..100 {
            hasher_small.update(&[i as u8]);
        }
        let hash_small = hasher_small.finalize();
        let data_small: Vec<u8> = (0..100).map(|i| i as u8).collect();
        let expected_small = tachyon::hash(&data_small);
        assert_eq!(
            hash_small, expected_small,
            "Multiple small updates must match one-shot"
        );

        // --- Edge Case 5: Large Input (>512 bytes) ---
        let data_large = vec![0x33u8; 2048];
        let mut hasher_large = Hasher::new().expect("CPU feature check failed");
        hasher_large.update(&data_large[..1000]);
        hasher_large.update(&data_large[1000..]);
        let hash_large = hasher_large.finalize();
        let expected_large = tachyon::hash(&data_large);
        assert_eq!(
            hash_large, expected_large,
            "Large streaming input must match one-shot"
        );
    }
}

// =============================================================================
// PARALLEL TESTS
// =============================================================================

#[test]
#[cfg(feature = "multithread")]
fn test_streaming_parallel() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use tachyon::Hasher;

        // Test 1: Small input (below CHUNK_SIZE) - matches oneshot
        let small_data = b"Small test data for parallel hasher";
        let mut hasher_small = Hasher::new().expect("CPU feature check failed");
        hasher_small.update(small_data);
        let hash_small = hasher_small.finalize();
        let expected_small = tachyon::hash(small_data);
        assert_eq!(
            hash_small, expected_small,
            "Streaming must match one-shot for small inputs"
        );

        // Test 2: Multiple updates produce consistent result
        let chunk_size = 64 * 1024;
        let mut hasher1 = Hasher::new().expect("CPU feature check failed");
        let mut hasher2 = Hasher::new().expect("CPU feature check failed");

        for i in 0..5 {
            let chunk = vec![i as u8; chunk_size];
            hasher1.update(&chunk);
            hasher2.update(&chunk);
        }

        assert_eq!(
            hasher1.finalize(),
            hasher2.finalize(),
            "Same updates must produce same hash"
        );

        // Test 3: Single update vs multiple updates with same total data
        let total_data = vec![0x42u8; 128 * 1024];
        let mut hasher_single = Hasher::new().expect("CPU feature check failed");
        hasher_single.update(&total_data);

        let mut hasher_multi = Hasher::new().expect("CPU feature check failed");
        for chunk in total_data.chunks(32 * 1024) {
            hasher_multi.update(chunk);
        }

        assert_eq!(
            hasher_single.finalize(),
            hasher_multi.finalize(),
            "Single vs multiple updates with same data must match"
        );

        // Test 4: Empty input
        let hasher_empty = Hasher::new().expect("CPU feature check failed");
        let hash_empty = hasher_empty.finalize();
        let expected_empty = tachyon::hash(b"");
        assert_eq!(
            hash_empty, expected_empty,
            "Empty streaming must match empty oneshot"
        );

        // Test 5: Large input - streaming must match oneshot
        let large_data = vec![0xAB_u8; 512 * 1024]; // 512 KB
        let mut hasher_large = Hasher::new().expect("CPU feature check failed");
        hasher_large.update(&large_data);
        let hash_large = hasher_large.finalize();
        let expected_large = tachyon::hash(&large_data);
        assert_eq!(
            hash_large, expected_large,
            "Streaming must match oneshot for large inputs"
        );
    }
}
