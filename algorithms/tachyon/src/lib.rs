#![cfg_attr(not(feature = "std"), no_std)]

//! # Tachyon
//!
//! High-performance, cryptographically hardened hash function.
//! Accelerated by AVX-512 + VAES.

//! # Usage
//! ```rust
//! use tachyon;
//!
//! // 1. Fast Hashing
//! let hash = tachyon::hash(b"Performance Matters");
//! println!("{:x?}", hash);
//!
//! // 2. Secure Verification
//! let valid = tachyon::verify(b"Performance Matters", &hash);
//! assert!(valid);
//!
//! // 3. Streaming (Big Data / Files)
//! use tachyon::Hasher;
//!
//! let mut hasher = Hasher::new()?;
//! hasher.update(b"Chunk 1");
//! hasher.update(b"Chunk 2");
//! let hash = hasher.finalize();
//! # Ok::<(), tachyon::CpuFeatureError>(())
//! ```

// =============================================================================
// MODULES
// =============================================================================

#[cfg(not(feature = "std"))]
extern crate alloc;

mod engine;
#[cfg(feature = "std")]
mod ffi;
// Re-export internal kernels for benchmarking/testing if needed, but hide from docs
#[doc(hidden)]
pub mod kernels; // Public for test/example use only
mod oneshot;
mod streaming;
pub(crate) mod types;

// =============================================================================
// EXPORTS
// =============================================================================

#[cfg(feature = "digest-trait")]
pub use digest;
pub use oneshot::{
    derive_key, hash, hash_full, hash_keyed, hash_parallel, hash_seeded, hash_with_domain, verify,
    verify_mac,
};
pub use streaming::TachyonHasher as Hasher;
pub use types::{custom_domain, CpuFeatureError, TachyonDomain};

/// Returns the name of the hardware backend currently in use.
#[must_use]
pub fn active_backend() -> &'static str {
    engine::get_active_backend_name()
}
