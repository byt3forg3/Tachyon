//! Portable State
//!
//! Shared state container and local constants for the portable kernel.

use super::utils::U128;

// =============================================================================
// STATE & TYPES
// =============================================================================

/// Local round count (matches `kernels::constants::ROUNDS`).
pub(super) const ROUNDS: usize = 10;

/// Internal per-call state for the portable hash kernel.
pub(super) struct TachyonState {
    pub(super) acc: [U128; 32],
    pub(super) domain: u64,
    pub(super) seed: u64,
    pub(super) key: [u8; crate::kernels::constants::HASH_SIZE],
    pub(super) has_key: bool,
}

// =============================================================================
// IMPLEMENTATION
// =============================================================================

impl TachyonState {
    pub(super) const fn new(
        domain: u64,
        seed: u64,
        key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
    ) -> Self {
        let mut s = Self {
            acc: [U128::zero(); 32],
            domain,
            seed,
            key: [0u8; crate::kernels::constants::HASH_SIZE],
            has_key: false,
        };
        if let Some(k) = key {
            s.has_key = true;
            let mut i = 0;
            while i < 32 {
                s.key[i] = k[i];
                i += 1;
            }
        }
        s
    }
}
