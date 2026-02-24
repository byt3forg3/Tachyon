//! AES-NI State Management
//!
//! Defines `AesNiState` struct and initialization for incremental hashing.
//! Uses 128-bit registers (4-Way ILP).

#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]

use crate::kernels::constants::{C0, C1, C2, C3, C4, C5, C6, C7, GOLDEN_RATIO, LANE_OFFSETS};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::{
    __m128i, _mm_add_epi64, _mm_aesenc_si128, _mm_loadu_si128, _mm_set1_epi64x, _mm_set_epi64x,
    _mm_xor_si128,
};

// =============================================================================
// AES-NI STATE
// =============================================================================

/// AES-NI State (32 x 128-bit registers).
/// 4-way split of the 8 x 512-bit AVX state.
#[derive(Clone, Copy, Debug)]
#[repr(align(16))]
pub struct AesNiState {
    pub(crate) acc: [__m128i; 32],
    pub(crate) block_count: u64, // Injected per block so identical data at different positions can't produce the same state
}

// =============================================================================
// IMPLEMENTATION
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl AesNiState {
    /// Initialize new state with optional key and seed.
    // SAFETY: Requires AES/SSE2/PCLMULQDQ CPU features (enforced by dispatcher).
    // Initializes 32 x 128-bit registers to emulate AVX-512 behavior. Key validated by type system.
    #[target_feature(enable = "aes")]
    #[target_feature(enable = "sse2")]
    #[target_feature(enable = "pclmulqdq")]
    #[allow(unsafe_code)]
    pub unsafe fn new(key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>, seed: u64) -> Self {
        #[inline]
        // SAFETY: SSE2 guaranteed by `new` caller. Compile-time constants only.
        unsafe fn init_xmm(base: u64, offset: u64) -> __m128i {
            _mm_set_epi64x(
                base.wrapping_add(offset + 1) as i64,
                base.wrapping_add(offset) as i64,
            )
        }

        let mut acc = [
            // Matches Avx512State::acc[0]
            init_xmm(C0, 0),
            init_xmm(C0, 2),
            init_xmm(C0, 4),
            init_xmm(C0, 6),
            // Matches Avx512State::acc[1]
            init_xmm(C1, 0),
            init_xmm(C1, 2),
            init_xmm(C1, 4),
            init_xmm(C1, 6),
            // Matches Avx512State::acc[2]
            init_xmm(C2, 0),
            init_xmm(C2, 2),
            init_xmm(C2, 4),
            init_xmm(C2, 6),
            // Matches Avx512State::acc[3]
            init_xmm(C3, 0),
            init_xmm(C3, 2),
            init_xmm(C3, 4),
            init_xmm(C3, 6),
            // Matches Avx512State::acc[4]
            init_xmm(C4, 0),
            init_xmm(C4, 2),
            init_xmm(C4, 4),
            init_xmm(C4, 6),
            // Matches Avx512State::acc[5]
            init_xmm(C5, 0),
            init_xmm(C5, 2),
            init_xmm(C5, 4),
            init_xmm(C5, 6),
            // Matches Avx512State::acc[6]
            init_xmm(C6, 0),
            init_xmm(C6, 2),
            init_xmm(C6, 4),
            init_xmm(C6, 6),
            // Matches Avx512State::acc[7]
            init_xmm(C7, 0),
            init_xmm(C7, 2),
            init_xmm(C7, 4),
            init_xmm(C7, 6),
        ];

        // 1. Non-linear Seed Mixing
        // seed=0 uses C5 to avoid timing leaks if we skipped mixing
        let seed_vec = if seed != 0 {
            _mm_set1_epi64x(seed as i64)
        } else {
            _mm_set1_epi64x(C5 as i64)
        };
        for a in &mut acc {
            *a = _mm_aesenc_si128(*a, seed_vec);
        }

        // 2. Key Absorption (2 Rounds)
        if let Some(k) = key {
            let mut key_block = [0u8; 64];
            key_block[0..32].copy_from_slice(k);
            key_block[32..64].copy_from_slice(k);

            let k0 = _mm_loadu_si128(key_block.as_ptr().cast());
            let k1 = _mm_loadu_si128(key_block.as_ptr().add(16).cast());
            // Derive k2/k3 via GOLDEN_RATIO XOR to break key duplication
            let gr = _mm_set1_epi64x(GOLDEN_RATIO as i64);
            let k2 = _mm_xor_si128(k0, gr);
            let k3 = _mm_xor_si128(k1, gr);

            // Per-accumulator lane offset differentiation + 2 AESENC rounds
            for i in 0..8 {
                let lo = _mm_set1_epi64x(LANE_OFFSETS[i] as i64);
                // Round 1: key + lane offset
                acc[i * 4] = _mm_aesenc_si128(acc[i * 4], _mm_add_epi64(k0, lo));
                acc[i * 4 + 1] = _mm_aesenc_si128(acc[i * 4 + 1], _mm_add_epi64(k1, lo));
                acc[i * 4 + 2] = _mm_aesenc_si128(acc[i * 4 + 2], _mm_add_epi64(k2, lo));
                acc[i * 4 + 3] = _mm_aesenc_si128(acc[i * 4 + 3], _mm_add_epi64(k3, lo));
                // Round 2: raw key
                acc[i * 4] = _mm_aesenc_si128(acc[i * 4], k0);
                acc[i * 4 + 1] = _mm_aesenc_si128(acc[i * 4 + 1], k1);
                acc[i * 4 + 2] = _mm_aesenc_si128(acc[i * 4 + 2], k2);
                acc[i * 4 + 3] = _mm_aesenc_si128(acc[i * 4 + 3], k3);
            }
        }

        Self {
            acc,
            block_count: 0,
        }
    }
}
