//! Execution Engine
//!
//! CPU dispatch and parallel processing.

pub mod dispatcher;
pub mod parallel;

pub use dispatcher::get_active_backend_name;
