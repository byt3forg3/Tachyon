#![cfg_attr(not(feature = "std"), no_std)]

//! # Tachyon Zero
//!
//! An extreme-performance hash function optimized for AVX-512 + VAES.
//!
//! # Design Goals
//! - **Speed:** Maximize throughput (GB/s) on modern `x86_64` CPUs.
//! - **Quality:** Pass `SMHasher` (statistically distinct), but not cryptographic.
//! - **State:** 512-bit (matching ZMM registers).
//! - **Requirement:** AVX-512F + AVX-512BW + VAES + VPCLMULQDQ.

// =============================================================================
// MODULES
// =============================================================================

mod avx512;
mod constants;

// =============================================================================
// EXPORTS
// =============================================================================

// Re-export the AVX-512 Hasher if available
#[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
pub use avx512::Hasher as HasherAvx512;

/// Opaque Hasher that strictly requires AVX-512.
///
/// # Panics
/// Panics on `new()` if the CPU does not support AVX-512F, AVX-512BW, VAES, and VPCLMULQDQ.
#[derive(Clone)]
pub struct Hasher {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
    inner: avx512::Hasher,
    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std")))]
    _dummy: (),
}

impl Hasher {
    #[allow(unsafe_code)]
    /// Create a new hasher instance.
    ///
    /// # Panics
    /// Panics if AVX-512 features are not detected.
    pub fn new() -> Self {
        #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
        {
            if is_avx512_supported() {
                // SAFE: Feature check performed
                let inner = unsafe { avx512::Hasher::new() };
                return Self { inner };
            }
        }

        // Panic on non-x86 or if features missing
        panic!("Tachyon Zero requires AVX-512 (AVX512F, AVX512BW, VAES, VPCLMULQDQ). Support for non-AVX systems has been removed.");
    }

    #[allow(unsafe_code)]
    /// Update the internal state with the given input data.
    pub fn update(&mut self, input: &[u8]) {
        #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
        unsafe {
            self.inner.update(input);
        }
    }

    #[allow(unsafe_code)]
    /// Finalize and return the 32-byte hash.
    pub fn finalize(self) -> [u8; 32] {
        #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
        unsafe {
            self.inner.finalize()
        }
        #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std")))]
        [0u8; 32] // Unreachable
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

/// One-shot hash convenience function
pub fn hash(input: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(input);
    hasher.finalize()
}

/// Returns the name of the hardware backend currently in use.
#[must_use]
pub fn active_backend() -> &'static str {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
    if is_avx512_supported() {
        return "AVX-512";
    }
    "None (Unsupported)"
}

#[allow(clippy::inline_always)]
#[inline(always)]
fn is_avx512_supported() -> bool {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), feature = "std"))]
    {
        std::is_x86_feature_detected!("avx512f")
            && std::is_x86_feature_detected!("avx512bw")
            && std::is_x86_feature_detected!("vaes")
            && std::is_x86_feature_detected!("vpclmulqdq")
    }
    #[cfg(not(feature = "std"))]
    {
        cfg!(target_feature = "avx512f")
            && cfg!(target_feature = "avx512bw")
            && cfg!(target_feature = "vaes")
            && cfg!(target_feature = "vpclmulqdq")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        false
    }
}
