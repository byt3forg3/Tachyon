//! Bolero Fuzz Tests
//!
//! These tests can be run as property tests via `cargo test`
//! or as full fuzz targets via `cargo bolero test [target_name]`.

/// Fuzz test module
#[cfg(test)]
mod fuzz {
    mod parallel;
    mod security;
    mod streaming;
    mod verification;
}
