//! Domain Separation Tests
//!
//! Validates that domain separation works correctly for oneshot and streaming APIs.

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use tachyon::{hash, hash_with_domain, TachyonDomain};

#[cfg(feature = "multithread")]
use tachyon::Hasher;

// =============================================================================
// BASIC TESTS
// =============================================================================

#[test]
fn test_domain_basic_hashing() {
    let data = b"Hello, Tachyon!";

    let hash1 = hash(data);
    let hash2 = hash(data);

    assert_eq!(hash1, hash2, "Same input should produce same hash");
}

#[test]
fn test_different_domains_produce_different_hashes() {
    let data = b"Hello, Tachyon!";

    let generic = hash_with_domain(data, TachyonDomain::Generic);
    let file_checksum = hash_with_domain(data, TachyonDomain::FileChecksum);
    let key_deriv = hash_with_domain(data, TachyonDomain::KeyDerivation);
    let msg_auth = hash_with_domain(data, TachyonDomain::MessageAuth);
    let db_index = hash_with_domain(data, TachyonDomain::DatabaseIndex);
    let content_addr = hash_with_domain(data, TachyonDomain::ContentAddressed);

    // All pairs must differ
    assert_ne!(generic, file_checksum);
    assert_ne!(generic, key_deriv);
    assert_ne!(generic, msg_auth);
    assert_ne!(generic, db_index);
    assert_ne!(generic, content_addr);
    assert_ne!(file_checksum, key_deriv);
    assert_ne!(file_checksum, msg_auth);
    assert_ne!(file_checksum, db_index);
    assert_ne!(key_deriv, msg_auth);
    assert_ne!(msg_auth, db_index);
}

#[test]
fn test_hash_equals_generic_domain() {
    let data = b"Test data";

    assert_eq!(
        hash(data),
        hash_with_domain(data, TachyonDomain::Generic),
        "hash() should equal hash_with_domain(Generic)"
    );
}

// =============================================================================
// STREAMING TESTS
// =============================================================================

#[test]
#[cfg(feature = "multithread")]
fn test_streaming_matches_oneshot() {
    let data = b"Hello, Tachyon!";

    let mut hasher = Hasher::new().unwrap();
    hasher.update(b"Hello, ");
    hasher.update(b"Tachyon!");
    let stream_hash = hasher.finalize();

    assert_eq!(stream_hash, hash(data), "Streaming should match oneshot");
}

#[test]
#[cfg(feature = "multithread")]
fn test_streaming_with_domain() {
    let data = b"Test data for domain";

    // Test each domain
    for domain in [
        TachyonDomain::Generic,
        TachyonDomain::FileChecksum,
        TachyonDomain::KeyDerivation,
        TachyonDomain::MessageAuth,
        TachyonDomain::DatabaseIndex,
        TachyonDomain::ContentAddressed,
    ] {
        let mut hasher = Hasher::new_with_domain(domain.to_u64()).unwrap();
        hasher.update(data);
        let stream_result = hasher.finalize();

        let oneshot_result = hash_with_domain(data, domain);

        assert_eq!(
            stream_result, oneshot_result,
            "Streaming with domain {:?} should match oneshot",
            domain
        );
    }
}

#[test]
#[cfg(feature = "multithread")]
fn test_streaming_incremental_with_domain() {
    let data = b"Incremental streaming test";

    let mut hasher = Hasher::new_with_domain(TachyonDomain::FileChecksum.to_u64()).unwrap();
    for &byte in data {
        hasher.update(&[byte]);
    }
    let result = hasher.finalize();

    let expected = hash_with_domain(data, TachyonDomain::FileChecksum);
    assert_eq!(result, expected, "Incremental streaming should match");
}

// =============================================================================
// MULTI-CHUNK TESTS
// =============================================================================

#[test]
fn test_domain_separation_large_input() {
    // Use various large sizes without assuming implementation details
    let sizes = [100_000, 500_000, 1_000_000];

    for &size in &sizes {
        let large = vec![42u8; size];

        let generic = hash_with_domain(&large, TachyonDomain::Generic);
        let file = hash_with_domain(&large, TachyonDomain::FileChecksum);
        let kdf = hash_with_domain(&large, TachyonDomain::KeyDerivation);

        assert_ne!(generic, file, "Size {}: Generic vs FileChecksum", size);
        assert_ne!(generic, kdf, "Size {}: Generic vs KeyDerivation", size);
        assert_ne!(file, kdf, "Size {}: FileChecksum vs KeyDerivation", size);
    }
}

#[test]
#[cfg(feature = "multithread")]
fn test_streaming_large_with_domain() {
    let large = vec![0xAAu8; 200_000]; // 200 KB - arbitrary large size

    let mut hasher = Hasher::new_with_domain(TachyonDomain::FileChecksum.to_u64()).unwrap();
    // Update in arbitrary chunk size (not aligned to implementation)
    for chunk in large.chunks(17_000) {
        hasher.update(chunk);
    }
    let stream_result = hasher.finalize();

    let oneshot_result = hash_with_domain(&large, TachyonDomain::FileChecksum);

    assert_eq!(
        stream_result, oneshot_result,
        "Large streaming with domain should match"
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_domain_separation_empty_input() {
    let empty = b"";

    let generic = hash_with_domain(empty, TachyonDomain::Generic);
    let file = hash_with_domain(empty, TachyonDomain::FileChecksum);

    assert_ne!(generic, file, "Empty input: domains should still differ");
}

#[test]
fn test_domain_separation_single_byte() {
    let single = b"X";

    let domains = [
        TachyonDomain::Generic,
        TachyonDomain::FileChecksum,
        TachyonDomain::KeyDerivation,
    ];

    let hashes: Vec<_> = domains
        .iter()
        .map(|&d| hash_with_domain(single, d))
        .collect();

    // All must be unique
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(
                hashes[i], hashes[j],
                "Single byte: domain {:?} vs {:?}",
                domains[i], domains[j]
            );
        }
    }
}

// =============================================================================
// CONSISTENCY TESTS
// =============================================================================

#[test]
fn test_domain_consistency_across_sizes() {
    // Use powers of 2 and primes to test various boundaries without assuming internals
    let sizes = [
        1, 7, 15, 31, 63, 127, 255, 511, // Small boundaries
        1_000, 5_000, 10_000, 50_000, // Medium arbitrary sizes
        100_000, 250_000, 500_000, 1_000_000, // Large sizes
    ];

    for &size in &sizes {
        let data = vec![0x42u8; size];

        let generic = hash_with_domain(&data, TachyonDomain::Generic);
        let file = hash_with_domain(&data, TachyonDomain::FileChecksum);

        assert_ne!(
            generic, file,
            "Size {}: Generic and FileChecksum must differ",
            size
        );

        // Verify consistency: hashing twice should give same result
        let generic2 = hash_with_domain(&data, TachyonDomain::Generic);
        let file2 = hash_with_domain(&data, TachyonDomain::FileChecksum);

        assert_eq!(
            generic, generic2,
            "Size {}: Generic should be consistent",
            size
        );
        assert_eq!(
            file, file2,
            "Size {}: FileChecksum should be consistent",
            size
        );
    }
}

// =============================================================================
// CROSS-BOUNDARY TESTS (no knowledge of internal thresholds)
// =============================================================================

#[test]
fn test_domain_separation_at_various_boundaries() {
    // Test around common power-of-2 boundaries that any hash might use
    let test_sizes = [
        (4095, 4096, 4097),       // 4K boundary
        (8191, 8192, 8193),       // 8K boundary
        (16383, 16384, 16385),    // 16K boundary
        (32767, 32768, 32769),    // 32K boundary
        (131071, 131072, 131073), // 128K boundary
        (262143, 262144, 262145), // 256K boundary
        (524287, 524288, 524289), // 512K boundary
    ];

    for &(before, at, after) in &test_sizes {
        for &size in &[before, at, after] {
            let data = vec![0x99u8; size];

            let generic = hash_with_domain(&data, TachyonDomain::Generic);
            let file = hash_with_domain(&data, TachyonDomain::FileChecksum);

            assert_ne!(
                generic, file,
                "Boundary test at size {}: domains must differ",
                size
            );
        }
    }
}

#[test]
#[cfg(feature = "multithread")]
fn test_streaming_misaligned_updates() {
    // Test with deliberately misaligned chunk sizes to stress-test implementation
    let data = vec![0x5A; 100_000];
    let chunk_sizes = [1, 7, 127, 1_001, 8_191, 16_383];

    for &chunk_size in &chunk_sizes {
        let mut hasher = Hasher::new_with_domain(TachyonDomain::FileChecksum.to_u64()).unwrap();

        for chunk in data.chunks(chunk_size) {
            hasher.update(chunk);
        }

        let stream_result = hasher.finalize();
        let oneshot_result = hash_with_domain(&data, TachyonDomain::FileChecksum);

        assert_eq!(
            stream_result, oneshot_result,
            "Misaligned streaming (chunk_size={}) should match oneshot",
            chunk_size
        );
    }
}
