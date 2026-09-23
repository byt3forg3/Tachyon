//! Cross-Architecture Consistency Tests
//!
//! Verifies that AES-NI, AVX-512 and Portable backends produce IDENTICAL results
//! for all input sizes and modes (standard, seeded, keyed, domain, streaming).

#![allow(unsafe_code)]
#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation, clippy::print_stdout)]

use tachyon::{hash, kernels, Hasher};

fn is_avx512_supported() -> bool {
    is_x86_feature_detected!("avx512f")
        && is_x86_feature_detected!("avx512bw")
        && is_x86_feature_detected!("vaes")
        && is_x86_feature_detected!("vpclmulqdq")
}

// Boundaries: BLOCK_SIZE=512, REMAINDER_CHUNK_SIZE=64.
const SIZES: &[usize] = &[
    0, 1, 7, 15, 31, // small
    63, 64, 65, // remainder chunk boundaries
    511, 512, 513, // block boundaries
    1023, 1024, 1025, // 2-block boundaries
    4096, 10007, // larger bulk
];

// =============================================================================
// 1. ALL-BACKEND CONSISTENCY (AVX-512 required)
// =============================================================================

#[test]
fn test_all_backends_consistency() {
    if !is_avx512_supported() {
        println!("Skipping: AVX-512 not supported.");
        return;
    }

    for &size in SIZES {
        let input: Vec<u8> = (0..size).map(|i| (i as u8).wrapping_mul(0x37)).collect();
        unsafe {
            let aesni = kernels::aesni::oneshot(&input, 0, 0, None);
            let avx512 = kernels::avx512::oneshot(&input, 0, 0, None);
            let portable = kernels::portable::oneshot(&input, 0, 0, None);

            assert_eq!(aesni, avx512, "AES-NI vs AVX-512: size={size}");
            assert_eq!(avx512, portable, "AVX-512 vs Portable: size={size}");
        }
    }
}

// =============================================================================
// 2. SEEDED CONSISTENCY (Fix 7 Verification)
// =============================================================================

#[test]
fn test_seeded_consistency() {
    if !is_avx512_supported() {
        return;
    }

    let seeds = [0u64, 1, 0xDEAD_BEEF, u64::MAX, 0x5555_5555_5555_5555];
    // Multiple sizes to exercise short, remainder, and block code paths.
    let sizes = [1usize, 64, 512, 1024, 4096];

    for size in sizes {
        let input: Vec<u8> = (0..size).map(|i| i as u8).collect();
        for seed in seeds {
            unsafe {
                let aesni = kernels::aesni::oneshot(&input, 0, seed, None);
                let avx512 = kernels::avx512::oneshot(&input, 0, seed, None);
                let portable = kernels::portable::oneshot(&input, 0, seed, None);

                assert_eq!(
                    aesni, avx512,
                    "AES-NI vs AVX-512: size={size} seed={seed:#x}"
                );
                assert_eq!(
                    avx512, portable,
                    "AVX-512 vs Portable: size={size} seed={seed:#x}"
                );
            }
        }
    }
}

// =============================================================================
// 3. KEYED CONSISTENCY (Fix 2 Verification)
// =============================================================================

#[test]
fn test_keyed_consistency() {
    if !is_avx512_supported() {
        return;
    }

    let keys: &[[u8; 32]] = &[
        [0u8; 32],
        [0xFFu8; 32],
        *b"12345678901234567890123456789012",
        {
            let mut k = [0u8; 32];
            k[0] = 1;
            k[31] = 1;
            k
        },
    ];
    let sizes = [1usize, 64, 512, 1024];

    for size in sizes {
        let input: Vec<u8> = (0..size).map(|i| i as u8).collect();
        for key in keys {
            unsafe {
                let aesni = kernels::aesni::oneshot(&input, 0, 0, Some(key));
                let avx512 = kernels::avx512::oneshot(&input, 0, 0, Some(key));
                let portable = kernels::portable::oneshot(&input, 0, 0, Some(key));

                assert_eq!(
                    aesni, avx512,
                    "AES-NI vs AVX-512: size={size} key[0]={:#x}",
                    key[0]
                );
                assert_eq!(
                    avx512, portable,
                    "AVX-512 vs Portable: size={size} key[0]={:#x}",
                    key[0]
                );
            }
        }
    }
}

// =============================================================================
// 4. DOMAIN CONSISTENCY
// =============================================================================

#[test]
fn test_domain_consistency() {
    if !is_avx512_supported() {
        return;
    }

    let domains = [0u64, 1, 5, 100, u64::MAX];
    let sizes = [1usize, 64, 512, 1024];

    for size in sizes {
        let input: Vec<u8> = (0..size).map(|i| i as u8).collect();
        for domain in domains {
            unsafe {
                let aesni = kernels::aesni::oneshot(&input, domain, 0, None);
                let avx512 = kernels::avx512::oneshot(&input, domain, 0, None);
                let portable = kernels::portable::oneshot(&input, domain, 0, None);

                assert_eq!(
                    aesni, avx512,
                    "AES-NI vs AVX-512: size={size} domain={domain}"
                );
                assert_eq!(
                    avx512, portable,
                    "AVX-512 vs Portable: size={size} domain={domain}"
                );
            }
        }
    }
}

// =============================================================================
// 5. STREAMING CONSISTENCY — runs on ALL hardware
// =============================================================================
// TachyonHasher with any chunk pattern must produce the same result as hash().

#[test]
fn test_streaming_consistency() {
    let chunk_sizes: &[usize] = &[1, 7, 32, 64, 100, 256, 512];
    let sizes: &[usize] = &[1, 7, 64, 100, 512, 1000, 1024, 4096];

    for &total in sizes {
        let input: Vec<u8> = (0..total).map(|i| (i as u8).wrapping_mul(0x37)).collect();
        let reference = hash(&input);

        for &chunk in chunk_sizes {
            if chunk > total {
                continue;
            }
            let mut h = Hasher::new();
            for part in input.chunks(chunk) {
                h.update(part);
            }
            let streamed = h.finalize();
            assert_eq!(
                reference, streamed,
                "Streaming mismatch: total={total} chunk={chunk}"
            );
        }
    }
}

// =============================================================================
// 6. AUTO VS PORTABLE — runs on ALL hardware
// =============================================================================

#[test]
fn test_auto_vs_portable() {
    for &size in SIZES {
        let input: Vec<u8> = (0..size).map(|i| (i as u8).wrapping_mul(0x37)).collect();
        let hash_auto = hash(&input);
        let hash_portable = kernels::portable::oneshot(&input, 0, 0, None);
        assert_eq!(hash_auto, hash_portable, "Auto vs Portable: size={size}");
    }
}

// =============================================================================
// 7. RANDOM CROSS-ARCH — requires AVX-512
// =============================================================================
// 200 random inputs + all path-boundary sizes; all three backends compared.

#[test]
fn test_random_cross_arch() {
    let mut rng = 0xDEAD_BEEF_CAFE_BABE_u64;
    let mut next_u64 = || {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        rng
    };

    let mut sizes: Vec<usize> = SIZES.to_vec();
    for _ in 0..200 {
        sizes.push((next_u64() % 8192) as usize);
    }

    let avx512_ok = is_avx512_supported();

    for len in sizes {
        let seed = next_u64();
        let mut input = vec![0u8; len];
        for b in &mut input {
            *b = (next_u64() & 0xFF) as u8;
        }

        if avx512_ok {
            unsafe {
                let aesni = kernels::aesni::oneshot(&input, 0, seed, None);
                let avx512 = kernels::avx512::oneshot(&input, 0, seed, None);
                let portable = kernels::portable::oneshot(&input, 0, seed, None);

                assert_eq!(aesni, avx512, "AES-NI vs AVX-512: len={len} seed={seed:#x}");
                assert_eq!(
                    avx512, portable,
                    "AVX-512 vs Portable: len={len} seed={seed:#x}"
                );
            }
        } else {
            // Fallback: auto-detected (seed=0) vs portable (seed=0).
            let auto_hash = hash(&input);
            let portable = kernels::portable::oneshot(&input, 0, 0, None);
            assert_eq!(auto_hash, portable, "Auto vs Portable: len={len}");
        }
    }
}
