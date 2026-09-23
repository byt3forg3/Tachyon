//! Portable AVX-512 Path Emulation
//!
//! Software emulation of the AVX-512-style bulk pipeline (plus routing).
//! Produces byte-identical results to AES-NI / AVX-512 backends.

use super::short::hash_short;
use super::state::{TachyonState, ROUNDS};
use super::utils::{aesenc, clmulepi64, ternary_xor, U128};
use crate::engine::dispatcher::CHUNK_SIZE;
use crate::engine::parallel::MerkleTree;
use crate::kernels::constants::{
    BLOCK_SIZE, C0, C1, C2, C3, C4, C5, C6, C7, CHAOS_BASE, CLMUL_CONSTANT, CLMUL_CONSTANT2,
    GOLDEN_RATIO, LANE_OFFSETS, LANE_STRIDE, NUM_LANES, REMAINDER_CHUNK_SIZE, RK_CHAIN, VEC_SIZE,
    WHITENING0, WHITENING1,
};

// =============================================================================
// LINEAR CORE
// =============================================================================

/// Initialize accumulators with seed and optional key.
fn linear_init(s: &mut TachyonState) {
    let c_vals = [C0, C1, C2, C3, C4, C5, C6, C7];
    for (i, acc) in s.acc.iter_mut().enumerate() {
        let base = c_vals[i / LANE_STRIDE];
        let offset = (i as u64 % LANE_STRIDE as u64) * 2;
        *acc = U128::from_u64s(base + offset, base + offset + 1);
    }

    let seed_val = if s.seed != 0 { s.seed } else { C5 };
    let seed_vec = U128::from_u64s(seed_val, seed_val);
    for acc in &mut s.acc {
        *acc = aesenc(*acc, seed_vec);
    }

    if s.has_key {
        let mut k0_arr = [0u8; VEC_SIZE];
        k0_arr.copy_from_slice(&s.key[0..VEC_SIZE]);
        let mut k1_arr = [0u8; VEC_SIZE];
        k1_arr.copy_from_slice(&s.key[VEC_SIZE..32]);
        let k0 = U128 { b: k0_arr };
        let k1 = U128 { b: k1_arr };
        let gr = U128::from_u64s(GOLDEN_RATIO, GOLDEN_RATIO);
        let k2 = k0.xor(&gr);
        let k3 = k1.xor(&gr);
        let keys = [k0, k1, k2, k3];

        for (i, offset_val) in LANE_OFFSETS.iter().enumerate().take(NUM_LANES) {
            let lo = U128::from_u64s(*offset_val, *offset_val);
            for (j, key) in keys.iter().enumerate() {
                let idx = i * LANE_STRIDE + j;
                s.acc[idx] = aesenc(s.acc[idx], key.add_epi64(&lo));
                s.acc[idx] = aesenc(s.acc[idx], *key);
            }
        }
    }
}

/// Compress a single `BLOCK_SIZE` byte block into the accumulator state.
#[allow(clippy::too_many_lines)]
fn linear_compress(s: &mut TachyonState, data: &[u8], block_idx: u64) {
    // ── 1. Round-key and lane setup ──────────────────────────────────────────
    let mid = ROUNDS / 2;
    let blk = U128::from_u64s(block_idx, block_idx);
    let wk = U128::from_u64s(WHITENING0, WHITENING1);

    let mut rk_base = [U128::zero(); 10];
    for (i, rk) in rk_base.iter_mut().enumerate().take(ROUNDS) {
        *rk = U128::from_u64s(RK_CHAIN[i].0, RK_CHAIN[i].1);
    }

    let mut lo_all = [U128::zero(); 32];
    for (i, lo) in lo_all.iter_mut().enumerate() {
        *lo = U128::from_u64s(LANE_OFFSETS[i], LANE_OFFSETS[i]);
    }

    let saves = s.acc;

    // ── 2. Load and whiten input block ───────────────────────────────────────
    let mut d = [[U128::zero(); LANE_STRIDE]; NUM_LANES];
    for (i, di) in d.iter_mut().enumerate() {
        for (j, dij) in di.iter_mut().enumerate() {
            let off = (i * LANE_STRIDE + j) * VEC_SIZE;
            let mut val = U128::zero();
            val.b.copy_from_slice(&data[off..off + VEC_SIZE]);
            *dij = aesenc(val, wk);
        }
    }

    // ── 3. First-half rounds: direct mapping ─────────────────────────────────
    for rk_val in rk_base.iter().take(mid) {
        let rk = *rk_val;
        for (i, acc) in s.acc.iter_mut().enumerate() {
            let group = i / LANE_STRIDE;
            let item = i % LANE_STRIDE;
            let key = d[group][item]
                .add_epi64(&rk)
                .add_epi64(&lo_all[i])
                .add_epi64(&blk);
            *acc = aesenc(*acc, key);
        }

        for (i, di) in d.iter_mut().enumerate() {
            let src = (i + 3) % NUM_LANES;
            for (j, dij) in di.iter_mut().enumerate() {
                *dij = dij.xor(&s.acc[src * LANE_STRIDE + j]);
            }
        }

        let old_acc = s.acc;
        for (i, group) in s.acc.chunks_mut(LANE_STRIDE).enumerate() {
            let src = (i + 1) % NUM_LANES;
            group.copy_from_slice(&old_acc[src * LANE_STRIDE..src * LANE_STRIDE + LANE_STRIDE]);
        }
    }

    // ── 4. Intermediate lane mix ─────────────────────────────────────────────
    let old_acc = s.acc;
    for (i, group) in s.acc.chunks_mut(LANE_STRIDE).enumerate() {
        for (j, acc) in group.iter_mut().enumerate() {
            *acc = old_acc[i * LANE_STRIDE + ((j + 1) % LANE_STRIDE)];
        }
    }

    // ── 5. Cross-accumulator diffusion: stage 1 ──────────────────────────────
    for lane in 0..LANE_STRIDE {
        for i in 0..LANE_STRIDE {
            let idx_lo = i * LANE_STRIDE + lane;
            let idx_hi = (i + LANE_STRIDE) * LANE_STRIDE + lane;
            let lo = s.acc[idx_lo];
            let hi = s.acc[idx_hi];
            s.acc[idx_lo] = lo.xor(&hi);
            s.acc[idx_hi] = hi.add_epi64(&lo);
        }
    }

    // ── 6. Cross-accumulator diffusion: stage 2 ──────────────────────────────
    for lane in 0..LANE_STRIDE {
        let g0 = lane;
        let g2 = 2 * LANE_STRIDE + lane;
        let a0 = s.acc[g0];
        let a2 = s.acc[g2];
        s.acc[g0] = a0.xor(&a2);
        s.acc[g2] = a2.add_epi64(&a0);

        let g1 = LANE_STRIDE + lane;
        let g3 = 3 * LANE_STRIDE + lane;
        let a1 = s.acc[g1];
        let a3 = s.acc[g3];
        s.acc[g1] = a1.xor(&a3);
        s.acc[g3] = a3.add_epi64(&a1);

        let g4 = 4 * LANE_STRIDE + lane;
        let g6 = 6 * LANE_STRIDE + lane;
        let a4 = s.acc[g4];
        let a6 = s.acc[g6];
        s.acc[g4] = a4.xor(&a6);
        s.acc[g6] = a6.add_epi64(&a4);

        let g5 = 5 * LANE_STRIDE + lane;
        let g7 = 7 * LANE_STRIDE + lane;
        let a5 = s.acc[g5];
        let a7 = s.acc[g7];
        s.acc[g5] = a5.xor(&a7);
        s.acc[g7] = a7.add_epi64(&a5);
    }

    // ── 7. Second-half rounds: data rotation ─────────────────────────────────
    for rk_val in rk_base.iter().skip(mid).take(ROUNDS - mid) {
        let rk = *rk_val;
        for (i, acc) in s.acc.iter_mut().enumerate() {
            let group = i / LANE_STRIDE;
            let item = i % LANE_STRIDE;
            let data_group = (group + LANE_STRIDE) % NUM_LANES;
            let key = d[data_group][item]
                .add_epi64(&rk)
                .add_epi64(&lo_all[i])
                .add_epi64(&blk);
            *acc = aesenc(*acc, key);
        }

        for (i, di) in d.iter_mut().enumerate() {
            let src = (i + 3) % NUM_LANES;
            for (j, dij) in di.iter_mut().enumerate() {
                *dij = dij.xor(&s.acc[src * LANE_STRIDE + j]);
            }
        }

        let old_acc = s.acc;
        for (i, group) in s.acc.chunks_mut(LANE_STRIDE).enumerate() {
            let src = (i + 1) % NUM_LANES;
            group.copy_from_slice(&old_acc[src * LANE_STRIDE..src * LANE_STRIDE + LANE_STRIDE]);
        }
    }

    // ── 8. Final lane mix ────────────────────────────────────────────────────
    let old_acc = s.acc;
    for (i, group) in s.acc.chunks_mut(LANE_STRIDE).enumerate() {
        for (j, acc) in group.iter_mut().enumerate() {
            *acc = old_acc[i * LANE_STRIDE + ((j + 1) % LANE_STRIDE)];
        }
    }

    // ── 9. Davies-Meyer feed-forward ─────────────────────────────────────────
    for (acc, save) in s.acc.iter_mut().zip(saves.iter()) {
        *acc = acc.xor(save);
    }
}

/// Finalize hash: process remainder, tree-merge accumulators, write 256-bit output.
#[allow(clippy::too_many_lines)]
fn linear_finalize(
    s: &mut TachyonState,
    remainder: &[u8],
    rem_len: usize,
    total_len: u64,
    out: &mut [u8],
) {
    // ── 1. Constant-time remainder processing ────────────────────────────────
    let mut offset = 0;
    let wk = U128::from_u64s(WHITENING0, WHITENING1);

    let mut chunk_idx = 0;
    while rem_len - offset >= REMAINDER_CHUNK_SIZE {
        let chunk = &remainder[offset..offset + REMAINDER_CHUNK_SIZE];
        let mut d_vec = [U128::zero(); LANE_STRIDE];
        for (j, dj) in d_vec.iter_mut().enumerate() {
            let mut val = U128::zero();
            val.b
                .copy_from_slice(&chunk[j * VEC_SIZE..(j + 1) * VEC_SIZE]);
            *dj = aesenc(val, wk);
        }

        let base = chunk_idx * LANE_STRIDE;
        let mut lo = [U128::zero(); LANE_STRIDE];
        for (j, lo_j) in lo.iter_mut().enumerate() {
            *lo_j = U128::from_u64s(LANE_OFFSETS[base + j], LANE_OFFSETS[base + j]);
        }
        let mut save = [U128::zero(); LANE_STRIDE];
        save.copy_from_slice(&s.acc[base..base + LANE_STRIDE]);

        for rk_vals in RK_CHAIN.iter().take(ROUNDS) {
            let rk = U128::from_u64s(rk_vals.0, rk_vals.1);
            for (j, acc_j) in s.acc[base..base + LANE_STRIDE].iter_mut().enumerate() {
                *acc_j = aesenc(*acc_j, d_vec[j].add_epi64(&rk).add_epi64(&lo[j]));
            }
            let tmp = s.acc[base];
            s.acc[base] = s.acc[base + 1];
            s.acc[base + 1] = s.acc[base + 2];
            s.acc[base + 2] = s.acc[base + 3];
            s.acc[base + 3] = tmp;

            for (j, dj) in d_vec.iter_mut().enumerate() {
                *dj = dj.xor(&s.acc[base + j]);
            }
        }

        for (acc_j, save_j) in s.acc[base..base + LANE_STRIDE].iter_mut().zip(save.iter()) {
            *acc_j = acc_j.xor(save_j);
        }
        offset += REMAINDER_CHUNK_SIZE;
        chunk_idx += 1;
    }

    // ── 2. Final padding block ───────────────────────────────────────────────
    let mut blk = [0u8; REMAINDER_CHUNK_SIZE];
    let left = rem_len - offset;
    if left > 0 {
        blk[0..left].copy_from_slice(&remainder[offset..offset + left]);
    }
    blk[left] = 0x80;

    let mut d0 = [U128::zero(); LANE_STRIDE];
    for (j, d0j) in d0.iter_mut().enumerate() {
        let mut val = U128::zero();
        val.b
            .copy_from_slice(&blk[j * VEC_SIZE..(j + 1) * VEC_SIZE]);
        *d0j = aesenc(val, wk);
    }

    // ── 3. Tree merge (32 -> 16 -> 8 -> 4) ───────────────────────────────────
    // Non-linear reduction using independent constants.
    let merge_rk0 = U128::from_u64s(C5, C5); // ln(11)
    let merge_rk1 = U128::from_u64s(C6, C6); // ln(13)
    let merge_rk2 = U128::from_u64s(C7, C7); // ln(17)

    // Level 0: 32 -> 16
    for i in 0..LANE_STRIDE {
        let t = i * LANE_STRIDE;
        let src = (i + LANE_STRIDE) * LANE_STRIDE;
        for j in 0..LANE_STRIDE {
            s.acc[t + j] = aesenc(s.acc[t + j], s.acc[src + j].xor(&merge_rk0));
            s.acc[t + j] = aesenc(s.acc[t + j], s.acc[t + j].xor(&merge_rk0)); // self-mix
        }
    }
    // Level 1: 16 -> 8
    for i in 0..2 {
        let t = i * LANE_STRIDE;
        let src = (i + 2) * LANE_STRIDE;
        for j in 0..LANE_STRIDE {
            s.acc[t + j] = aesenc(s.acc[t + j], s.acc[src + j].xor(&merge_rk1));
            s.acc[t + j] = aesenc(s.acc[t + j], s.acc[t + j].xor(&merge_rk1)); // self-mix
        }
    }
    // Level 2: 8 -> 4
    {
        let t = 0;
        let src = LANE_STRIDE;
        for j in 0..LANE_STRIDE {
            s.acc[t + j] = aesenc(s.acc[t + j], s.acc[src + j].xor(&merge_rk2));
            s.acc[t + j] = aesenc(s.acc[t + j], s.acc[t + j].xor(&merge_rk2)); // self-mix
        }
    }

    // ── 4. Quadratic CLMUL hardening ─────────────────────────────────────────
    let clmul_k = U128::from_u64s(CLMUL_CONSTANT, CLMUL_CONSTANT2);
    for acc in &mut s.acc[..LANE_STRIDE] {
        // Round 1: polynomial mixing in GF(2)[x]
        let cl1 = clmulepi64(*acc, clmul_k, 0x00).xor(&clmulepi64(*acc, clmul_k, 0x11));
        // AES barrier: polynomial product as round key (degree ~254)
        let mid = aesenc(*acc, cl1);
        // Round 2: self-multiply lo×hi → quadratic in GF(2)[x] (degree ~254²)
        let cl2 = clmulepi64(mid, mid, 0x01);
        // Nonlinear fold: aesenc eliminates linear shortcut back to original state
        *acc = aesenc(*acc, cl1.xor(&cl2));
    }

    // ── 5. Final block processing: length/domain injection ───────────────────
    let mut save0 = [U128::zero(); LANE_STRIDE];
    save0.copy_from_slice(&s.acc[..LANE_STRIDE]);

    let meta = [
        U128::from_u64s(s.domain ^ total_len, CHAOS_BASE),
        U128::from_u64s(total_len, s.domain),
        U128::from_u64s(CHAOS_BASE, total_len),
        U128::from_u64s(s.domain, CHAOS_BASE),
    ];

    for (j, acc) in s.acc[..LANE_STRIDE].iter_mut().enumerate() {
        *acc = ternary_xor(*acc, d0[j], meta[j]);
    }

    for (r, rk_vals) in RK_CHAIN.iter().enumerate().take(ROUNDS) {
        let rk = U128::from_u64s(rk_vals.0, rk_vals.1);
        for (j, acc_j) in s.acc[..LANE_STRIDE].iter_mut().enumerate() {
            *acc_j = aesenc(*acc_j, d0[j].add_epi64(&rk));
        }
        let tmp = s.acc[0];
        s.acc[0] = s.acc[1];
        s.acc[1] = s.acc[2];
        s.acc[2] = s.acc[3];
        s.acc[3] = tmp;

        if r % 2 == 1 {
            for (j, dj) in d0.iter_mut().enumerate() {
                *dj = dj.xor(&s.acc[j]);
            }
        }
    }

    for (acc_j, save_j) in s.acc[..LANE_STRIDE].iter_mut().zip(save0.iter()) {
        *acc_j = acc_j.xor(save_j);
    }

    // ── 6. Multi-round key absorption ────────────────────────────────────────
    if s.has_key {
        let mut k0_arr = [0u8; VEC_SIZE];
        k0_arr.copy_from_slice(&s.key[0..VEC_SIZE]);
        let mut k1_arr = [0u8; VEC_SIZE];
        k1_arr.copy_from_slice(&s.key[VEC_SIZE..32]);
        let k0 = U128 { b: k0_arr };
        let k1 = U128 { b: k1_arr };

        // Round 1: cross (k0, k1, k1, k0)
        s.acc[0] = aesenc(s.acc[0], k0);
        s.acc[1] = aesenc(s.acc[1], k1);
        s.acc[2] = aesenc(s.acc[2], k1);
        s.acc[3] = aesenc(s.acc[3], k0);
        // Round 2: inverted cross (k1, k0, k0, k1)
        s.acc[0] = aesenc(s.acc[0], k1);
        s.acc[1] = aesenc(s.acc[1], k0);
        s.acc[2] = aesenc(s.acc[2], k0);
        s.acc[3] = aesenc(s.acc[3], k1);
        // Round 3: direct (k0, k1, k0, k1)
        s.acc[0] = aesenc(s.acc[0], k0);
        s.acc[1] = aesenc(s.acc[1], k1);
        s.acc[2] = aesenc(s.acc[2], k0);
        s.acc[3] = aesenc(s.acc[3], k1);
        // Round 4: halved (k0, k0, k1, k1)
        s.acc[0] = aesenc(s.acc[0], k0);
        s.acc[1] = aesenc(s.acc[1], k0);
        s.acc[2] = aesenc(s.acc[2], k1);
        s.acc[3] = aesenc(s.acc[3], k1);
    }

    // ── 7. Final lane reduction (4 -> 1 -> 256-bit output) ───────────────────
    // Round 1: self-mix
    let mut a = [U128::zero(); 4];
    for (j, aj) in a.iter_mut().enumerate() {
        *aj = aesenc(s.acc[j], s.acc[j]);
    }

    // Round 2: cross-half mix
    let b0 = aesenc(a[0], a[2]);
    let b1 = aesenc(a[1], a[3]);
    let b2 = aesenc(a[2], a[0]);
    let b3 = aesenc(a[3], a[1]);

    // Round 3: adjacent-pair mix with asymmetry break per lane
    let mut c = [U128::zero(); 4];
    c[0] = aesenc(b0, b1);
    c[1] = aesenc(b1, b0.xor(&merge_rk2));
    c[2] = aesenc(b2, b3.xor(&merge_rk1));
    c[3] = aesenc(b3, b2.xor(&merge_rk0));

    // Round 4: cross-half fold
    let d_res0 = aesenc(c[0], c[2]);
    let d_res1 = aesenc(c[1], c[3]);

    // Round 5: final mix for full 256-bit diffusion
    let e0 = aesenc(d_res0, d_res1);
    let e1 = aesenc(d_res1, d_res0.xor(&merge_rk2));

    out[0..VEC_SIZE].copy_from_slice(&e0.b);
    out[VEC_SIZE..32].copy_from_slice(&e1.b);
}

// =============================================================================
// ONE-SHOT ENTRY
// =============================================================================

/// Portable software implementation of Tachyon.
///
/// Handles ALL input sizes:
/// - Small (< `CHUNK_SIZE`): inline linear hash via `oneshot_direct`
/// - Large (>= `CHUNK_SIZE`): Merkle tree with `oneshot_direct` as leaf/node kernel
pub fn oneshot(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    if input.len() >= CHUNK_SIZE {
        return merkle_hash(input, domain, seed, key);
    }
    oneshot_direct(input, domain, seed, key)
}

/// Direct linear hash — no Merkle dispatch.
/// Used internally by `merkle_hash` for leaf and node compressions.
fn oneshot_direct(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    let mut out = [0u8; crate::kernels::constants::HASH_SIZE];
    if input.len() < 64 && seed == 0 && key.is_none() {
        hash_short(input, input.len(), domain, seed, key, &mut out);
    } else {
        let mut s = TachyonState::new(domain, seed, key);
        linear_init(&mut s);

        let mut off = 0;
        let mut chunk_idx = 0;
        while input.len() - off >= BLOCK_SIZE {
            linear_compress(&mut s, &input[off..off + BLOCK_SIZE], chunk_idx);
            chunk_idx += 1;
            off += BLOCK_SIZE;
        }

        linear_finalize(
            &mut s,
            &input[off..],
            input.len() - off,
            input.len() as u64,
            &mut out,
        );
    }
    out
}

// =============================================================================
// MERKLE TREE (Large Input Path)
// =============================================================================

/// Merkle-tree hash for inputs >= `CHUNK_SIZE`.
///
/// Mirrors `MerkleTree` from `engine/parallel.rs` but forces `oneshot_direct`
/// (the portable linear kernel) for all leaf and node compressions.
fn merkle_hash(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    let mut tree = MerkleTree::with_kernel(oneshot_direct, domain, seed);
    if let Some(key_ref) = key {
        tree.set_key(key_ref);
    }

    let full_bytes = (input.len() / CHUNK_SIZE) * CHUNK_SIZE;
    if full_bytes > 0 {
        tree.process_slice(&input[..full_bytes]);
    }

    tree.finalize(&input[full_bytes..], input.len() as u64)
}
