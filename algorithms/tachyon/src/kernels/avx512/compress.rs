//! AVX-512 Block Compression
//!
//! High-performance update function for 32nd-track architecture.
//! Uses AESENC-derived key schedule for non-linear round key generation
//! and cross-accumulator XOR for full diffusion.

#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]

use super::state::Avx512State;
use crate::kernels::constants::{
    BLOCK_SIZE, LANE_OFFSETS, RK_CHAIN, ROUNDS, WHITENING0, WHITENING1,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::{
    _mm512_add_epi64, _mm512_aesenc_epi128, _mm512_alignr_epi64, _mm512_loadu_si512,
    _mm512_set1_epi64, _mm512_set_epi64, _mm512_xor_si512,
};

// =============================================================================
// AVX-512 BLOCK COMPRESSION
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Avx512State {
    /// Process blocks in `BLOCK_SIZE` byte increments.
    /// Uses 8 accumulators for 32 parallel tracks.
    // SAFETY: Requires AVX-512F/BW/VAES (enforced by dispatcher). Pointer arithmetic uses
    // compile-time offsets on pre-validated BLOCK_SIZE chunks, no out-of-bounds access.
    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "avx512bw")]
    #[target_feature(enable = "vaes")]
    #[allow(unsafe_code)]
    pub unsafe fn update(&mut self, input: &[u8]) {
        let mid = ROUNDS / 2;

        // Load precomputed round key schedule with asymmetric lo/hi pattern
        let rk_base: [_; ROUNDS] = core::array::from_fn(|r| {
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

        // Pre-broadcast per-accumulator lane offsets
        // Each ZMM register covers 4 internal tracks. We assign unique offsets to each.
        let lo: [_; 8] = core::array::from_fn(|i| {
            let base = i * 4;
            _mm512_set_epi64(
                LANE_OFFSETS[base + 3] as i64,
                LANE_OFFSETS[base + 3] as i64,
                LANE_OFFSETS[base + 2] as i64,
                LANE_OFFSETS[base + 2] as i64,
                LANE_OFFSETS[base + 1] as i64,
                LANE_OFFSETS[base + 1] as i64,
                LANE_OFFSETS[base] as i64,
                LANE_OFFSETS[base] as i64,
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

        let mut block_idx = self.block_count;

        let mut acc0 = self.acc[0];
        let mut acc1 = self.acc[1];
        let mut acc2 = self.acc[2];
        let mut acc3 = self.acc[3];
        let mut acc4 = self.acc[4];
        let mut acc5 = self.acc[5];
        let mut acc6 = self.acc[6];
        let mut acc7 = self.acc[7];

        for block in input.chunks_exact(BLOCK_SIZE) {
            let ptr = block.as_ptr();
            let blk = _mm512_set1_epi64(block_idx as i64);

            // Load BLOCK_SIZE bytes of data and pre-whiten
            let mut d0 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.cast()), wk);
            let mut d1 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(64).cast()), wk);
            let mut d2 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(128).cast()), wk);
            let mut d3 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(192).cast()), wk);
            let mut d4 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(256).cast()), wk);
            let mut d5 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(320).cast()), wk);
            let mut d6 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(384).cast()), wk);
            let mut d7 = _mm512_aesenc_epi128(_mm512_loadu_si512(ptr.add(448).cast()), wk);

            // Save state for Davies-Meyer
            let s0 = acc0;
            let s1 = acc1;
            let s2 = acc2;
            let s3 = acc3;
            let s4 = acc4;
            let s5 = acc5;
            let s6 = acc6;
            let s7 = acc7;

            // 1. First Half Rounds: Direct Mapping (d_i -> acc_i)
            for &rk in rk_base.iter().take(mid) {
                acc0 = _mm512_aesenc_epi128(
                    acc0,
                    _mm512_add_epi64(d0, _mm512_add_epi64(rk, _mm512_add_epi64(lo[0], blk))),
                );
                acc1 = _mm512_aesenc_epi128(
                    acc1,
                    _mm512_add_epi64(d1, _mm512_add_epi64(rk, _mm512_add_epi64(lo[1], blk))),
                );
                acc2 = _mm512_aesenc_epi128(
                    acc2,
                    _mm512_add_epi64(d2, _mm512_add_epi64(rk, _mm512_add_epi64(lo[2], blk))),
                );
                acc3 = _mm512_aesenc_epi128(
                    acc3,
                    _mm512_add_epi64(d3, _mm512_add_epi64(rk, _mm512_add_epi64(lo[3], blk))),
                );
                acc4 = _mm512_aesenc_epi128(
                    acc4,
                    _mm512_add_epi64(d4, _mm512_add_epi64(rk, _mm512_add_epi64(lo[4], blk))),
                );
                acc5 = _mm512_aesenc_epi128(
                    acc5,
                    _mm512_add_epi64(d5, _mm512_add_epi64(rk, _mm512_add_epi64(lo[5], blk))),
                );
                acc6 = _mm512_aesenc_epi128(
                    acc6,
                    _mm512_add_epi64(d6, _mm512_add_epi64(rk, _mm512_add_epi64(lo[6], blk))),
                );
                acc7 = _mm512_aesenc_epi128(
                    acc7,
                    _mm512_add_epi64(d7, _mm512_add_epi64(rk, _mm512_add_epi64(lo[7], blk))),
                );

                // State-Feedback: stride-3 cross-infect (gcd(3,8)=1 → full diffusion in 3 rounds)
                d0 = _mm512_xor_si512(d0, acc3);
                d1 = _mm512_xor_si512(d1, acc4);
                d2 = _mm512_xor_si512(d2, acc5);
                d3 = _mm512_xor_si512(d3, acc6);
                d4 = _mm512_xor_si512(d4, acc7);
                d5 = _mm512_xor_si512(d5, acc0);
                d6 = _mm512_xor_si512(d6, acc1);
                d7 = _mm512_xor_si512(d7, acc2);

                let tmp = acc0;
                acc0 = acc1;
                acc1 = acc2;
                acc2 = acc3;
                acc3 = acc4;
                acc4 = acc5;
                acc5 = acc6;
                acc6 = acc7;
                acc7 = tmp;
            }

            // 2. Intermediate Lane Mix (Intra-register)
            acc0 = _mm512_alignr_epi64(acc0, acc0, 2);
            acc1 = _mm512_alignr_epi64(acc1, acc1, 2);
            acc2 = _mm512_alignr_epi64(acc2, acc2, 2);
            acc3 = _mm512_alignr_epi64(acc3, acc3, 2);
            acc4 = _mm512_alignr_epi64(acc4, acc4, 2);
            acc5 = _mm512_alignr_epi64(acc5, acc5, 2);
            acc6 = _mm512_alignr_epi64(acc6, acc6, 2);
            acc7 = _mm512_alignr_epi64(acc7, acc7, 2);

            // 3. Cross-Accumulator Diffusion Stage 1 (Pairs 0-4, 1-5...)
            // XOR lower, ADD upper (Asymmetric)
            let lo_save0 = acc0;
            let lo_save1 = acc1;
            let lo_save2 = acc2;
            let lo_save3 = acc3;
            acc0 = _mm512_xor_si512(acc0, acc4);
            acc1 = _mm512_xor_si512(acc1, acc5);
            acc2 = _mm512_xor_si512(acc2, acc6);
            acc3 = _mm512_xor_si512(acc3, acc7);
            acc4 = _mm512_add_epi64(acc4, lo_save0);
            acc5 = _mm512_add_epi64(acc5, lo_save1);
            acc6 = _mm512_add_epi64(acc6, lo_save2);
            acc7 = _mm512_add_epi64(acc7, lo_save3);

            // 4. Cross-Accumulator Diffusion Stage 2 (Pairs 0-2, 1-3...)
            // Ensures full diameter-3 diffusion
            let bf0 = acc0;
            let bf1 = acc1;
            let bf4 = acc4;
            let bf5 = acc5;
            acc0 = _mm512_xor_si512(acc0, acc2);
            acc2 = _mm512_add_epi64(acc2, bf0);
            acc1 = _mm512_xor_si512(acc1, acc3);
            acc3 = _mm512_add_epi64(acc3, bf1);
            acc4 = _mm512_xor_si512(acc4, acc6);
            acc6 = _mm512_add_epi64(acc6, bf4);
            acc5 = _mm512_xor_si512(acc5, acc7);
            acc7 = _mm512_add_epi64(acc7, bf5);

            // 5. Second Half Rounds: Data Rotation (d_{i+4} -> acc_i)
            for &rk in rk_base.iter().skip(mid) {
                acc0 = _mm512_aesenc_epi128(
                    acc0,
                    _mm512_add_epi64(d4, _mm512_add_epi64(rk, _mm512_add_epi64(lo[0], blk))),
                );
                acc1 = _mm512_aesenc_epi128(
                    acc1,
                    _mm512_add_epi64(d5, _mm512_add_epi64(rk, _mm512_add_epi64(lo[1], blk))),
                );
                acc2 = _mm512_aesenc_epi128(
                    acc2,
                    _mm512_add_epi64(d6, _mm512_add_epi64(rk, _mm512_add_epi64(lo[2], blk))),
                );
                acc3 = _mm512_aesenc_epi128(
                    acc3,
                    _mm512_add_epi64(d7, _mm512_add_epi64(rk, _mm512_add_epi64(lo[3], blk))),
                );
                acc4 = _mm512_aesenc_epi128(
                    acc4,
                    _mm512_add_epi64(d0, _mm512_add_epi64(rk, _mm512_add_epi64(lo[4], blk))),
                );
                acc5 = _mm512_aesenc_epi128(
                    acc5,
                    _mm512_add_epi64(d1, _mm512_add_epi64(rk, _mm512_add_epi64(lo[5], blk))),
                );
                acc6 = _mm512_aesenc_epi128(
                    acc6,
                    _mm512_add_epi64(d2, _mm512_add_epi64(rk, _mm512_add_epi64(lo[6], blk))),
                );
                acc7 = _mm512_aesenc_epi128(
                    acc7,
                    _mm512_add_epi64(d3, _mm512_add_epi64(rk, _mm512_add_epi64(lo[7], blk))),
                );

                // State-Feedback: stride-3 cross-infect (second half)
                d0 = _mm512_xor_si512(d0, acc3);
                d1 = _mm512_xor_si512(d1, acc4);
                d2 = _mm512_xor_si512(d2, acc5);
                d3 = _mm512_xor_si512(d3, acc6);
                d4 = _mm512_xor_si512(d4, acc7);
                d5 = _mm512_xor_si512(d5, acc0);
                d6 = _mm512_xor_si512(d6, acc1);
                d7 = _mm512_xor_si512(d7, acc2);

                let tmp = acc0;
                acc0 = acc1;
                acc1 = acc2;
                acc2 = acc3;
                acc3 = acc4;
                acc4 = acc5;
                acc5 = acc6;
                acc6 = acc7;
                acc7 = tmp;
            }

            // 6. Final Lane Mix
            acc0 = _mm512_alignr_epi64(acc0, acc0, 2);
            acc1 = _mm512_alignr_epi64(acc1, acc1, 2);
            acc2 = _mm512_alignr_epi64(acc2, acc2, 2);
            acc3 = _mm512_alignr_epi64(acc3, acc3, 2);
            acc4 = _mm512_alignr_epi64(acc4, acc4, 2);
            acc5 = _mm512_alignr_epi64(acc5, acc5, 2);
            acc6 = _mm512_alignr_epi64(acc6, acc6, 2);
            acc7 = _mm512_alignr_epi64(acc7, acc7, 2);

            // 7. Davies-Meyer Feed-Forward
            acc0 = _mm512_xor_si512(acc0, s0);
            acc1 = _mm512_xor_si512(acc1, s1);
            acc2 = _mm512_xor_si512(acc2, s2);
            acc3 = _mm512_xor_si512(acc3, s3);
            acc4 = _mm512_xor_si512(acc4, s4);
            acc5 = _mm512_xor_si512(acc5, s5);
            acc6 = _mm512_xor_si512(acc6, s6);
            acc7 = _mm512_xor_si512(acc7, s7);
            block_idx += 1;
        }

        // Store back state
        self.acc[0] = acc0;
        self.acc[1] = acc1;
        self.acc[2] = acc2;
        self.acc[3] = acc3;
        self.acc[4] = acc4;
        self.acc[5] = acc5;
        self.acc[6] = acc6;
        self.acc[7] = acc7;
        self.block_count = block_idx;
    }
}
