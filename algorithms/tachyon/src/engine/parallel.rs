//! Parallel Execution Engine (Merkle Tree)
//!
//! This module contains the core Merkle Tree logic used by both the streaming hasher
//! and the one-shot parallel hasher. It handles:
//! 1. State management (sparse stack)
//! 2. Leaf hashing (parallel via Rayon if `multithread` feature enabled, otherwise serial)
//! 3. Tree reduction

use crate::engine::dispatcher::{self, CHUNK_SIZE};
use crate::types::KernelFn;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

// =============================================================================
// CONSTANTS
// =============================================================================

// Internal Merkle tree domains
pub use crate::kernels::constants::{DOMAIN_LEAF, DOMAIN_NODE};

// =============================================================================
// MERKLE TREE ENGINE
// =============================================================================

/// Core Merkle Tree Engine (Sparse Stack)
#[derive(Clone)]
pub struct MerkleTree {
    stack: Vec<Option<[u8; 32]>>, // Sparse representation
    kernel: KernelFn,             // Function pointer
    domain: u64,                  // Finalization domain
    seed: u64,                    // Randomization seed
    key: Option<[u8; 32]>,        // Optional MAC key
}

impl MerkleTree {
    /// Create a new Merkle Tree engine.
    pub fn new(domain: u64, seed: u64) -> Self {
        Self {
            stack: Vec::with_capacity(16),
            kernel: dispatcher::get_best_kernel(),
            domain,
            seed,
            key: None,
        }
    }

    /// Process data slice (must be multiple of `CHUNK_SIZE`).
    pub fn process_slice(&mut self, data: &[u8]) {
        debug_assert!(data.len().is_multiple_of(CHUNK_SIZE));

        // 1. Parallel Leaf Hashing
        let leaves: Vec<[u8; 32]> = data.process_chunks(CHUNK_SIZE, |chunk| {
            (self.kernel)(chunk, DOMAIN_LEAF, self.seed, self.key.as_ref())
        });

        // 2. Serial Tree Update
        for leaf in leaves {
            self.push_leaf(leaf);
        }
    }

    /// Push leaf hash into stack, collapsing nodes if needed.
    pub fn push_leaf(&mut self, mut hash: [u8; 32]) {
        let mut level = 0;
        loop {
            while self.stack.len() <= level {
                self.stack.push(None);
            }

            match self.stack[level].take() {
                None => {
                    self.stack[level] = Some(hash);
                    break;
                }
                Some(sibling) => {
                    hash =
                        compress_nodes(&sibling, &hash, self.kernel, self.seed, self.key.as_ref());
                    level += 1;
                }
            }
        }
    }

    /// Finalize tree, processing remainder and returning root hash.
    pub fn finalize(mut self, remainder: &[u8], total_len: u64) -> [u8; 32] {
        // Optimization: Small input (single chunk) -> direct hash
        if self.stack.is_empty() {
            return (self.kernel)(remainder, self.domain, self.seed, self.key.as_ref());
        }

        // Process remainder as final leaf
        if !remainder.is_empty() {
            let leaf_hash = (self.kernel)(remainder, DOMAIN_LEAF, self.seed, self.key.as_ref());
            self.push_leaf(leaf_hash);
        }

        // Collapse stack to root
        let mut result: Option<[u8; 32]> = None;
        for node in self.stack.into_iter().flatten() {
            result = Some(match result {
                None => node,
                Some(right) => {
                    compress_nodes(&node, &right, self.kernel, self.seed, self.key.as_ref())
                }
            });
        }

        let tree_root =
            result.unwrap_or_else(|| (self.kernel)(&[], 0, self.seed, self.key.as_ref()));

        // Finalization: Mix domain + total length (Length Commitment)
        let mut buf = [0u8; 48];
        buf[0..32].copy_from_slice(&tree_root);
        buf[32..40].copy_from_slice(&self.domain.to_le_bytes());
        buf[40..48].copy_from_slice(&total_len.to_le_bytes());
        (self.kernel)(&buf, 0, self.seed, self.key.as_ref())
    }

    /// Set optional key for MAC.
    pub const fn set_key(&mut self, key: &[u8; 32]) {
        self.key = Some(*key);
    }

    /// Reset the tree (clear stack), keeping configuration (domain/seed).
    pub fn reset(&mut self) {
        self.stack.clear();
    }
}

/// Helper for feature-agnostic chunk processing
trait ChunkProcessor {
    fn process_chunks<F, R>(self, chunk_size: usize, f: F) -> Vec<R>
    where
        F: Fn(&[u8]) -> R + Sync + Send,
        R: Send;
}

impl ChunkProcessor for &[u8] {
    fn process_chunks<F, R>(self, chunk_size: usize, f: F) -> Vec<R>
    where
        F: Fn(&[u8]) -> R + Sync + Send,
        R: Send,
    {
        #[cfg(feature = "multithread")]
        {
            use rayon::prelude::*;
            self.par_chunks(chunk_size).map(f).collect()
        }
        #[cfg(not(feature = "multithread"))]
        {
            self.chunks(chunk_size).map(f).collect()
        }
    }
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/// Compress two child nodes into a parent node.
#[inline]
fn compress_nodes(
    left: &[u8; 32],
    right: &[u8; 32],
    kernel: KernelFn,
    seed: u64,
    key: Option<&[u8; 32]>,
) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[0..32].copy_from_slice(left);
    buf[32..64].copy_from_slice(right);
    kernel(&buf, DOMAIN_NODE, seed, key)
}
