//! AVX-512 State Management
//!
//! Defines `Avx512State` struct and initialization.

#![allow(clippy::cast_possible_wrap)]

use crate::kernels::constants::{C0, C1, C2, C3, C4, C5, C6, C7, GOLDEN_RATIO, LANE_OFFSETS};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::{
    __m512i, _mm512_add_epi64, _mm512_aesenc_epi128, _mm512_loadu_si512, _mm512_set1_epi64,
    _mm512_set_epi64, _mm512_xor_si512,
};

// =============================================================================
// AVX-512 STATE
// =============================================================================

/// AVX-512 State (8 x 512-bit ZMM registers).
#[derive(Clone, Copy, Debug)]
#[repr(align(64))]
pub struct Avx512State {
    pub(crate) acc: [__m512i; 8],
    pub(crate) block_count: u64, // Position binding
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Avx512State {
    /// Initialize new state with optional key (seed=0).
    /// Safe wrapper for `new_with_seed`.
    // SAFETY: Requires AVX-512F/BW/VAES CPU features (enforced by dispatcher).
    // All operations are register-based SIMD with no undefined behavior.
    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "avx512bw")]
    #[target_feature(enable = "vaes")]
    #[allow(unsafe_code)]
    pub unsafe fn new(key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>) -> Self {
        Self::new_with_seed(key, 0)
    }

    /// Initialize new state with optional key and seed.
    // SAFETY: Requires AVX-512F/BW/VAES CPU features (enforced by dispatcher).
    // All operations are register-based SIMD. Key validated by type system.
    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "avx512bw")]
    #[target_feature(enable = "vaes")]
    #[allow(unsafe_code)]
    pub unsafe fn new_with_seed(
        key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
        seed: u64,
    ) -> Self {
        // Init register with distinct elements to break symmetry
        // SAFETY: AVX-512F guaranteed by `new_with_seed` caller. Compile-time constants only.
        unsafe fn init_reg(base: u64) -> __m512i {
            _mm512_set_epi64(
                base.wrapping_add(7) as i64,
                base.wrapping_add(6) as i64,
                base.wrapping_add(5) as i64,
                base.wrapping_add(4) as i64,
                base.wrapping_add(3) as i64,
                base.wrapping_add(2) as i64,
                base.wrapping_add(1) as i64,
                base as i64,
            )
        }

        let mut acc = [
            init_reg(C0),
            init_reg(C1),
            init_reg(C2),
            init_reg(C3),
            init_reg(C4),
            init_reg(C5),
            init_reg(C6),
            init_reg(C7),
        ];

        // 1. Non-linear Seed Mixing
        // seed=0 uses C5 to avoid timing leaks
        let s_vec = if seed != 0 {
            _mm512_set1_epi64(seed as i64)
        } else {
            _mm512_set1_epi64(C5 as i64)
        };
        for a in &mut acc {
            *a = _mm512_aesenc_epi128(*a, s_vec);
        }

        // 2. Key Absorption (2 Rounds)
        if let Some(k) = key {
            let mut key_block = [0u8; 64];
            key_block[0..32].copy_from_slice(k);
            key_block[32..64].copy_from_slice(k);

            let k_vec = _mm512_loadu_si512(key_block.as_ptr().cast());
            // XOR upper 256 bits with GOLDEN_RATIO to break key duplication
            let gr_mask = _mm512_set_epi64(
                GOLDEN_RATIO as i64,
                GOLDEN_RATIO as i64,
                GOLDEN_RATIO as i64,
                GOLDEN_RATIO as i64,
                0,
                0,
                0,
                0,
            );
            let k_vec = _mm512_xor_si512(k_vec, gr_mask);

            for (i, a) in acc.iter_mut().enumerate() {
                // Per-accumulator differentiation via lane offset
                let lane_k = _mm512_add_epi64(k_vec, _mm512_set1_epi64(LANE_OFFSETS[i] as i64));
                *a = _mm512_aesenc_epi128(*a, lane_k);
                *a = _mm512_aesenc_epi128(*a, k_vec);
            }
        }

        Self {
            acc,
            block_count: 0,
        }
    }
}
