//! Kernel Dispatcher
//!
//! Contains hardware-specific implementations of the Tachyon hash function.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod aesni;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod avx512;
pub mod constants;
pub mod portable;
