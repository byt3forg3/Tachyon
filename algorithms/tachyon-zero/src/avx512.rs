//! AVX-512 Optimized Core
//!
//! High-performance update function using 512-bit registers.
//!
//! # Architecture
//! - **State**: 512-bit (4x 128-bit accumulators in 4 ZMM registers, mapped to `acc0-acc3`).
//! - **Injection**: AES-NI (1 Round per block).
//! - **Mixing**: `vpternlogd` (3-way boolean logic) for non-linear diffusion.
//! - **Finalization**: Parallel AES folding + Horizontal Lane Shuffle.

use crate::constants::{KEYS, Z0, Z1, Z2, Z3};

#[cfg(target_arch = "x86")]
use core::arch::x86::{
    __m512i, _mm256_storeu_si256, _mm512_aesenc_epi128, _mm512_castsi512_si256, _mm512_loadu_si512,
    _mm512_set1_epi64, _mm512_shuffle_i64x2, _mm512_ternarylogic_epi64, _mm512_xor_si512,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m512i, _mm256_storeu_si256, _mm512_aesenc_epi128, _mm512_castsi512_si256, _mm512_loadu_si512,
    _mm512_set1_epi64, _mm512_shuffle_i64x2, _mm512_ternarylogic_epi64, _mm512_xor_si512,
};

// =============================================================================
// STATE STRUCT
// =============================================================================

/// AVX-512 aligned state.
#[derive(Clone)]
pub struct Hasher {
    acc0: __m512i,
    acc1: __m512i,
    acc2: __m512i,
    acc3: __m512i,
    buffer: [u8; 256],
    buf_len: usize,
}

// =============================================================================
// IMPLEMENTATION
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Hasher {
    #[target_feature(enable = "avx512f")]
    #[allow(unsafe_code)]
    /// Create a new AVX-512 hasher state.
    ///
    /// # Safety
    /// Requires AVX-512F instruction set support.
    pub unsafe fn new() -> Self {
        #[allow(clippy::cast_possible_wrap)]
        Self {
            acc0: _mm512_set1_epi64(Z0 as i64),
            acc1: _mm512_set1_epi64(Z1 as i64),
            acc2: _mm512_set1_epi64(Z2 as i64),
            acc3: _mm512_set1_epi64(Z3 as i64),
            buffer: [0u8; 256],
            buf_len: 0,
        }
    }

    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "avx512bw")]
    #[target_feature(enable = "vaes")]
    #[target_feature(enable = "vpclmulqdq")]
    #[allow(unsafe_code)]
    /// Update the state with new input data.
    ///
    /// # Safety
    /// Requires AVX-512F, AVX-512BW, VAES, and VPCLMULQDQ instruction set support.
    #[allow(clippy::cast_possible_wrap)]
    pub unsafe fn update(&mut self, mut input: &[u8]) {
        let k0 = _mm512_set1_epi64(KEYS[0] as i64);
        let k1 = _mm512_set1_epi64(KEYS[1] as i64);
        let k2 = _mm512_set1_epi64(KEYS[2] as i64);
        let k3 = _mm512_set1_epi64(KEYS[3] as i64);

        // 1. Fill Buffer if not empty
        if self.buf_len > 0 {
            let needed = 256 - self.buf_len;
            if input.len() >= needed {
                core::ptr::copy_nonoverlapping(
                    input.as_ptr(),
                    self.buffer.as_mut_ptr().add(self.buf_len),
                    needed,
                );
                self.compress_block(self.buffer.as_ptr(), k0, k1, k2, k3);
                self.buf_len = 0;
                input = &input[needed..];
            } else {
                // Not enough to fill buffer
                core::ptr::copy_nonoverlapping(
                    input.as_ptr(),
                    self.buffer.as_mut_ptr().add(self.buf_len),
                    input.len(),
                );
                self.buf_len += input.len();
                return;
            }
        }

        // 2. Process Full Blocks
        let mut ptr = input.as_ptr();
        let mut len = input.len();

        while len >= 256 {
            self.compress_block(ptr, k0, k1, k2, k3);
            ptr = ptr.add(256);
            len -= 256;
        }

        // 3. Buffer Remaining
        if len > 0 {
            core::ptr::copy_nonoverlapping(ptr, self.buffer.as_mut_ptr(), len);
            self.buf_len = len;
        }
    }

    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "vaes")]
    #[allow(unsafe_code)]
    unsafe fn compress_block(
        &mut self,
        ptr: *const u8,
        k0: __m512i,
        k1: __m512i,
        k2: __m512i,
        k3: __m512i,
    ) {
        let d0 = _mm512_loadu_si512(ptr.cast());
        let d1 = _mm512_loadu_si512(ptr.add(64).cast());
        let d2 = _mm512_loadu_si512(ptr.add(128).cast());
        let d3 = _mm512_loadu_si512(ptr.add(192).cast());

        // --- ROUND 1: Injection + AES ---
        self.acc0 = _mm512_aesenc_epi128(self.acc0, _mm512_xor_si512(d0, k0));
        self.acc1 = _mm512_aesenc_epi128(self.acc1, _mm512_xor_si512(d1, k1));
        self.acc2 = _mm512_aesenc_epi128(self.acc2, _mm512_xor_si512(d2, k2));
        self.acc3 = _mm512_aesenc_epi128(self.acc3, _mm512_xor_si512(d3, k3));

        // --- ROUND 2: VPTERNLOGD ---
        self.acc0 = _mm512_ternarylogic_epi64(self.acc0, d0, k0, 0x96);
        self.acc1 = _mm512_ternarylogic_epi64(self.acc1, d1, k1, 0x96);
        self.acc2 = _mm512_ternarylogic_epi64(self.acc2, d2, k2, 0x96);
        self.acc3 = _mm512_ternarylogic_epi64(self.acc3, d3, k3, 0x96);
    }

    #[target_feature(enable = "avx512f")]
    #[target_feature(enable = "avx512bw")]
    #[target_feature(enable = "vaes")]
    #[target_feature(enable = "vpclmulqdq")]
    #[allow(unsafe_code)]
    /// Finalize the state and return the 32-byte hash.
    ///
    /// # Safety
    /// Requires AVX-512F, AVX-512BW, VAES, and VPCLMULQDQ instruction set support.
    #[allow(clippy::cast_possible_wrap)]
    pub unsafe fn finalize(mut self) -> [u8; 32] {
        let k0 = _mm512_set1_epi64(KEYS[0] as i64);
        let k1 = _mm512_set1_epi64(KEYS[1] as i64);

        // =========================================================================
        // REMAINDER HANDLING
        // =========================================================================

        if self.buf_len > 0 {
            // Pad
            if self.buf_len < 256 {
                self.buffer[self.buf_len] = 0x80;
            }

            // We only process what we have, using a simplified folding for the tail
            let mut ptr = self.buffer.as_ptr();
            let mut len = self.buf_len;

            while len >= 64 {
                let d = _mm512_loadu_si512(ptr.cast());
                self.acc0 = _mm512_xor_si512(self.acc0, d);
                self.acc0 = _mm512_aesenc_epi128(self.acc0, k0);
                ptr = ptr.add(64);
                len -= 64;
            }

            if len > 0 {
                // Partial remaining < 64
                // Safe because buffer is large enough
                let last = _mm512_loadu_si512(ptr.cast());
                self.acc0 = _mm512_xor_si512(self.acc0, last);
                self.acc0 = _mm512_aesenc_epi128(self.acc0, k0);
            }
        }

        // =========================================================================
        // FINAL FOLD
        // =========================================================================

        self.acc0 = _mm512_aesenc_epi128(self.acc0, self.acc1);
        self.acc2 = _mm512_aesenc_epi128(self.acc2, self.acc3);
        self.acc0 = _mm512_aesenc_epi128(self.acc0, self.acc2);

        // Horizontal Mix
        let shuf1 = _mm512_shuffle_i64x2(self.acc0, self.acc0, 0xB1);
        self.acc0 = _mm512_aesenc_epi128(self.acc0, shuf1);

        let shuf2 = _mm512_shuffle_i64x2(self.acc0, self.acc0, 0x4E);
        self.acc0 = _mm512_aesenc_epi128(self.acc0, shuf2);

        // Finalize
        self.acc0 = _mm512_aesenc_epi128(self.acc0, k0);
        self.acc0 = _mm512_aesenc_epi128(self.acc0, k1);

        let mut output = [0u8; 32];
        let lo = _mm512_castsi512_si256(self.acc0);
        _mm256_storeu_si256(output.as_mut_ptr().cast(), lo);
        output
    }
}
