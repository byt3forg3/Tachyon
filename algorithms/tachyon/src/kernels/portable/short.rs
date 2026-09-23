//! Portable Short Path
//!
//! Software short-input path using `U128` operations.

use super::state::ROUNDS;
use super::utils::{aesenc, U128};
use crate::kernels::constants::{
    C0, C5, C6, C7, CHAOS_BASE, GOLDEN_RATIO, LANE_OFFSETS, LANE_STRIDE, REMAINDER_CHUNK_SIZE,
    RK_CHAIN, SHORT_INIT, VEC_SIZE, WHITENING0, WHITENING1,
};

// =============================================================================
// ONE-SHOT SHORT PATH
// =============================================================================

/// One-shot hash for small inputs (< `REMAINDER_CHUNK_SIZE` bytes), mirrors the short path of `AesNiState::finalize`.
#[allow(clippy::too_many_lines)]
pub(super) fn hash_short(
    input: &[u8],
    len: usize,
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
    out: &mut [u8],
) {
    // ── 1. State and key setup ───────────────────────────────────────────────
    let mut acc = [U128::zero(); LANE_STRIDE];
    let has_key = key.is_some();

    if seed == 0 && !has_key {
        for (i, acc_i) in acc.iter_mut().enumerate().take(LANE_STRIDE) {
            *acc_i = U128::from_u64s(SHORT_INIT[i].0, SHORT_INIT[i].1);
        }
    } else {
        let base = C0;
        for (i, acc_i) in acc.iter_mut().enumerate().take(LANE_STRIDE) {
            *acc_i = U128::from_u64s(base + (i as u64) * 2, base + (i as u64) * 2 + 1);
        }
        let s_val = if seed != 0 { seed } else { C5 };
        let s_vec = U128::from_u64s(s_val, s_val);
        for acc_i in &mut acc {
            *acc_i = aesenc(*acc_i, s_vec);
        }

        if let Some(k) = key {
            let mut k0_arr = [0u8; VEC_SIZE];
            k0_arr.copy_from_slice(&k[0..VEC_SIZE]);
            let mut k1_arr = [0u8; VEC_SIZE];
            k1_arr.copy_from_slice(&k[VEC_SIZE..32]);
            let k0 = U128 { b: k0_arr };
            let k1 = U128 { b: k1_arr };
            let gr = U128::from_u64s(GOLDEN_RATIO, GOLDEN_RATIO);
            let k2 = k0.xor(&gr);
            let k3 = k1.xor(&gr);
            let keys = [k0, k1, k2, k3];
            let lo_val = LANE_OFFSETS[0];
            let lo = U128::from_u64s(lo_val, lo_val);
            for (j, k_val) in keys.iter().enumerate() {
                acc[j] = aesenc(acc[j], k_val.add_epi64(&lo));
                acc[j] = aesenc(acc[j], *k_val);
            }
        }
    }

    // ── 2. Block materialization and round processing ───────────────────────
    let wk = U128::from_u64s(WHITENING0, WHITENING1);
    let mut blk = [0u8; REMAINDER_CHUNK_SIZE];
    blk[0..len].copy_from_slice(&input[0..len]);
    blk[len] = 0x80;

    let mut d = [U128::zero(); LANE_STRIDE];
    for (i, di) in d.iter_mut().enumerate() {
        let mut val = U128::zero();
        val.b
            .copy_from_slice(&blk[i * VEC_SIZE..(i + 1) * VEC_SIZE]);
        *di = aesenc(val, wk);
    }

    let saves = acc;

    let meta = [
        U128::from_u64s(domain ^ (len as u64), CHAOS_BASE),
        U128::from_u64s(len as u64, domain),
        U128::from_u64s(CHAOS_BASE, len as u64),
        U128::from_u64s(domain, CHAOS_BASE),
    ];

    for (i, acc_i) in acc.iter_mut().enumerate() {
        *acc_i = acc_i.xor(&d[i].xor(&meta[i]));
    }

    let mut lo = [U128::zero(); LANE_STRIDE];
    for (i, lo_i) in lo.iter_mut().enumerate() {
        *lo_i = U128::from_u64s(LANE_OFFSETS[i], LANE_OFFSETS[i]);
    }

    for (r, rk_vals) in RK_CHAIN.iter().enumerate().take(ROUNDS) {
        let rk = U128::from_u64s(rk_vals.0, rk_vals.1);
        for (i, acc_i) in acc.iter_mut().enumerate().take(LANE_STRIDE) {
            *acc_i = aesenc(*acc_i, d[i].add_epi64(&rk).add_epi64(&lo[i]));
        }
        if r % 2 == 1 {
            let t = acc;
            d[0] = d[0].xor(&t[1]);
            d[1] = d[1].xor(&t[2]);
            d[2] = d[2].xor(&t[3]);
            d[3] = d[3].xor(&t[0]);
        }
        let tmp = acc[0];
        acc[0] = acc[1];
        acc[1] = acc[2];
        acc[2] = acc[3];
        acc[3] = tmp;
    }

    for (acc_i, save_i) in acc.iter_mut().zip(saves.iter()) {
        *acc_i = acc_i.xor(save_i);
    }

    // ── 3. Final lane reduction ──────────────────────────────────────────────
    let mut a = [U128::zero(); LANE_STRIDE];
    for (i, ai) in a.iter_mut().enumerate() {
        *ai = aesenc(acc[i], acc[i]);
    }

    let b0 = aesenc(a[0], a[2]);
    let b1 = aesenc(a[1], a[3]);
    let b2 = aesenc(a[2], a[0]);
    let b3 = aesenc(a[3], a[1]);

    let mut c = [U128::zero(); LANE_STRIDE];
    let merge_rk0 = U128::from_u64s(C5, C5);
    let merge_rk1 = U128::from_u64s(C6, C6);
    let merge_rk2 = U128::from_u64s(C7, C7);
    c[0] = aesenc(b0, b1);
    c[1] = aesenc(b1, b0.xor(&merge_rk2));
    c[2] = aesenc(b2, b3.xor(&merge_rk1));
    c[3] = aesenc(b3, b2.xor(&merge_rk0));

    let d_res0 = aesenc(c[0], c[2]);
    let d_res1 = aesenc(c[1], c[3]);

    let e0 = aesenc(d_res0, d_res1);
    let e1 = aesenc(d_res1, d_res0.xor(&merge_rk2));

    out[0..VEC_SIZE].copy_from_slice(&e0.b);
    out[VEC_SIZE..32].copy_from_slice(&e1.b);
}
