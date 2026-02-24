//! Portable software implementation of AES and CLMUL primitives.

// AES S-Box
#[rustfmt::skip]
const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16
];

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct U128 {
    pub b: [u8; 16],
}

impl U128 {
    pub const fn zero() -> Self {
        Self { b: [0; 16] }
    }

    pub fn from_u64s(lo: u64, hi: u64) -> Self {
        let mut b = [0u8; 16];
        b[0..8].copy_from_slice(&lo.to_le_bytes());
        b[8..16].copy_from_slice(&hi.to_le_bytes());
        Self { b }
    }

    pub fn xor(&self, other: &Self) -> Self {
        let mut res = Self::zero();
        for (i, res_i) in res.b.iter_mut().enumerate() {
            *res_i = self.b[i] ^ other.b[i];
        }
        res
    }

    pub fn add_epi64(&self, other: &Self) -> Self {
        let a_lo = u64::from_le_bytes([
            self.b[0], self.b[1], self.b[2], self.b[3], self.b[4], self.b[5], self.b[6], self.b[7],
        ]);
        let a_hi = u64::from_le_bytes([
            self.b[8], self.b[9], self.b[10], self.b[11], self.b[12], self.b[13], self.b[14],
            self.b[15],
        ]);
        let b_lo = u64::from_le_bytes([
            other.b[0], other.b[1], other.b[2], other.b[3], other.b[4], other.b[5], other.b[6],
            other.b[7],
        ]);
        let b_hi = u64::from_le_bytes([
            other.b[8],
            other.b[9],
            other.b[10],
            other.b[11],
            other.b[12],
            other.b[13],
            other.b[14],
            other.b[15],
        ]);

        Self::from_u64s(a_lo.wrapping_add(b_lo), a_hi.wrapping_add(b_hi))
    }
}

/// GF(2^8) multiplication by 2 (used in `MixColumns`).
/// Branchless: `b >> 7` extracts the MSB as 0 or 1; multiplying by `GF_POLY`
/// produces the conditional reduction polynomial without a data-dependent branch.
const fn gf_double(b: u8) -> u8 {
    (b << 1) ^ ((b >> 7) * crate::kernels::constants::GF_POLY)
}

/// AES `MixColumns` on a single 4-byte column.
fn mix_column(c: &mut [u8]) {
    let t = [c[0], c[1], c[2], c[3]];
    c[0] = gf_double(t[0] ^ t[1]) ^ t[1] ^ t[2] ^ t[3];
    c[1] = gf_double(t[1] ^ t[2]) ^ t[2] ^ t[3] ^ t[0];
    c[2] = gf_double(t[2] ^ t[3]) ^ t[3] ^ t[0] ^ t[1];
    c[3] = gf_double(t[3] ^ t[0]) ^ t[0] ^ t[1] ^ t[2];
}

pub fn aesenc(state: U128, key: U128) -> U128 {
    let mut s = state.b;

    // SubBytes
    for b in &mut s {
        *b = SBOX[*b as usize];
    }

    // ShiftRows
    // Row 0: No shift
    // Row 1: Shift left 1
    let tmp = s[1];
    s[1] = s[5];
    s[5] = s[9];
    s[9] = s[13];
    s[13] = tmp;
    // Row 2: Shift left 2
    let tmp1 = s[2];
    let tmp2 = s[6];
    s[2] = s[10];
    s[6] = s[14];
    s[10] = tmp1;
    s[14] = tmp2;
    // Row 3: Shift left 3
    let tmp = s[15];
    s[15] = s[11];
    s[11] = s[7];
    s[7] = s[3];
    s[3] = tmp;

    // MixColumns
    mix_column(&mut s[0..4]);
    mix_column(&mut s[4..8]);
    mix_column(&mut s[8..12]);
    mix_column(&mut s[12..16]);

    // AddRoundKey
    let mut res = U128::zero();
    for (i, res_i) in res.b.iter_mut().enumerate() {
        *res_i = s[i] ^ key.b[i];
    }
    res
}

pub fn ternary_xor(a: U128, b: U128, c: U128) -> U128 {
    a.xor(&b).xor(&c)
}

/// Carryless multiplication of two 64-bit integers (widening to 128-bit).
///
/// Implemented branchless: a data-dependent branch on individual bits of `b`
/// could leak timing information. Instead, each bit of `b` is converted to an
/// all-ones/all-zeros mask via `wrapping_neg`, and XOR is always performed.
fn clmul_u64(a: u64, b: u64) -> (u64, u64) {
    let mut res_lo = 0u64;
    let mut res_hi = 0u64;

    for i in 0..64 {
        // Branchless: mask is 0xFFFF... if bit i of b is set, 0 otherwise.
        let mask = ((b >> i) & 1).wrapping_neg();
        let msg_lo = a << i;
        let msg_hi = if i == 0 { 0 } else { a >> (64 - i) }; // i==0 guard: loop counter, not data
        res_lo ^= msg_lo & mask;
        res_hi ^= msg_hi & mask;
    }
    (res_lo, res_hi)
}

pub fn clmulepi64(a: U128, b: U128, imm: i32) -> U128 {
    let a_lo = u64::from_le_bytes([
        a.b[0], a.b[1], a.b[2], a.b[3], a.b[4], a.b[5], a.b[6], a.b[7],
    ]);
    let a_hi = u64::from_le_bytes([
        a.b[8], a.b[9], a.b[10], a.b[11], a.b[12], a.b[13], a.b[14], a.b[15],
    ]);
    let b_lo = u64::from_le_bytes([
        b.b[0], b.b[1], b.b[2], b.b[3], b.b[4], b.b[5], b.b[6], b.b[7],
    ]);
    let b_hi = u64::from_le_bytes([
        b.b[8], b.b[9], b.b[10], b.b[11], b.b[12], b.b[13], b.b[14], b.b[15],
    ]);

    let a_val = if (imm & 0x10) != 0 { a_hi } else { a_lo };
    let b_val = if (imm & 0x01) != 0 { b_hi } else { b_lo };

    let (lo, hi) = clmul_u64(a_val, b_val);
    U128::from_u64s(lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aesenc_manual_verification() {
        let c0 = 0xB172_17F7_D1CF_79AB;
        let seed = 0xDEAD_BEEF;

        let acc = U128::from_u64s(c0, c0 + 1);
        let key = U128::from_u64s(seed, seed);

        let res = aesenc(acc, key);

        let expected_lo = 0x321c_e16f_8973_6a62;
        let expected_hi = 0x321c_e16f_8780_999f;

        println!("Rust U128: {res:?}");
        let res_lo = u64::from_le_bytes([
            res.b[0], res.b[1], res.b[2], res.b[3], res.b[4], res.b[5], res.b[6], res.b[7],
        ]);
        let res_hi = u64::from_le_bytes([
            res.b[8], res.b[9], res.b[10], res.b[11], res.b[12], res.b[13], res.b[14], res.b[15],
        ]);

        println!("Rust Lo: {res_lo:016x}");
        println!("Rust Hi: {res_hi:016x}");

        assert_eq!(res_lo, expected_lo, "Low 64-bit mismatch");
        assert_eq!(res_hi, expected_hi, "High 64-bit mismatch");
    }
}
