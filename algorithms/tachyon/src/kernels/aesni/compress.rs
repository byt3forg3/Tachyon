//! AES-NI Block Compression
//!
//! Implements the update function for processing 1024-byte chunks (32-track model).

#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::identity_op)]
#![allow(clippy::erasing_op)]

use super::state::AesNiState;
use crate::kernels::constants::{
    BLOCK_SIZE, LANE_OFFSETS, RK_CHAIN, ROUNDS, VEC_SIZE, WHITENING0, WHITENING1,
};

use core::arch::x86_64::{
    __m128i, _mm_add_epi64, _mm_aesenc_si128, _mm_loadu_si128, _mm_set1_epi64x, _mm_set_epi64x,
    _mm_xor_si128,
};

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/// Process a single `BLOCK_SIZE` byte block through the compression function.
/// Uses 32 XMM registers to simulate 8 ZMM registers.
// SAFETY: Requires AES/SSE2 CPU features. Pointer arithmetic uses compile-time offsets
// on BLOCK_SIZE-validated input. Called only from `update` with valid slice-derived pointers.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "aes")]
#[target_feature(enable = "sse2")]
#[allow(unsafe_code)]
unsafe fn compress_block(acc: &mut [__m128i; 32], ptr: *const u8, block_idx: u64) {
    let mid = ROUNDS / 2;

    // Load precomputed round keys
    let rk_base: [__m128i; ROUNDS] =
        core::array::from_fn(|r| _mm_set_epi64x(RK_CHAIN[r].1 as i64, RK_CHAIN[r].0 as i64));

    // Per-accumulator lane offsets + block position
    let lo_all: [__m128i; 32] = core::array::from_fn(|i| _mm_set1_epi64x(LANE_OFFSETS[i] as i64));
    let blk = _mm_set1_epi64x(block_idx as i64);

    let wk = _mm_set_epi64x(WHITENING1 as i64, WHITENING0 as i64);

    let saves = *acc;

    // Pre-load 512B of data into mutable vars (for state-feedback evolution)
    let mut d: [[__m128i; 4]; 8] = core::array::from_fn(|i| {
        let off = i * 64;
        [
            _mm_aesenc_si128(_mm_loadu_si128(ptr.add(off).cast()), wk),
            _mm_aesenc_si128(_mm_loadu_si128(ptr.add(off + VEC_SIZE).cast()), wk),
            _mm_aesenc_si128(_mm_loadu_si128(ptr.add(off + VEC_SIZE * 2).cast()), wk),
            _mm_aesenc_si128(_mm_loadu_si128(ptr.add(off + VEC_SIZE * 3).cast()), wk),
        ]
    });

    // 1. First Half Rounds: Direct Mapping (d_i -> acc_i)
    for &rk in rk_base.iter().take(mid) {
        for ((acc_chunk, d_chunk), lo_chunk) in
            acc.chunks_exact_mut(4).zip(&d).zip(lo_all.chunks_exact(4))
        {
            acc_chunk[0] = _mm_aesenc_si128(
                acc_chunk[0],
                _mm_add_epi64(
                    d_chunk[0],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[0], blk)),
                ),
            );
            acc_chunk[1] = _mm_aesenc_si128(
                acc_chunk[1],
                _mm_add_epi64(
                    d_chunk[1],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[1], blk)),
                ),
            );
            acc_chunk[2] = _mm_aesenc_si128(
                acc_chunk[2],
                _mm_add_epi64(
                    d_chunk[2],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[2], blk)),
                ),
            );
            acc_chunk[3] = _mm_aesenc_si128(
                acc_chunk[3],
                _mm_add_epi64(
                    d_chunk[3],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[3], blk)),
                ),
            );
        }
        // State-Feedback: stride-3 cross-infect (gcd(3,8)=1 → full diffusion in 3 rounds)
        for (i, d_i) in d.iter_mut().enumerate() {
            let src = (i + 3) % 8;
            d_i[0] = _mm_xor_si128(d_i[0], acc[src * 4]);
            d_i[1] = _mm_xor_si128(d_i[1], acc[src * 4 + 1]);
            d_i[2] = _mm_xor_si128(d_i[2], acc[src * 4 + 2]);
            d_i[3] = _mm_xor_si128(d_i[3], acc[src * 4 + 3]);
        }
        // Cyclic accumulator rotation (shift groups of 4)
        let old = *acc;
        for i in 0..8 {
            acc[i * 4] = old[((i + 1) % 8) * 4];
            acc[i * 4 + 1] = old[((i + 1) % 8) * 4 + 1];
            acc[i * 4 + 2] = old[((i + 1) % 8) * 4 + 2];
            acc[i * 4 + 3] = old[((i + 1) % 8) * 4 + 3];
        }
    }

    // 2. Intermediate Mix: Intra-register Rotation
    // Simulates _mm512_alignr_epi64(_, _, 2)
    let old_m = *acc;
    for i in 0..8 {
        acc[i * 4] = old_m[i * 4 + 1];
        acc[i * 4 + 1] = old_m[i * 4 + 2];
        acc[i * 4 + 2] = old_m[i * 4 + 3];
        acc[i * 4 + 3] = old_m[i * 4];
    }

    // 3. Cross-Accumulator Diffusion Stage 1 (Pairs 0-4, 1-5...)
    // XOR lower, ADD upper (Asymmetric)
    for lane in 0..4 {
        for i in 0..4 {
            let lo_val = acc[i * 4 + lane];
            let hi_val = acc[(i + 4) * 4 + lane];
            acc[i * 4 + lane] = _mm_xor_si128(lo_val, hi_val);
            acc[(i + 4) * 4 + lane] = _mm_add_epi64(hi_val, lo_val);
        }
    }

    // 4. Cross-Accumulator Diffusion Stage 2 (Pairs 0-2, 1-3...)
    // Ensures full diameter-3 diffusion
    for lane in 0..4 {
        let a0 = acc[0 * 4 + lane];
        let a2 = acc[2 * 4 + lane];
        acc[0 * 4 + lane] = _mm_xor_si128(a0, a2);
        acc[2 * 4 + lane] = _mm_add_epi64(a2, a0);

        let a1 = acc[1 * 4 + lane];
        let a3 = acc[3 * 4 + lane];
        acc[1 * 4 + lane] = _mm_xor_si128(a1, a3);
        acc[3 * 4 + lane] = _mm_add_epi64(a3, a1);

        let a4 = acc[4 * 4 + lane];
        let a6 = acc[6 * 4 + lane];
        acc[4 * 4 + lane] = _mm_xor_si128(a4, a6);
        acc[6 * 4 + lane] = _mm_add_epi64(a6, a4);

        let a5 = acc[5 * 4 + lane];
        let a7 = acc[7 * 4 + lane];
        acc[5 * 4 + lane] = _mm_xor_si128(a5, a7);
        acc[7 * 4 + lane] = _mm_add_epi64(a7, a5);
    }

    // 5. Second Half Rounds: Data Rotation (d_{i+4} -> acc_i)
    for &rk in rk_base.iter().take(ROUNDS).skip(mid) {
        for (i, (acc_chunk, lo_chunk)) in acc
            .chunks_exact_mut(4)
            .zip(lo_all.chunks_exact(4))
            .enumerate()
        {
            let data_idx = (i + 4) % 8;
            acc_chunk[0] = _mm_aesenc_si128(
                acc_chunk[0],
                _mm_add_epi64(
                    d[data_idx][0],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[0], blk)),
                ),
            );
            acc_chunk[1] = _mm_aesenc_si128(
                acc_chunk[1],
                _mm_add_epi64(
                    d[data_idx][1],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[1], blk)),
                ),
            );
            acc_chunk[2] = _mm_aesenc_si128(
                acc_chunk[2],
                _mm_add_epi64(
                    d[data_idx][2],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[2], blk)),
                ),
            );
            acc_chunk[3] = _mm_aesenc_si128(
                acc_chunk[3],
                _mm_add_epi64(
                    d[data_idx][3],
                    _mm_add_epi64(rk, _mm_add_epi64(lo_chunk[3], blk)),
                ),
            );
        }
        // State-Feedback: stride-3 cross-infect (second half)
        for (i, d_i) in d.iter_mut().enumerate() {
            let src = (i + 3) % 8;
            d_i[0] = _mm_xor_si128(d_i[0], acc[src * 4]);
            d_i[1] = _mm_xor_si128(d_i[1], acc[src * 4 + 1]);
            d_i[2] = _mm_xor_si128(d_i[2], acc[src * 4 + 2]);
            d_i[3] = _mm_xor_si128(d_i[3], acc[src * 4 + 3]);
        }
        let old = *acc;
        for i in 0..8 {
            acc[i * 4] = old[((i + 1) % 8) * 4];
            acc[i * 4 + 1] = old[((i + 1) % 8) * 4 + 1];
            acc[i * 4 + 2] = old[((i + 1) % 8) * 4 + 2];
            acc[i * 4 + 3] = old[((i + 1) % 8) * 4 + 3];
        }
    }

    // 6. Final Mix: Intra-register Rotation
    let old_f = *acc;
    for i in 0..8 {
        acc[i * 4] = old_f[i * 4 + 1];
        acc[i * 4 + 1] = old_f[i * 4 + 2];
        acc[i * 4 + 2] = old_f[i * 4 + 3];
        acc[i * 4 + 3] = old_f[i * 4];
    }

    // 7. Davies-Meyer Feed-Forward
    for i in 0..32 {
        acc[i] = _mm_xor_si128(acc[i], saves[i]);
    }
}

// =============================================================================
// BLOCK COMPRESSION
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl AesNiState {
    /// Process 1024-byte chunks to match AVX-512 8-accumulator model.
    // SAFETY: Requires AES/SSE2 CPU features (enforced by dispatcher).
    // Calls `compress_block` with pointers from validated slice chunks via `chunks_exact`.
    #[target_feature(enable = "aes")]
    #[target_feature(enable = "sse2")]
    #[allow(unsafe_code)]
    pub unsafe fn update(&mut self, input: &[u8]) {
        let mut acc = self.acc;
        let mut block_idx = self.block_count;

        // Process 2 blocks (1024 bytes) per iteration
        let mut chunks = input.chunks_exact(1024);
        for big_chunk in chunks.by_ref() {
            let ptr = big_chunk.as_ptr();
            compress_block(&mut acc, ptr, block_idx);
            block_idx += 1;
            compress_block(&mut acc, ptr.add(BLOCK_SIZE), block_idx);
            block_idx += 1;
        }

        // Handle remainder FULL blocks (512B each)
        for chunk in chunks.remainder().chunks_exact(BLOCK_SIZE) {
            compress_block(&mut acc, chunk.as_ptr(), block_idx);
            block_idx += 1;
        }

        self.acc = acc;
        self.block_count = block_idx;
    }
}
