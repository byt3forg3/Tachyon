//! AVX-512 Kernel Module
//!
//! High-performance hash implementation using AVX-512 + VAES instructions.

mod compress;
mod finalize;
mod state;

// Re-export public API
pub use finalize::oneshot;
pub use state::Avx512State;
