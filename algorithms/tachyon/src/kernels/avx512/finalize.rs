//! AVX-512 Finalization
//!
//! Implements finalization for the 32-track model with:
//! - Constant-time remainder processing (no data-dependent branches)
//! - CLMUL hardening with independent constant
//! - Multi-round key absorption

#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_wrap)]

use super::state::Avx512State;
use crate::kernels::constants::{
    BLOCK_SIZE, C5, C6, C7, CHAOS_BASE, CLMUL_CONSTANT, CLMUL_CONSTANT2, GOLDEN_RATIO,
    LANE_OFFSETS, REMAINDER_CHUNK_SIZE, RK_CHAIN, ROUNDS, VEC_SIZE, WHITENING0, WHITENING1,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::{
    _mm512_add_epi64, _mm512_aesenc_epi128, _mm512_alignr_epi64, _mm512_castsi512_si128,
    _mm512_clmulepi64_epi128, _mm512_loadu_si512, _mm512_set1_epi64, _mm512_set_epi64,
    _mm512_shuffle_i32x4, _mm512_ternarylogic_epi64, _mm512_xor_si512, _mm_storeu_si128,
};

// =============================================================================
// IMPLEMENTATION
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Avx512State {
    /// Finalize hash with remainder and produce 256-bit output.
    // SAFETY: Requires AVX-512F/BW/VAES/VPCLMULQDQ (enforced by dispatcher).
    // Constant-time processing, safe `copy_nonoverlapping` (no overlap, bounds checked).
    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "avx512bw")]
    #[target_feature(enable = "vpclmulqdq")]
    #[target_feature(enable = "vaes")]
    #[allow(unsafe_code)]
    pub unsafe fn finalize(
        self,
        remainder: &[u8],
        total_length: u64,
        domain_id: u64,
        key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
    ) -> [u8; crate::kernels::constants::HASH_SIZE] {
        let mut acc = self.acc;

        // Load precomputed round key schedule with asymmetric lo/hi pattern
        let rk_chain: [_; ROUNDS] = core::array::from_fn(|r| {
            _mm512_set_epi64(
                RK_CHAIN[r].1 as i64,
                RK_CHAIN[r].0 as i64,
                RK_CHAIN[r].1 as i64,
                RK_CHAIN[r].0 as i64,
                RK_CHAIN[r].1 as i64,
                RK_CHAIN[r].0 as i64,
                RK_CHAIN[r].1 as i64,
                RK_CHAIN[r].0 as i64,
            )
        });

        let wk = _mm512_set_epi64(
            WHITENING1 as i64,
            WHITENING0 as i64,
            WHITENING1 as i64,
            WHITENING0 as i64,
            WHITENING1 as i64,
            WHITENING0 as i64,
            WHITENING1 as i64,
            WHITENING0 as i64,
        );

        // 1. CONSTANT-TIME REMAINDER PROCESSING
        // Process each remainder block. Input length is public.
        let mut chunks = remainder.chunks_exact(REMAINDER_CHUNK_SIZE);

        for (i, c) in chunks.by_ref().take(8).enumerate() {
            let mut d = _mm512_aesenc_epi128(_mm512_loadu_si512(c.as_ptr().cast()), wk);
            let base = i * 4;
            let lo = _mm512_set_epi64(
                LANE_OFFSETS[base + 3] as i64,
                LANE_OFFSETS[base + 3] as i64,
                LANE_OFFSETS[base + 2] as i64,
                LANE_OFFSETS[base + 2] as i64,
                LANE_OFFSETS[base + 1] as i64,
                LANE_OFFSETS[base + 1] as i64,
                LANE_OFFSETS[base] as i64,
                LANE_OFFSETS[base] as i64,
            );
            let save = acc[i];

            for &rk in &rk_chain {
                acc[i] =
                    _mm512_aesenc_epi128(acc[i], _mm512_add_epi64(d, _mm512_add_epi64(rk, lo)));
                // Cross-lane rotation for intra-register diffusion
                acc[i] = _mm512_alignr_epi64(acc[i], acc[i], 2);
                // State-Feedback: self-feedback (single track)
                d = _mm512_xor_si512(d, acc[i]);
            }

            acc[i] = _mm512_xor_si512(acc[i], save);
        }

        // 2. FINAL PADDING BLOCK
        let rem = chunks.remainder();
        let mut block = [0u8; REMAINDER_CHUNK_SIZE];
        if !rem.is_empty() {
            core::ptr::copy_nonoverlapping(rem.as_ptr(), block.as_mut_ptr(), rem.len());
        }
        block[rem.len()] = 0x80;
        let mut d0 = _mm512_aesenc_epi128(_mm512_loadu_si512(block.as_ptr().cast()), wk);

        // 3. TREE MERGE (8 -> 4 -> 2 -> 1)
        // Non-linear reduction using independent constants.
        let merge_rk0 = _mm512_set1_epi64(C5 as i64); // ln(11)
        let merge_rk1 = _mm512_set1_epi64(C6 as i64); // ln(13)
        let merge_rk2 = _mm512_set1_epi64(C7 as i64); // ln(17)
                                                      // Level 0: 8 -> 4
        for i in 0..4 {
            acc[i] = _mm512_aesenc_epi128(acc[i], _mm512_xor_si512(acc[i + 4], merge_rk0));
            acc[i] = _mm512_aesenc_epi128(acc[i], _mm512_xor_si512(acc[i], merge_rk0));
            // self-mix
        }
        // Level 1: 4 -> 2
        acc[0] = _mm512_aesenc_epi128(acc[0], _mm512_xor_si512(acc[2], merge_rk1));
        acc[0] = _mm512_aesenc_epi128(acc[0], _mm512_xor_si512(acc[0], merge_rk1)); // self-mix
        acc[1] = _mm512_aesenc_epi128(acc[1], _mm512_xor_si512(acc[3], merge_rk1));
        acc[1] = _mm512_aesenc_epi128(acc[1], _mm512_xor_si512(acc[1], merge_rk1)); // self-mix
                                                                                    // Level 2: 2 -> 1
        acc[0] = _mm512_aesenc_epi128(acc[0], _mm512_xor_si512(acc[1], merge_rk2));
        acc[0] = _mm512_aesenc_epi128(acc[0], _mm512_xor_si512(acc[0], merge_rk2)); // self-mix

        // 4. QUADRATIC CLMUL HARDENING
        // Round 1: polynomial mixing in GF(2)[x]
        // Use different polynomials for low/high halves to avoid algebraic dependencies.
        let clmul_k = _mm512_set_epi64(
            CLMUL_CONSTANT2 as i64,
            CLMUL_CONSTANT as i64, // Lane 3
            CLMUL_CONSTANT2 as i64,
            CLMUL_CONSTANT as i64, // Lane 2
            CLMUL_CONSTANT2 as i64,
            CLMUL_CONSTANT as i64, // Lane 1
            CLMUL_CONSTANT2 as i64,
            CLMUL_CONSTANT as i64, // Lane 0
        );
        let cl1 = _mm512_xor_si512(
            _mm512_clmulepi64_epi128(acc[0], clmul_k, 0x00),
            _mm512_clmulepi64_epi128(acc[0], clmul_k, 0x11),
        );
        // AES barrier: polynomial product as round key (degree ~254)
        let mid = _mm512_aesenc_epi128(acc[0], cl1);
        // Round 2: self-multiply lo×hi → quadratic in GF(2)[x] (degree ~254²)
        let cl2 = _mm512_clmulepi64_epi128(mid, mid, 0x01);
        // Nonlinear fold: AESENC eliminates linear shortcut back to original state
        acc[0] = _mm512_aesenc_epi128(acc[0], _mm512_xor_si512(cl1, cl2));

        let save0 = acc[0];

        // 5. FINAL BLOCK PROCESSING (Length/Domain Injection)
        let meta_vec = _mm512_set_epi64(
            CHAOS_BASE as i64,
            domain_id as i64,
            total_length as i64,
            CHAOS_BASE as i64,
            domain_id as i64,
            total_length as i64,
            CHAOS_BASE as i64,
            (domain_id ^ total_length) as i64,
        );

        acc[0] = _mm512_ternarylogic_epi64(acc[0], d0, meta_vec, 0x96);

        for (r, &rk) in rk_chain.iter().enumerate() {
            acc[0] = _mm512_aesenc_epi128(acc[0], _mm512_add_epi64(d0, rk));
            // Cross-lane rotation for intra-register diffusion
            acc[0] = _mm512_alignr_epi64(acc[0], acc[0], 2);
            // State-Feedback every 2 rounds: self-feedback on odd rounds
            if r % 2 == 1 {
                d0 = _mm512_xor_si512(d0, acc[0]);
            }
        }

        acc[0] = _mm512_xor_si512(acc[0], save0);

        // 6. MULTI-ROUND KEY ABSORPTION
        // 4 distinct permutations ensure full key diffusion.
        if let Some(k) = key {
            let mut key_block = [0u8; REMAINDER_CHUNK_SIZE];
            key_block[0..32].copy_from_slice(k);
            key_block[32..64].copy_from_slice(k);
            let k0 = _mm512_loadu_si512(key_block.as_ptr().cast());
            // Break key duplication: XOR upper 256 bits with GOLDEN_RATIO
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
            let k0 = _mm512_xor_si512(k0, gr_mask);
            // Round 1: cross (0x14)
            let k_r1 = _mm512_shuffle_i32x4(k0, k0, 0x14);
            acc[0] = _mm512_aesenc_epi128(acc[0], k_r1);
            // Round 2: inverted cross (0x41)
            let k_r2 = _mm512_shuffle_i32x4(k0, k0, 0x41);
            acc[0] = _mm512_aesenc_epi128(acc[0], k_r2);
            // Round 3: direct (k0, k1, k0, k1)
            let k_r3 = _mm512_shuffle_i32x4(k0, k0, 0x44);
            acc[0] = _mm512_aesenc_epi128(acc[0], k_r3);
            // Round 4: halved (0x50)
            let k_r4 = _mm512_shuffle_i32x4(k0, k0, 0x50);
            acc[0] = _mm512_aesenc_epi128(acc[0], k_r4);
        }

        // 7. FINAL LANE REDUCTION (512-bit -> 256-bit)
        // Pure AESENC cross-lane mixing for guaranteed full diffusion.

        // Round 1: Self-mix
        let mix_r1 = _mm512_aesenc_epi128(acc[0], acc[0]);

        // Round 2: Cross-half mix (lane0↔lane2, lane1↔lane3)
        let mix_r1_swap = _mm512_shuffle_i32x4(mix_r1, mix_r1, 0x4E); // swap halves [2,3,0,1]
        let mix_r2 = _mm512_aesenc_epi128(mix_r1, mix_r1_swap);

        // Round 3: Adjacent-pair mix (shuffle_i32x4 0xB1 = [1,0,3,2])
        // Asymmetry break: independent constants per lane (Lane 0 = reference, 1-3 get C7/C6/C5)
        let mix_r2_swap = _mm512_shuffle_i32x4(mix_r2, mix_r2, 0xB1);
        let asymmetry_mask = _mm512_set_epi64(
            C5 as i64, C5 as i64, // Lane 3
            C6 as i64, C6 as i64, // Lane 2
            C7 as i64, C7 as i64, // Lane 1
            0, 0, // Lane 0 (reference)
        );
        let mix_r2_swap_masked = _mm512_xor_si512(mix_r2_swap, asymmetry_mask);
        let mix_r3 = _mm512_aesenc_epi128(mix_r2, mix_r2_swap_masked);

        // Round 4: Cross-half fold for 512→256 reduction
        let mix_r3_hi = _mm512_shuffle_i32x4(mix_r3, mix_r3, 0x4E);
        let mix_r4 = _mm512_aesenc_epi128(mix_r3, mix_r3_hi);

        // Round 5: Adjacent-pair final mix for full 32-bit cross-diffusion
        let mix_r4_swap = _mm512_shuffle_i32x4(mix_r4, mix_r4, 0xB1); // [1,0,3,2]
        let mix_r4_swap_masked = _mm512_xor_si512(mix_r4_swap, asymmetry_mask);
        let mix_r5 = _mm512_aesenc_epi128(mix_r4, mix_r4_swap_masked);

        // Extract lower 256 bits (lanes 0 and 1)
        let mut res = [0u8; crate::kernels::constants::HASH_SIZE];
        _mm_storeu_si128(res.as_mut_ptr().cast(), _mm512_castsi512_si128(mix_r5));
        let mix_r5_lane1 = _mm512_shuffle_i32x4(mix_r5, mix_r5, 0x01); // bring lane1 to lane0
        _mm_storeu_si128(
            res.as_mut_ptr().add(VEC_SIZE).cast(),
            _mm512_castsi512_si128(mix_r5_lane1),
        );
        res
    }
}

// =============================================================================
// ONE-SHOT API
// =============================================================================

/// One-shot hashing for AVX-512.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
// SAFETY: Requires AVX-512F/BW/VAES/VPCLMULQDQ. Short path (< 64B) uses AES-NI instead.
// Delegates to documented helpers: `aesni::short::oneshot_short`, `new_with_seed`, `finalize`.
#[target_feature(enable = "avx512f")]
#[target_feature(enable = "avx512bw")]
#[target_feature(enable = "vpclmulqdq")]
#[target_feature(enable = "vaes")]
#[allow(unsafe_code)]
pub unsafe fn oneshot(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    // Short path guard BEFORE state init (avoid wasted work)
    if input.len() < REMAINDER_CHUNK_SIZE {
        return crate::kernels::aesni::short::oneshot_short(input, domain, seed, key);
    }

    let mut state = Avx512State::new_with_seed(key, seed);

    let chunks_len = input.len() / BLOCK_SIZE * BLOCK_SIZE;
    state.update(&input[..chunks_len]);
    state.finalize(&input[chunks_len..], input.len() as u64, domain, key)
}
