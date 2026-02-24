//! Tachyon Kernel Constants
//!
//! All constants (except Golden Ratio) are derived from a single rule:
//!
//! ```text
//! constant = floor(frac(ln(p)) * 2^64)
//! ```
//!
//! where p is a prime number and frac(x) = x - floor(x).
//!
//! This ensures "nothing up my sleeve" — every constant is independently
//! reproducible from the natural logarithm of a prime.
//!
//! Verify: `python3 scripts/generate_constants.py`
//!
//! Prime assignment (consecutive, partitioned by purpose):
//!   C0-C3, C5-C7      : ln(2, 3, 5, 7, 11, 13, 17)
//!   `WHITENING0/1`     : ln(19), ln(23)
//!   `KEY_SCHEDULE_MULT`: ln(29)
//!   `CLMUL_CONSTANT`   : ln(31)
//!   `LANE_OFFSETS`     : ln(37..191) — 32 consecutive primes
//!   C4, `KEY_SCHEDULE_BASE`, `CHAOS_BASE`: Golden Ratio (φ)

// =============================================================================
// ROUNDS
// =============================================================================

/// 10 rounds for complete AES diffusion.
pub const ROUNDS: usize = 10;

// =============================================================================
// GOLDEN RATIO
// =============================================================================

/// Golden Ratio (φ) in 64-bit fixed-point: floor(2^64 / φ)
pub const GOLDEN_RATIO: u64 = 0x9E37_79B9_7F4A_7C15;

// =============================================================================
// INITIALIZATION CONSTANTS — frac(ln(p)) for consecutive primes
// =============================================================================

pub const C0: u64 = 0xB172_17F7_D1CF_79AB; // ln(2)
pub const C1: u64 = 0x193E_A7AA_D030_A976; // ln(3)
pub const C2: u64 = 0x9C04_1F7E_D8D3_36AF; // ln(5)
pub const C3: u64 = 0xF227_2AE3_25A5_7546; // ln(7)
pub const C4: u64 = GOLDEN_RATIO; // φ
pub const C5: u64 = 0x65DC_76EF_E6E9_76F7; // ln(11)
pub const C6: u64 = 0x90A0_8566_318A_1FD0; // ln(13)
pub const C7: u64 = 0xD54D_783F_4FEF_39DF; // ln(17)

// =============================================================================
// STRUCTURAL CONSTANTS
// =============================================================================

/// Block size for main compression function (in bytes).
pub const BLOCK_SIZE: usize = 512;

/// Remainder chunk size for finalization (in bytes).
pub const REMAINDER_CHUNK_SIZE: usize = 64;

/// Number of parallel lanes in the accumulator state.
pub const NUM_LANES: usize = 8;

/// Elements per lane (number of 128-bit vectors per lane).
pub const LANE_STRIDE: usize = 4;

/// Size of a single 128-bit vector in bytes.
pub const VEC_SIZE: usize = 16;

/// AES GF(2^8) reduction polynomial: x^8 + x^4 + x^3 + x + 1
pub const GF_POLY: u8 = 0x1b;

/// Hash output size in bytes (256-bit digest).
pub const HASH_SIZE: usize = 32;

// =============================================================================
// KEY SCHEDULE
// =============================================================================

/// Starting value for the AESENC-derived round key chain.
pub const KEY_SCHEDULE_BASE: u64 = GOLDEN_RATIO;
/// Multiplier for per-round diversification: frac(ln(29))
pub const KEY_SCHEDULE_MULT: u64 = 0x5E07_1979_BFC3_D7AC; // ln(29)

/// Per-accumulator lane offsets — 32 unique offsets for full track diversification.
/// Derived from primes 37..191.
pub const LANE_OFFSETS: [u64; 32] = [
    0x9C65_1DC7_58F7_A6F2, // ln(37)
    0xB6AC_A8B1_D589_B575, // ln(41)
    0xC2DE_02C2_9D82_22CB, // ln(43)
    0xD9A3_45F2_1E16_CB31, // ln(47)
    0xF865_0D04_4795_568F, // ln(53)
    0x13D9_7E71_CA5E_2DA9, // ln(59)
    0x1C62_3AC4_9B03_386C, // ln(61)
    0x3466_BC4A_044B_5829, // ln(67)
    0x433E_FD09_35B2_3D6B, // ln(71)
    0x4A5B_8CC8_8BF9_8CD3, // ln(73)
    0x5E94_226B_EC5C_BFB8, // ln(79)
    0x6B39_2358_B920_6784, // ln(83)
    0x7D17_45EB_A2BD_8E2D, // ln(89)
    0x9320_4239_52FE_003B, // ln(97)
    0x9D78_89C6_EE8C_2F8E, // ln(101)
    0xA27D_9956_44FA_F994, // ln(103)
    0xAC3E_82AF_D1D6_DC79, // ln(107)
    0xB0FC_2CC0_5541_91F5, // ln(109)
    0xBA36_168C_E0D6_EE1D, // ln(113)
    0xD81C_A518_0B90_858D, // ln(127)
    0xE00C_EE88_B218_9A5C, // ln(131)
    0xEB83_DEB5_6027_349A, // ln(137)
    0xEF39_AF05_C2C4_931B, // ln(139)
    0x0102_A006_F9CB_3C2A, // ln(149)
    0x046C_738E_0014_C2F8, // ln(151)
    0x0E66_2006_8217_19E4, // ln(157)
    0x1800_035E_755E_C056, // ln(163)
    0x1E34_D7AD_75D7_A815, // ln(167)
    0x273E_1E31_1EA1_A70B, // ln(173)
    0x2FF8_8423_D216_0504, // ln(179)
    0x32D0_B391_A3CA_A870, // ln(181)
    0x4094_FDCB_1C2E_7EE1, // ln(191)
];

// =============================================================================
// FINALIZATION
// =============================================================================

/// Chaos injection for entropy in sparse inputs.
pub const CHAOS_BASE: u64 = GOLDEN_RATIO;

/// Independent CLMUL constant: frac(ln(31)).
/// Polynomial coefficient in GF(2^128).
pub const CLMUL_CONSTANT: u64 = 0x6F19_C912_256B_3E22; // ln(31)

/// Independent constant for CLMUL polynomial differentiation: frac(ln(193)).
pub const CLMUL_CONSTANT2: u64 = 0x433F_AA0A_5398_8000; // ln(193)

/// Constants for data pre-whitening: frac(ln(19)) and frac(ln(23)).
pub const WHITENING0: u64 = 0xF1C6_C0C0_9665_8E40; // ln(19)
pub const WHITENING1: u64 = 0x22AF_BFBA_367E_0122; // ln(23)

// Merkle tree node type tags (XORed into domain field to distinguish leaf/node)
pub const DOMAIN_LEAF: u64 = 0xFFFF_FFFF_0000_0000;
pub const DOMAIN_NODE: u64 = 0xFFFF_FFFF_0000_0001;

// =============================================================================
// SHORT PATH PRECOMPUTED STATE
// =============================================================================

/// Precomputed post-merge state for `seed=0, key=None`.
/// Recompute: `cargo test dump_precomputed_short_init -- --ignored --nocapture`
pub const SHORT_INIT: [(u64, u64); 4] = [
    (0x8572_268C_3E8B_949A, 0x5526_0EB0_F6D0_8B28),
    (0x7B6B_8694_04C5_10F3, 0x5815_3672_FF72_57BB),
    (0x23AE_5234_151A_861E, 0x436D_9112_8FA3_A475),
    (0x2D3E_A94F_6D07_F7BC, 0x31C0_28B3_04D2_3746),
];

// =============================================================================
// PRECOMPUTED ROUND KEY CHAIN
// =============================================================================

/// Precomputed AESENC-derived round key schedule.
/// Recompute: `cargo test dump_precomputed_rk_chain -- --ignored --nocapture`
pub const RK_CHAIN: [(u64, u64); 10] = [
    (0x9E37_79B9_7F4A_7C15, 0xFBEB_0F56_99A3_0AE2),
    (0xE077_2D41_8B60_4247, 0xCB99_FBAD_2127_15AA),
    (0x9943_E41C_900E_A2BD, 0x3391_839B_4E1D_B7D2),
    (0x3FDD_17D0_1F01_E973, 0x4FE6_2D4E_63CB_7DB7),
    (0x7C5B_6818_36BF_20E5, 0x20EA_7205_0896_74B4),
    (0x57E5_2B0B_6FD1_22C4, 0x92E2_3D97_BDB0_1EAB),
    (0x9E66_7CEF_9217_7102, 0x1A17_61F6_D1C3_AAA5),
    (0x5976_F92D_468F_E2FD, 0xAE36_2340_5BAF_D085),
    (0xCD2A_F6F6_F29B_F341, 0xD310_BEDD_A16B_12D4),
    (0xD11A_12CC_D34B_BD1B, 0xAC09_BEFD_5925_A5FE),
];
