//! Cross-Architecture Consistency Tests
//!
//! Verifies that AES-NI, AVX-512 and Portable backends produce IDENTICAL results
//! for all modes of operation (Standard, Seeded, Keyed).
//! This ensures that CPU feature detection does not alter the cryptographic output.
//!
//! Coverage:
//! - Standard Hashing (Empty, Small, Large, Unaligned)
//! - Seeded Hashing (Verifies Fix 7: Seed Mixing)
//! - Keyed Hashing (Verifies Fix 2: Key-Block Asymmetry)

#![allow(unsafe_code)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

use tachyon::kernels;

fn is_avx512_supported() -> bool {
    is_x86_feature_detected!("avx512f")
        && is_x86_feature_detected!("avx512bw")
        && is_x86_feature_detected!("vaes")
        && is_x86_feature_detected!("vpclmulqdq")
}

// =============================================================================
// STANDARD HASH CONSISTENCY
// =============================================================================

#[test]
fn test_standard_consistency() {
    if !is_avx512_supported() {
        println!("Skipping: AVX-512 not supported.");
        return;
    }

    let scenarios: Vec<(&str, Vec<u8>)> = vec![
        ("Empty", vec![]),
        ("Small", b"Tachyon".to_vec()),
        ("Exact Block (64)", vec![0u8; 64]),
        ("Exact Block (512)", vec![1u8; 512]),
        ("Unaligned (63)", vec![2u8; 63]),
        ("Unaligned (513)", vec![3u8; 513]),
        ("Large (1024)", vec![1u8; 1024]),
        ("Large (4096)", vec![b'c'; 4096]),
        ("Prime Length (101)", vec![0u8; 101]),
    ];

    for (name, input) in scenarios {
        unsafe {
            let aesni = kernels::aesni::oneshot(&input, 0, 0, None);
            let avx512 = kernels::avx512::oneshot(&input, 0, 0, None);

            assert_eq!(aesni, avx512, "Mismatch in Standard Mode: {name}");
        }
    }
}

// =============================================================================
// SEEDED CONSISTENCY (Fix 7 Verification)
// =============================================================================

#[test]
fn test_seeded_consistency() {
    if !is_avx512_supported() {
        return;
    }

    // Test specific seeds including edge cases
    let seeds = [
        0,                     // Default
        1,                     // Small
        0xDEAD_BEEF,           // Pattern
        u64::MAX,              // All ones
        0x5555_5555_5555_5555, // Alternating
    ];
    let input = b"SeededInputData";

    for seed in seeds {
        unsafe {
            let aesni = kernels::aesni::oneshot(input, 0, seed, None);
            let avx512 = kernels::avx512::oneshot(input, 0, seed, None);

            assert_eq!(aesni, avx512, "Mismatch in Seeded Mode: Seed={seed:x}");
        }
    }
}

// =============================================================================
// KEYED CONSISTENCY (Fix 2 Verification)
// =============================================================================

#[test]
fn test_keyed_consistency() {
    if !is_avx512_supported() {
        return;
    }

    let keys = [
        [0u8; 32],                            // Zero Key
        [0xFFu8; 32],                         // All Ones Key
        *b"12345678901234567890123456789012", // ASCII Key
        {
            let mut k = [0u8; 32];
            k[0] = 1;
            k[31] = 1;
            k
        }, // Sparse Key
    ];
    let input = b"KeyedMessageAuthCode";

    for key in keys {
        unsafe {
            let aesni = kernels::aesni::oneshot(input, 0, 0, Some(&key));
            let avx512 = kernels::avx512::oneshot(input, 0, 0, Some(&key));

            assert_eq!(
                aesni,
                avx512,
                "Mismatch in Keyed Mode (MAC): Key={:x?}",
                hex::encode(key)
            );
        }
    }
}

// =============================================================================
// DOMAIN CONSISTENCY
// =============================================================================

#[test]
fn test_domain_consistency() {
    if !is_avx512_supported() {
        return;
    }

    let domains = [0, 1, 5, 100, u64::MAX];
    let input = b"DomainSeparated";

    for domain in domains {
        unsafe {
            let aesni = kernels::aesni::oneshot(input, domain, 0, None);
            let avx512 = kernels::avx512::oneshot(input, domain, 0, None);

            assert_eq!(aesni, avx512, "Mismatch in Domain Mode: Domain={domain}");
        }
    }
}
// =============================================================================
// PURE RUST CROSS-ARCH VERIFICATION (Auto vs Portable)
// =============================================================================

use tachyon::hash;

#[test]
fn test_auto_vs_portable() {
    let scenarios: Vec<(&str, Vec<u8>)> = vec![
        ("Empty", vec![]),
        ("Small (7 bytes)", b"Tachyon".to_vec()),
        ("Exact Block (64)", vec![0u8; 64]),
        ("Exact Block (512)", vec![1u8; 512]),
        ("Unaligned (63)", vec![2u8; 63]),
        ("Unaligned (513)", vec![3u8; 513]),
        // Border of chunk size
        ("Medium (1000)", vec![0u8; 1000]),
        ("Large (1MB - 1)", vec![0xAAu8; 1024 * 1024 - 1]),
    ];

    for (name, input) in scenarios {
        // 1. Auto-detected (Best available on this machine)
        let hash_auto = hash(&input);

        // 2. Forced Portable
        // Note: We use 0 for domain/seed and None for key to match tachyon::hash defaults
        let hash_portable = kernels::portable::oneshot(&input, 0, 0, None);

        assert_eq!(
            hash_auto, hash_portable,
            "Mismatch Auto vs Portable: {}",
            name
        );
    }
}

// Property-based test for random lengths
#[test]
fn test_random_lengths() {
    // Simple pseudo-random generator to avoid dependencies
    let mut rng = 0xDEAD_BEEF_CAFE_BABE_u64;
    let mut next_u64 = || {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        rng
    };

    for _ in 0..100 {
        let len = (next_u64() % 8192) as usize;
        let mut input = vec![0u8; len];
        for b in &mut input {
            *b = (next_u64() & 0xFF) as u8;
        }

        let hash_auto = hash(&input);
        let hash_portable = kernels::portable::oneshot(&input, 0, 0, None);

        assert_eq!(
            hash_auto, hash_portable,
            "Mismatch Auto vs Portable on random input len={}",
            len
        );
    }
}
