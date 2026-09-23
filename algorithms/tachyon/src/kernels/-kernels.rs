//! Hash Kernels
//!
//! Contains hardware-specific implementations of the Tachyon hash function.

// =============================================================================
// MODULES
// =============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[path = "aesni/-aesni.rs"]
pub mod aesni;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[path = "avx512/-avx512.rs"]
pub mod avx512;
#[path = "constants.rs"]
pub mod constants;
#[path = "portable/-portable.rs"]
pub mod portable;
