//! AVX-512 Kernel
//!
//! High-performance hash implementation using AVX-512 + VAES instructions.

// =============================================================================
// MODULES
// =============================================================================

#[path = "compress.rs"]
mod compress;
#[path = "finalize.rs"]
mod finalize;
#[path = "state.rs"]
mod state;

// =============================================================================
// EXPORTS
// =============================================================================

pub use finalize::oneshot;
pub use state::Avx512State;
