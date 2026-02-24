//! AES-NI Kernel Module
//!
//! Low-latency hash implementation using AES-NI instructions for small inputs.
//! Optimized for 4-Way ILP (256-byte chunks) using 16 accumulators.

// =============================================================================
// MODULES
// =============================================================================

mod compress;
mod finalize;
pub(crate) mod short;
mod state;

// =============================================================================
// EXPORTS
// =============================================================================

// Re-export public API
pub use finalize::oneshot;
pub use state::AesNiState;
