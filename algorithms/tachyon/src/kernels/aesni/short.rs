//! AES-NI Short Path
//!
//! Unified kernel for inputs < 64 bytes. Algorithmically identical to
//! `AesNiState::new() + finalize()` (the large path), but avoids the
//! full compress pipeline. Used by ALL dispatchers (hybrid, avx512, aesni).
//!
//! **Optimization:** For `seed=0, key=None`, the 4 post-merge registers are loaded
//! from precomputed constants, skipping initialization overhead.

#![allow(clippy::cast_possible_wrap)]

use super::state::AesNiState;
use crate::kernels::constants::{
    C7, CHAOS_BASE, LANE_OFFSETS, REMAINDER_CHUNK_SIZE, RK_CHAIN, ROUNDS, SHORT_INIT, VEC_SIZE,
    WHITENING0, WHITENING1,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::{
    __m128i, _mm_add_epi64, _mm_aesenc_si128, _mm_loadu_si128, _mm_set1_epi64x, _mm_set_epi64x,
    _mm_storeu_si128, _mm_xor_si128,
};

// =============================================================================
// ONE-SHOT API
// =============================================================================

/// One-shot hash for small inputs (< 64 bytes).
/// Optimization: Skips compress pipeline and uses precomputed state for common case (seed=0).
// SAFETY: Requires AES/SSE2/PCLMULQDQ CPU features (enforced by dispatcher).
// Fast path uses precomputed constants, fallback delegates to `AesNiState::new` + `finalize`.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "aes")]
#[target_feature(enable = "sse2")]
#[target_feature(enable = "pclmulqdq")]
#[allow(unsafe_code)]
#[allow(clippy::cast_possible_wrap)]
pub unsafe fn oneshot_short(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    // Fast path: precomputed state for seed=0, key=None
    if seed == 0 && key.is_none() {
        return finalize_short_precomputed(input, input.len() as u64, domain);
    }

    // Fallback: full initialization for non-default seed/key
    let state = AesNiState::new(key, seed);
    state.finalize(input, input.len() as u64, domain, key)
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/// Finalize using precomputed post-merge state (seed=0, key=None).
///
/// Exactly mirrors `AesNiState::finalize()` from step 4 onwards,
/// but loads 4 precomputed registers instead of running init + tree merge + CLMUL.
// SAFETY: Requires AES/SSE2/PCLMULQDQ. Uses precomputed SHORT_INIT constants.
// Safe `copy_nonoverlapping` (no overlap, max 63 bytes into 64-byte buffer).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "aes")]
#[target_feature(enable = "sse2")]
#[target_feature(enable = "pclmulqdq")]
#[allow(unsafe_code)]
unsafe fn finalize_short_precomputed(
    input: &[u8],
    total_length: u64,
    domain_id: u64,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    use crate::kernels::constants::{C5, C6};
    // Load precomputed round keys
    let rk_chain: [__m128i; ROUNDS] =
        core::array::from_fn(|r| _mm_set_epi64x(RK_CHAIN[r].1 as i64, RK_CHAIN[r].0 as i64));

    let wk = _mm_set_epi64x(WHITENING1 as i64, WHITENING0 as i64);

    // Load precomputed post-merge state
    let mut acc: [__m128i; 4] = [
        _mm_set_epi64x(SHORT_INIT[0].1 as i64, SHORT_INIT[0].0 as i64),
        _mm_set_epi64x(SHORT_INIT[1].1 as i64, SHORT_INIT[1].0 as i64),
        _mm_set_epi64x(SHORT_INIT[2].1 as i64, SHORT_INIT[2].0 as i64),
        _mm_set_epi64x(SHORT_INIT[3].1 as i64, SHORT_INIT[3].0 as i64),
    ];

    // 2. Prepare Final Padding Block (all input fits in one block)
    let mut block = [0u8; REMAINDER_CHUNK_SIZE];
    if !input.is_empty() {
        core::ptr::copy_nonoverlapping(input.as_ptr(), block.as_mut_ptr(), input.len());
    }
    block[input.len()] = 0x80;
    let mut d0 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().cast()), wk);
    let mut d1 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().add(VEC_SIZE).cast()), wk);
    let mut d2 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().add(VEC_SIZE * 2).cast()), wk);
    let mut d3 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().add(VEC_SIZE * 3).cast()), wk);

    // 3. Tree Merge + CLMUL: skipped — precomputed state already encodes the post-merge result
    let saves_final = [acc[0], acc[1], acc[2], acc[3]]; // saved for Davies-Meyer feed-forward

    // 4. Final Block Processing - Positional domain/length injection
    let meta0 = _mm_set_epi64x(
        CHAOS_BASE as i64,                 // e1
        (domain_id ^ total_length) as i64, // e0
    );
    let meta1 = _mm_set_epi64x(
        domain_id as i64,    // e3
        total_length as i64, // e2
    );
    let meta2 = _mm_set_epi64x(
        total_length as i64, // e5
        CHAOS_BASE as i64,   // e4
    );
    let meta3 = _mm_set_epi64x(
        CHAOS_BASE as i64, // e7
        domain_id as i64,  // e6
    );

    // Mix data + metadata into accumulators
    acc[0] = _mm_xor_si128(acc[0], _mm_xor_si128(d0, meta0));
    acc[1] = _mm_xor_si128(acc[1], _mm_xor_si128(d1, meta1));
    acc[2] = _mm_xor_si128(acc[2], _mm_xor_si128(d2, meta2));
    acc[3] = _mm_xor_si128(acc[3], _mm_xor_si128(d3, meta3));

    // Pre-load lane offsets for sub-lanes 0-3 to avoid redundant broadcasts in the loop
    let lo0 = _mm_set1_epi64x(LANE_OFFSETS[0] as i64);
    let lo1 = _mm_set1_epi64x(LANE_OFFSETS[1] as i64);
    let lo2 = _mm_set1_epi64x(LANE_OFFSETS[2] as i64);
    let lo3 = _mm_set1_epi64x(LANE_OFFSETS[3] as i64);

    // AESENC rounds with state-feedback every 2 rounds
    // Feedback on odd rounds (1,3,5,7,9): breaks attacker key control after round 1,
    // while halving latency overhead vs every-round feedback (~15 cy instead of ~30).
    for (r, &rk) in rk_chain.iter().enumerate().take(ROUNDS) {
        acc[0] = _mm_aesenc_si128(acc[0], _mm_add_epi64(d0, _mm_add_epi64(rk, lo0)));
        acc[1] = _mm_aesenc_si128(acc[1], _mm_add_epi64(d1, _mm_add_epi64(rk, lo1)));
        acc[2] = _mm_aesenc_si128(acc[2], _mm_add_epi64(d2, _mm_add_epi64(rk, lo2)));
        acc[3] = _mm_aesenc_si128(acc[3], _mm_add_epi64(d3, _mm_add_epi64(rk, lo3)));

        // State-Feedback every 2 rounds: stride-1 cross-infect on odd rounds
        if r % 2 == 1 {
            let (t0, t1, t2, t3) = (acc[0], acc[1], acc[2], acc[3]);
            d0 = _mm_xor_si128(d0, t1);
            d1 = _mm_xor_si128(d1, t2);
            d2 = _mm_xor_si128(d2, t3);
            d3 = _mm_xor_si128(d3, t0);
        }
        // Cyclic lane rotation
        let tmp = acc[0];
        acc[0] = acc[1];
        acc[1] = acc[2];
        acc[2] = acc[3];
        acc[3] = tmp;
    }

    // Davies-Meyer feed-forward with saved post-merge state
    for i in 0..4 {
        acc[i] = _mm_xor_si128(acc[i], saves_final[i]);
    }

    // Round 1: Self-mix for non-linear amplification
    let a0 = _mm_aesenc_si128(acc[0], acc[0]);
    let a1 = _mm_aesenc_si128(acc[1], acc[1]);
    let a2 = _mm_aesenc_si128(acc[2], acc[2]);
    let a3 = _mm_aesenc_si128(acc[3], acc[3]);

    // Round 2: Cross-half mix (shuffle_i32x4 0x4E = [2,3,0,1])
    let b0 = _mm_aesenc_si128(a0, a2);
    let b1 = _mm_aesenc_si128(a1, a3);
    let b2 = _mm_aesenc_si128(a2, a0);
    let b3 = _mm_aesenc_si128(a3, a1);

    // Round 3: Adjacent-pair mix (shuffle_i32x4 0xB1 = [1,0,3,2])
    // Asymmetry break: independent constants per lane to match AVX-512 asymmetry_mask
    // Lane 0 (c0): no mask; Lane 1 (c1): C7; Lane 2 (c2): C6; Lane 3 (c3): C5

    let merge_rk0 = _mm_set1_epi64x(C5 as i64);
    let merge_rk1 = _mm_set1_epi64x(C6 as i64);
    let merge_rk2 = _mm_set1_epi64x(C7 as i64);
    let c0 = _mm_aesenc_si128(b0, b1);
    let c1 = _mm_aesenc_si128(b1, _mm_xor_si128(b0, merge_rk2));
    let c2 = _mm_aesenc_si128(b2, _mm_xor_si128(b3, merge_rk1));
    let c3 = _mm_aesenc_si128(b3, _mm_xor_si128(b2, merge_rk0));

    // Round 4: Cross-half fold (shuffle_i32x4 0x4E = [2,3,0,1])
    let d0 = _mm_aesenc_si128(c0, c2);
    let d1 = _mm_aesenc_si128(c1, c3);

    // Round 5: Cross-half final mix for full 32-bit diffusion
    let e0 = _mm_aesenc_si128(d0, d1);
    let e1 = _mm_aesenc_si128(d1, _mm_xor_si128(d0, merge_rk2));

    // Output: lanes 0 and 1 (matching AVX-512 extracting lower 256 bits)
    let mut res = [0u8; crate::kernels::constants::HASH_SIZE];
    _mm_storeu_si128(res.as_mut_ptr().cast(), e0);
    _mm_storeu_si128(res.as_mut_ptr().add(VEC_SIZE).cast(), e1);
    res
}
