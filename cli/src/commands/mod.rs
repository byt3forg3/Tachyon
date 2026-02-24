//! CLI Commands
//!
//! All tachyon CLI commands organized as separate modules.

mod check;
mod hash;

pub use check::check_mode;
pub use hash::{hash_files, Algorithm};
