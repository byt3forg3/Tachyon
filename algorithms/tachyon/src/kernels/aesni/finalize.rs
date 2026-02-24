//! AES-NI Finalization
//!
//! Implements finalization for the 32-track model.
//! Features constant-time remainder processing and CLMUL hardening.

#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]

use super::state::AesNiState;
use crate::kernels::constants::{
    BLOCK_SIZE, C5, C6, C7, CHAOS_BASE, CLMUL_CONSTANT, CLMUL_CONSTANT2, LANE_OFFSETS,
    REMAINDER_CHUNK_SIZE, RK_CHAIN, ROUNDS, VEC_SIZE, WHITENING0, WHITENING1,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::{
    __m128i, _mm_add_epi64, _mm_aesenc_si128, _mm_clmulepi64_si128, _mm_loadu_si128,
    _mm_set1_epi64x, _mm_set_epi64x, _mm_storeu_si128, _mm_xor_si128,
};

// =============================================================================
// IMPLEMENTATION
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl AesNiState {
    /// Finalize hash with remainder and produce 256-bit output.
    // SAFETY: Requires AES/SSE2/PCLMULQDQ CPU features (enforced by dispatcher).
    // Constant-time processing, safe `copy_nonoverlapping` (no overlap, bounds checked).
    #[target_feature(enable = "aes")]
    #[target_feature(enable = "sse2")]
    #[target_feature(enable = "pclmulqdq")]
    #[allow(unsafe_code)]
    pub unsafe fn finalize(
        self,
        remainder: &[u8],
        total_length: u64,
        domain_id: u64,
        key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
    ) -> [u8; crate::kernels::constants::HASH_SIZE] {
        let mut acc = self.acc;

        // Load precomputed round keys
        let rk_chain: [__m128i; ROUNDS] =
            core::array::from_fn(|r| _mm_set_epi64x(RK_CHAIN[r].1 as i64, RK_CHAIN[r].0 as i64));

        let wk = _mm_set_epi64x(WHITENING1 as i64, WHITENING0 as i64);

        // 1. CONSTANT-TIME REMAINDER PROCESSING
        // Process 64-byte chunks. Input length is public, so access pattern is safe.
        let mut chunks = remainder.chunks_exact(REMAINDER_CHUNK_SIZE);
        for i in 0..8 {
            if let Some(c) = chunks.next() {
                let ptr = c.as_ptr();
                let mut d0 = _mm_aesenc_si128(_mm_loadu_si128(ptr.cast()), wk);
                let mut d1 = _mm_aesenc_si128(_mm_loadu_si128(ptr.add(VEC_SIZE).cast()), wk);
                let mut d2 = _mm_aesenc_si128(_mm_loadu_si128(ptr.add(VEC_SIZE * 2).cast()), wk);
                let mut d3 = _mm_aesenc_si128(_mm_loadu_si128(ptr.add(VEC_SIZE * 3).cast()), wk);
                let saves = [acc[i * 4], acc[i * 4 + 1], acc[i * 4 + 2], acc[i * 4 + 3]];

                let offset_base = i * 4;
                let lo0 = _mm_set1_epi64x(LANE_OFFSETS[offset_base] as i64);
                let lo1 = _mm_set1_epi64x(LANE_OFFSETS[offset_base + 1] as i64);
                let lo2 = _mm_set1_epi64x(LANE_OFFSETS[offset_base + 2] as i64);
                let lo3 = _mm_set1_epi64x(LANE_OFFSETS[offset_base + 3] as i64);

                for &rk_chain_r in rk_chain.iter().take(ROUNDS) {
                    acc[offset_base] = _mm_aesenc_si128(
                        acc[offset_base],
                        _mm_add_epi64(d0, _mm_add_epi64(rk_chain_r, lo0)),
                    );
                    acc[offset_base + 1] = _mm_aesenc_si128(
                        acc[offset_base + 1],
                        _mm_add_epi64(d1, _mm_add_epi64(rk_chain_r, lo1)),
                    );
                    acc[offset_base + 2] = _mm_aesenc_si128(
                        acc[offset_base + 2],
                        _mm_add_epi64(d2, _mm_add_epi64(rk_chain_r, lo2)),
                    );
                    acc[offset_base + 3] = _mm_aesenc_si128(
                        acc[offset_base + 3],
                        _mm_add_epi64(d3, _mm_add_epi64(rk_chain_r, lo3)),
                    );
                    // State-Feedback: self-feedback (single track, 4 lanes)
                    // Matches AVX-512 alignr(acc, 2) logic: d0^=acc[1], d1^=acc[2]...
                    d0 = _mm_xor_si128(d0, acc[i * 4 + 1]);
                    d1 = _mm_xor_si128(d1, acc[i * 4 + 2]);
                    d2 = _mm_xor_si128(d2, acc[i * 4 + 3]);
                    d3 = _mm_xor_si128(d3, acc[i * 4]);
                    // Cyclic lane rotation
                    let tmp = acc[i * 4];
                    acc[i * 4] = acc[i * 4 + 1];
                    acc[i * 4 + 1] = acc[i * 4 + 2];
                    acc[i * 4 + 2] = acc[i * 4 + 3];
                    acc[i * 4 + 3] = tmp;
                }

                acc[i * 4] = _mm_xor_si128(acc[i * 4], saves[0]);
                acc[i * 4 + 1] = _mm_xor_si128(acc[i * 4 + 1], saves[1]);
                acc[i * 4 + 2] = _mm_xor_si128(acc[i * 4 + 2], saves[2]);
                acc[i * 4 + 3] = _mm_xor_si128(acc[i * 4 + 3], saves[3]);
            }
        }

        // 2. FINAL PADDING BLOCK
        let rem = chunks.remainder();
        let mut block = [0u8; REMAINDER_CHUNK_SIZE];
        if !rem.is_empty() {
            core::ptr::copy_nonoverlapping(rem.as_ptr(), block.as_mut_ptr(), rem.len());
        }
        block[rem.len()] = 0x80;
        let mut d0 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().cast()), wk);
        let mut d1 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().add(VEC_SIZE).cast()), wk);
        let mut d2 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().add(VEC_SIZE * 2).cast()), wk);
        let mut d3 = _mm_aesenc_si128(_mm_loadu_si128(block.as_ptr().add(VEC_SIZE * 3).cast()), wk);

        // 3. TREE MERGE (32 -> 16 -> 8 -> 4)
        // Non-linear reduction using independent constants
        let merge_rk0 = _mm_set1_epi64x(C5 as i64); // ln(11)
        let merge_rk1 = _mm_set1_epi64x(C6 as i64); // ln(13)
        let merge_rk2 = _mm_set1_epi64x(C7 as i64); // ln(17)

        // Level 0: 32 -> 16
        for i in 0..16 {
            acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i + 16], merge_rk0));
            acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i], merge_rk0));
            // self-mix
        }
        // Level 1: 16 -> 8
        for i in 0..4 {
            acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i + 8], merge_rk1));
            acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i], merge_rk1)); // self-mix
            acc[i + 4] = _mm_aesenc_si128(acc[i + 4], _mm_xor_si128(acc[i + 12], merge_rk1));
            acc[i + 4] = _mm_aesenc_si128(acc[i + 4], _mm_xor_si128(acc[i + 4], merge_rk1));
            // self-mix
        }
        // Level 2: 8 -> 4
        for i in 0..4 {
            acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i + 4], merge_rk2));
            acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i], merge_rk2));
            // self-mix
        }

        // 4. QUADRATIC CLMUL HARDENING
        // Round 1: polynomial mixing in GF(2)[x]
        // Use different polynomials for low/high halves to avoid algebraic dependencies.
        let clmul_k = _mm_set_epi64x(CLMUL_CONSTANT2 as i64, CLMUL_CONSTANT as i64);
        for a in acc.iter_mut().take(4) {
            let cl1 = _mm_xor_si128(
                _mm_clmulepi64_si128(*a, clmul_k, 0x00),
                _mm_clmulepi64_si128(*a, clmul_k, 0x11),
            );
            // AES barrier: polynomial product as round key (degree ~254)
            let mid = _mm_aesenc_si128(*a, cl1);
            // Round 2: self-multiply lo×hi → quadratic in GF(2)[x] (degree ~254²)
            let cl2 = _mm_clmulepi64_si128(mid, mid, 0x01);
            // Nonlinear fold: AESENC eliminates linear shortcut back to original state
            *a = _mm_aesenc_si128(*a, _mm_xor_si128(cl1, cl2));
        }

        let saves_final = [acc[0], acc[1], acc[2], acc[3]];

        // 5. FINAL BLOCK PROCESSING (Length/Domain Injection)
        // Matches AVX-512 lane decomposition:
        // Lane 0 = (e0, e1), Lane 1 = (e2, e3), Lane 2 = (e4, e5), Lane 3 = (e6, e7)
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

        acc[0] = _mm_xor_si128(acc[0], _mm_xor_si128(d0, meta0));
        acc[1] = _mm_xor_si128(acc[1], _mm_xor_si128(d1, meta1));
        acc[2] = _mm_xor_si128(acc[2], _mm_xor_si128(d2, meta2));
        acc[3] = _mm_xor_si128(acc[3], _mm_xor_si128(d3, meta3));

        // AESENC rounds with state-feedback every 2 rounds
        // Feedback on odd rounds (1,3,5,7,9): breaks attacker key control after round 1,
        // while halving latency overhead vs every-round feedback (~15 cy instead of ~30).
        for (r, &rk) in rk_chain.iter().enumerate().take(ROUNDS) {
            for (i, a) in acc.iter_mut().enumerate().take(4) {
                let d = match i {
                    0 => d0,
                    1 => d1,
                    2 => d2,
                    _ => d3,
                };
                *a = _mm_aesenc_si128(*a, _mm_add_epi64(d, rk));
            }
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

        for i in 0..4 {
            acc[i] = _mm_xor_si128(acc[i], saves_final[i]);
        }

        // 4 distinct permutations ensure full key diffusion.
        if let Some(k) = key {
            let k0 = _mm_loadu_si128(k.as_ptr().cast());
            let k1 = _mm_loadu_si128(k.as_ptr().add(VEC_SIZE).cast());

            // Round 1: cross pattern (k0, k1, k1, k0)
            acc[0] = _mm_aesenc_si128(acc[0], k0);
            acc[1] = _mm_aesenc_si128(acc[1], k1);
            acc[2] = _mm_aesenc_si128(acc[2], k1);
            acc[3] = _mm_aesenc_si128(acc[3], k0);

            // Round 2: inverted cross (k1, k0, k0, k1)
            acc[0] = _mm_aesenc_si128(acc[0], k1);
            acc[1] = _mm_aesenc_si128(acc[1], k0);
            acc[2] = _mm_aesenc_si128(acc[2], k0);
            acc[3] = _mm_aesenc_si128(acc[3], k1);

            // Round 3: direct (k0, k1, k0, k1)
            acc[0] = _mm_aesenc_si128(acc[0], k0);
            acc[1] = _mm_aesenc_si128(acc[1], k1);
            acc[2] = _mm_aesenc_si128(acc[2], k0);
            acc[3] = _mm_aesenc_si128(acc[3], k1);

            // Round 4: halved (k0, k0, k1, k1)
            acc[0] = _mm_aesenc_si128(acc[0], k0);
            acc[1] = _mm_aesenc_si128(acc[1], k0);
            acc[2] = _mm_aesenc_si128(acc[2], k1);
            acc[3] = _mm_aesenc_si128(acc[3], k1);
        }

        // Round 1: Self-mix
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
        // Asymmetry break: apply independent constants per lane to match AVX-512 asymmetry_mask
        // AVX-512: {C5,C5, C6,C6, C7,C7, 0,0} applied to shuffled input before mix.
        // Lane 0 (c0): no mask (reference)
        // Lane 1 (c1): XOR with C7 = merge_rk2
        // Lane 2 (c2): XOR with C6 = merge_rk1
        // Lane 3 (c3): XOR with C5 = merge_rk0
        let c0 = _mm_aesenc_si128(b0, b1);
        let c1 = _mm_aesenc_si128(b1, _mm_xor_si128(b0, merge_rk2));
        let c2 = _mm_aesenc_si128(b2, _mm_xor_si128(b3, merge_rk1));
        let c3 = _mm_aesenc_si128(b3, _mm_xor_si128(b2, merge_rk0));

        // Round 4: Cross-half fold (shuffle_i32x4 0x4E = [2,3,0,1])
        let d0 = _mm_aesenc_si128(c0, c2);
        let d1 = _mm_aesenc_si128(c1, c3);

        // Round 5: Cross-half final mix for full 32-bit diffusion
        // Matches AVX-512: lane1 gets C7 asymmetry (merge_rk2), lane0 is reference.
        let e0 = _mm_aesenc_si128(d0, d1);
        let e1 = _mm_aesenc_si128(d1, _mm_xor_si128(d0, merge_rk2));

        // Output: lanes 0 and 1 (matching AVX-512 extracting lower 256 bits)
        let mut res = [0u8; crate::kernels::constants::HASH_SIZE];
        _mm_storeu_si128(res.as_mut_ptr().cast(), e0);
        _mm_storeu_si128(res.as_mut_ptr().add(VEC_SIZE).cast(), e1);
        res
    }
}

// =============================================================================
// ONE-SHOT API
// =============================================================================

/// One-shot hashing for AES-NI.
// SAFETY: Requires AES/SSE2/PCLMULQDQ CPU features (enforced by dispatcher).
// Delegates to documented safe helpers `AesNiState::new` and `finalize`.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "aes")]
#[target_feature(enable = "sse2")]
#[target_feature(enable = "pclmulqdq")]
#[allow(unsafe_code)]
pub unsafe fn oneshot(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    // Short path guard BEFORE state init (avoid wasted work)
    if input.len() < REMAINDER_CHUNK_SIZE {
        return super::short::oneshot_short(input, domain, seed, key);
    }

    let mut state = AesNiState::new(key, seed);

    let chunks_len = input.len() / BLOCK_SIZE * BLOCK_SIZE;
    state.update(&input[..chunks_len]);
    state.finalize(&input[chunks_len..], input.len() as u64, domain, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::constants::{C5, KEY_SCHEDULE_BASE, KEY_SCHEDULE_MULT};

    #[test]
    #[ignore = "utility"] // Utility: only run manually to regenerate SHORT_INIT constants
    #[allow(unsafe_code)]
    fn dump_precomputed_short_init() {
        unsafe {
            let state = AesNiState::new(None, 0);
            let mut acc = state.acc;

            // Tree Merge with level-differentiated round keys (matches finalize)
            let merge_rk0 = _mm_set1_epi64x(C5 as i64);
            let merge_rk1 = _mm_set1_epi64x(C6 as i64);
            let merge_rk2 = _mm_set1_epi64x(C7 as i64);
            for i in 0..16 {
                acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i + 16], merge_rk0));
                acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i], merge_rk0));
                // self-mix
            }
            for i in 0..4 {
                acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i + 8], merge_rk1));
                acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i], merge_rk1)); // self-mix
                acc[i + 4] = _mm_aesenc_si128(acc[i + 4], _mm_xor_si128(acc[i + 12], merge_rk1));
                acc[i + 4] = _mm_aesenc_si128(acc[i + 4], _mm_xor_si128(acc[i + 4], merge_rk1));
                // self-mix
            }
            for i in 0..4 {
                acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i + 4], merge_rk2));
                acc[i] = _mm_aesenc_si128(acc[i], _mm_xor_si128(acc[i], merge_rk2));
                // self-mix
            }

            // Quadratic CLMUL Hardening (matches finalize)
            let clmul_k = _mm_set_epi64x(CLMUL_CONSTANT2 as i64, CLMUL_CONSTANT as i64);
            for a in acc.iter_mut().take(4) {
                let cl1 = _mm_xor_si128(
                    _mm_clmulepi64_si128(*a, clmul_k, 0x00),
                    _mm_clmulepi64_si128(*a, clmul_k, 0x11),
                );
                let mid = _mm_aesenc_si128(*a, cl1);
                let cl2 = _mm_clmulepi64_si128(mid, mid, 0x01);
                *a = _mm_aesenc_si128(*a, _mm_xor_si128(cl1, cl2));
            }

            // Print as u64 pairs for hardcoding
            for (i, a) in acc.iter().enumerate().take(4) {
                let mut bytes = [0u8; 16];
                _mm_storeu_si128(bytes.as_mut_ptr().cast(), *a);
                let lo = u64::from_le_bytes(*bytes.as_ptr().cast::<[u8; 8]>());
                let hi = u64::from_le_bytes(*bytes.as_ptr().add(8).cast::<[u8; 8]>());
                eprintln!("SHORT_INIT[{i}]: lo=0x{lo:016X}, hi=0x{hi:016X}");
            }
        }
    }

    #[test]
    #[ignore = "utility"] // Utility: only run manually to regenerate RK_CHAIN constants
    #[allow(unsafe_code)]
    fn dump_precomputed_rk_chain() {
        unsafe {
            // Compute rk_chain with asymmetric lo/hi (Fix 1)
            let mut rk_chain = [_mm_set1_epi64x(0); 10]; // max ROUNDS
            rk_chain[0] = _mm_set_epi64x((KEY_SCHEDULE_BASE ^ C5) as i64, KEY_SCHEDULE_BASE as i64);
            for r in 1..10 {
                let div = _mm_set_epi64x(
                    (KEY_SCHEDULE_MULT
                        .wrapping_add(r as u64)
                        .wrapping_mul(KEY_SCHEDULE_BASE)
                        ^ C5) as i64,
                    KEY_SCHEDULE_MULT
                        .wrapping_add(r as u64)
                        .wrapping_mul(KEY_SCHEDULE_BASE) as i64,
                );
                rk_chain[r] = _mm_aesenc_si128(rk_chain[r - 1], div);
            }

            for (r, rk) in rk_chain.iter().enumerate() {
                let mut bytes = [0u8; 16];
                _mm_storeu_si128(bytes.as_mut_ptr().cast(), *rk);
                let lo = u64::from_le_bytes(*bytes.as_ptr().cast::<[u8; 8]>());
                let hi = u64::from_le_bytes(*bytes.as_ptr().add(8).cast::<[u8; 8]>());
                eprintln!("RK_CHAIN[{r}]: lo=0x{lo:016X}, hi=0x{hi:016X}");
            }
        }
    }
}
