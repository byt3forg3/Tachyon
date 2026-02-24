//! Tachyon Zero Constants
//!
//! All constants are derived from the same principle as Tachyon:
//!
//! ```text
//! constant = floor(frac(ln(p)) * 2^64)
//! ```
//! where p is a prime number.
//!
//! Verify: `python3 scripts/generate_constants.py`

// =============================================================================
// GOLDEN RATIO & SEEDS
// =============================================================================

/// Golden Ratio (φ) in 64-bit fixed-point: floor(2^64 / φ)
#[allow(dead_code)]
pub const GOLDEN_RATIO: u64 = 0x9E37_79B9_7F4A_7C15; // Not used currently

/// Initial Accumulator States (Derived from primes ln(2)..ln(7))
pub const Z0: u64 = 0xB172_17F7_D1CF_79AB; // ln(2)
pub const Z1: u64 = 0x193E_A7AA_D030_A976; // ln(3)
pub const Z2: u64 = 0x9C04_1F7E_D8D3_36AF; // ln(5)
pub const Z3: u64 = 0xF227_2AE3_25A5_7546; // ln(7)

// =============================================================================
// KEY SCHEDULE / MIXING CONSTANTS
// =============================================================================

/// Round Constants for injection (Primes ln(11)..ln(37))
pub const KEYS: [u64; 8] = [
    0x65DC_76EF_E6E9_76F7, // ln(11)
    0x90A0_8566_318A_1FD0, // ln(13)
    0xD54D_783F_4FEF_39DF, // ln(17)
    0xF1C6_C0C0_9665_8E40, // ln(19)
    0x22AF_BFBA_367E_0122, // ln(23)
    0x5E07_1979_BFC3_D7AC, // ln(29)
    0x6F19_C912_256B_3E22, // ln(31)
    0x9C65_1DC7_58F7_A6F2, // ln(37)
];
