//! Streaming Hasher
//!
//! True Merkle Tree incremental hashing with O(log n) memory.
//! Uses zero-copy parallel processing for high throughput.

use crate::engine::{dispatcher::CHUNK_SIZE, parallel::MerkleTree};
use crate::types::CpuFeatureError;

#[cfg(feature = "digest-trait")]
use crypto_common::{Key, KeySizeUser};
#[cfg(feature = "digest-trait")]
use digest::typenum::U32;
#[cfg(feature = "digest-trait")]
use digest::Output;
#[cfg(feature = "digest-trait")]
use digest::{FixedOutput, HashMarker, KeyInit, OutputSizeUser, Reset, Update};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

// =============================================================================
// STREAMING HASHER
// =============================================================================

/// Streaming hasher using Merkle Tree for O(log n) memory.
///
/// Uses zero-copy parallel chunk processing for high throughput.
pub struct TachyonHasher {
    /// Remainder buffer (always < `CHUNK_SIZE` bytes)
    buffer: Vec<u8>,
    /// The Merkle Tree Engine
    tree: MerkleTree,
    /// Total bytes processed
    total_len: u64,
}

impl TachyonHasher {
    // =========================================================================
    // INITIALIZATION
    // =========================================================================

    /// Create new streaming hasher.
    ///
    /// # Errors
    /// Returns `CpuFeatureError` if required CPU features are not available.
    pub fn new() -> Result<Self, CpuFeatureError> {
        Self::new_full(0, 0)
    }

    /// Create new hasher with domain separation.
    ///
    /// # Errors
    /// Returns `CpuFeatureError` if required CPU features are not available.
    pub fn new_with_domain(domain: u64) -> Result<Self, CpuFeatureError> {
        Self::new_full(domain, 0)
    }

    /// Full initialization with domain and seed.
    ///
    /// # Errors
    /// Returns `CpuFeatureError` if required CPU features are not available.
    pub fn new_full(domain: u64, seed: u64) -> Result<Self, CpuFeatureError> {
        Ok(Self {
            buffer: Vec::with_capacity(CHUNK_SIZE),
            tree: MerkleTree::new(domain, seed),
            total_len: 0,
        })
    }

    // =========================================================================
    // STATE MODIFICATION
    // =========================================================================

    /// Add data to the hasher.
    ///
    /// Zero-copy processing: data is hashed directly without buffering
    /// when possible, remainder stored for next update.
    pub fn update(&mut self, data: &[u8]) {
        self.total_len += data.len() as u64;

        // Fast path: if buffer is empty and data is large, process directly
        if self.buffer.is_empty() && data.len() >= CHUNK_SIZE {
            let (_complete_bytes, remainder) = self.process_direct(data);
            if !remainder.is_empty() {
                self.buffer.extend_from_slice(remainder);
            }
            // `process_direct` already updated the tree for the complete parts
            return;
        }

        // Slow path: combine with existing buffer
        self.buffer.extend_from_slice(data);

        // With multithread: batch until we have 2+ chunks for parallel processing
        #[cfg(feature = "multithread")]
        if self.buffer.len() >= CHUNK_SIZE * 2 {
            let complete_bytes = (self.buffer.len() / CHUNK_SIZE) * CHUNK_SIZE;
            let to_process: Vec<u8> = self.buffer.drain(..complete_bytes).collect();
            self.tree.process_slice(&to_process);
        }
        // Without multithread: process as soon as one full chunk is ready
        #[cfg(not(feature = "multithread"))]
        if self.buffer.len() >= CHUNK_SIZE {
            let complete_bytes = (self.buffer.len() / CHUNK_SIZE) * CHUNK_SIZE;
            let to_process: Vec<u8> = self.buffer.drain(..complete_bytes).collect();
            self.tree.process_slice(&to_process);
        }
    }

    /// Set optional key for MAC.
    pub const fn set_key(&mut self, key: &[u8; crate::kernels::constants::HASH_SIZE]) {
        self.tree.set_key(key);
    }

    /// Process data directly from an input slice (zero-copy fast path).
    /// Returns (`processed_bytes`, `remainder_slice`).
    fn process_direct<'a>(&mut self, data: &'a [u8]) -> (usize, &'a [u8]) {
        let complete_chunks = data.len() / CHUNK_SIZE;
        let complete_bytes = complete_chunks * CHUNK_SIZE;

        if complete_bytes > 0 {
            self.tree.process_slice(&data[..complete_bytes]);
        }

        (complete_bytes, &data[complete_bytes..])
    }

    /// Finalize and return hash.
    #[must_use]
    pub fn finalize(mut self) -> [u8; crate::kernels::constants::HASH_SIZE] {
        // Process any remaining complete chunks
        if self.buffer.len() >= CHUNK_SIZE {
            let complete_bytes = (self.buffer.len() / CHUNK_SIZE) * CHUNK_SIZE;
            let to_process: Vec<u8> = self.buffer.drain(..complete_bytes).collect();
            self.tree.process_slice(&to_process);
        }

        let len = self.total_len;
        self.tree.finalize(&self.buffer, len)
    }

    /// Reset hasher for reuse.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.tree.reset();
        self.total_len = 0;
    }
}

// =============================================================================
// TRAIT IMPL
// =============================================================================

impl Default for TachyonHasher {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| panic!("{}", e))
    }
}

#[cfg(feature = "digest-trait")]
impl OutputSizeUser for TachyonHasher {
    type OutputSize = U32;
}

#[cfg(feature = "digest-trait")]
impl KeySizeUser for TachyonHasher {
    type KeySize = U32;
}

#[cfg(feature = "digest-trait")]
impl Update for TachyonHasher {
    fn update(&mut self, data: &[u8]) {
        self.update(data);
    }
}

#[cfg(feature = "digest-trait")]
impl FixedOutput for TachyonHasher {
    fn finalize_into(self, out: &mut Output<Self>) {
        let res = self.finalize();
        out.copy_from_slice(&res);
    }
}

#[cfg(feature = "digest-trait")]
impl Reset for TachyonHasher {
    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(feature = "digest-trait")]
impl HashMarker for TachyonHasher {}

#[cfg(feature = "digest-trait")]
impl KeyInit for TachyonHasher {
    #[allow(clippy::expect_used)]
    fn new(key: &Key<Self>) -> Self {
        // Safe conversion since KeySize is U32 (32 bytes)
        let k: [u8; crate::kernels::constants::HASH_SIZE] =
            key.as_slice().try_into().expect("Key length mismatch");
        let mut hasher = Self::new().expect("Hardware support required");
        hasher.set_key(&k);
        hasher
    }
}

impl Clone for TachyonHasher {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            tree: self.tree.clone(),
            total_len: self.total_len,
        }
    }
}
