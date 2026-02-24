//! Shared types used across the Tachyon library.

use core::fmt;
#[cfg(feature = "std")]
use std::error;

// =============================================================================
// KERNEL INTERFACE
// =============================================================================

/// Unified kernel function signature: `(input, domain_id, seed, key) -> hash`.
///
/// All hardware backends (AVX-512, AES-NI) and the portable fallback implement
/// this same signature so the dispatcher can swap them at runtime.
pub type KernelFn = fn(
    &[u8],
    u64,
    u64,
    Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE];

// =============================================================================
// DOMAIN SEPARATION
// =============================================================================

/// Domain identifiers to prevent cross-domain attacks.
///
/// Ensures that `Hash(data, domain=A) ≠ Hash(data, domain=B)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum TachyonDomain {
    /// Generic hashing (default)
    Generic = 0,
    /// File checksums and deduplication
    FileChecksum = 1,
    /// Key derivation function
    KeyDerivation = 2,
    /// Message authentication code
    MessageAuth = 3,
    /// Database indexing
    DatabaseIndex = 4,
    /// Content-addressable storage
    ContentAddressed = 5,
}

/// Create a custom domain with a user-defined ID.
///
/// Sets the sentinel bit `0x1000_0000_0000_0000` to ensure custom IDs
/// never collide with the built-in `TachyonDomain` variants (which use
/// values 0–5).
#[must_use]
pub const fn custom_domain(id: u16) -> u64 {
    0x1000_0000_0000_0000_u64 | (id as u64)
}

impl TachyonDomain {
    /// Convert domain to 64-bit constant.
    #[must_use]
    pub const fn to_u64(self) -> u64 {
        self as u64
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Error for unsupported CPU features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeatureError {
    missing: &'static str,
}

impl CpuFeatureError {
    /// Create a new `CpuFeatureError` describing the missing CPU feature.
    pub const fn new(missing: &'static str) -> Self {
        Self { missing }
    }
}

impl fmt::Display for CpuFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CPU feature '{}' required. Tachyon needs AVX-512F, AVX-512BW, VAES. \
             Supported: Intel Ice Lake+, AMD Zen 4+",
            self.missing
        )
    }
}

#[cfg(feature = "std")]
impl error::Error for CpuFeatureError {}
