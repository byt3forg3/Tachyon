//! AES-NI Kernel
//!
//! Low-latency hash implementation using AES-NI instructions for small inputs.
//! Optimized for 4-Way ILP (256-byte chunks) using 16 accumulators.

// =============================================================================
// MODULES
// =============================================================================

#[path = "compress.rs"]
mod compress;
#[path = "finalize.rs"]
mod finalize;
#[path = "short.rs"]
pub(crate) mod short;
#[path = "state.rs"]
mod state;

// =============================================================================
// EXPORTS
// =============================================================================

// Re-export public API
pub use finalize::oneshot;
pub use state::AesNiState;
