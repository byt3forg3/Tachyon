//! Execution Engine
//!
//! CPU dispatch and parallel processing.

// =============================================================================
// MODULES
// =============================================================================

#[path = "dispatcher.rs"]
pub mod dispatcher;
#[path = "parallel.rs"]
pub mod parallel;

// =============================================================================
// EXPORTS
// =============================================================================

pub use dispatcher::get_active_backend_name;
