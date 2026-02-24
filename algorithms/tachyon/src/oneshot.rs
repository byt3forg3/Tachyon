//! Public API Layer
//!
use crate::engine::dispatcher;
use crate::types::TachyonDomain;
use subtle::ConstantTimeEq;

// Threshold for switching to the Merkle-tree path (256 KB)
const CHUNK_SIZE: usize = crate::engine::dispatcher::CHUNK_SIZE;

// =============================================================================
// GENERIC HASHING
// =============================================================================

/// Compute Tachyon hash.
///
/// **Requires:** AVX-512F + VAES or AES-NI. Panics if unsupported.
///
/// # Example
/// ```rust
/// let hash = tachyon::hash(b"Performance");
/// ```
#[must_use]
#[inline]
pub fn hash(input: &[u8]) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash_full(input, TachyonDomain::Generic, 0)
}

/// Compute hash using multiple threads (explicitly).
///
/// Aliases to `hash()` which automatically selects parallel execution
/// for large inputs. Kept for API compatibility.
#[must_use]
#[inline]
pub fn hash_parallel(input: &[u8]) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash(input)
}

/// Compute Tachyon hash with a seed.
///
/// Used for `SMHasher` compatibility and randomized hashing.
#[must_use]
#[inline]
pub fn hash_seeded(input: &[u8], seed: u64) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash_full(input, TachyonDomain::Generic, seed)
}

// =============================================================================
// VERIFICATION
// =============================================================================

/// Verify hash in constant time (timing attack resistant).
///
/// Use for: passwords, API keys, integrity checks.
///
/// # Example
/// ```rust
/// let data = b"Secure Data";
/// let hash = tachyon::hash(data);
/// assert!(tachyon::verify(data, &hash));
/// ```
#[must_use]
pub fn verify(input: &[u8], expected: &[u8; crate::kernels::constants::HASH_SIZE]) -> bool {
    let computed = hash(input);
    computed.ct_eq(expected).into()
}

// =============================================================================
// DOMAIN SEPARATION
// =============================================================================

/// Hash with domain separation.
///
/// Prevents cross-protocol attacks by ensuring `Hash(A) != Hash(A | Domain)`.
///
/// # Example
/// ```rust
/// use tachyon::{hash_with_domain, TachyonDomain};
///
/// let file_hash = hash_with_domain(b"data", TachyonDomain::FileChecksum);
/// let db_hash = hash_with_domain(b"data", TachyonDomain::DatabaseIndex);
/// assert_ne!(file_hash, db_hash);
/// ```
#[must_use]
#[inline]
pub fn hash_with_domain(
    input: &[u8],
    domain: TachyonDomain,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash_full(input, domain, 0)
}

/// Full hashing API with domain and seed.
#[must_use]
#[inline]
pub fn hash_full(
    input: &[u8],
    domain: TachyonDomain,
    seed: u64,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash_full_internal(input, domain.to_u64(), None, seed)
}

// =============================================================================
// KEYED HASHING (MAC)
// =============================================================================

/// Hash with 256-bit key for message authentication.
///
/// # Example
/// ```rust
/// use tachyon::hash_keyed;
///
/// let key = [42u8; 32];
/// let mac = hash_keyed(b"message", &key);
/// assert!(tachyon::verify_mac(b"message", &key, &mac));
/// ```
#[must_use]
pub fn hash_keyed(
    input: &[u8],
    key: &[u8; crate::kernels::constants::HASH_SIZE],
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash_full_internal(input, TachyonDomain::MessageAuth.to_u64(), Some(*key), 0)
}

/// Verify MAC in constant time.
#[must_use]
pub fn verify_mac(
    input: &[u8],
    key: &[u8; crate::kernels::constants::HASH_SIZE],
    expected: &[u8; crate::kernels::constants::HASH_SIZE],
) -> bool {
    let computed = hash_keyed(input, key);
    computed.ct_eq(expected).into()
}

// =============================================================================
// KEY DERIVATION
// =============================================================================

/// Derive 256-bit key from master key using context string.
///
/// Uses domain separation specifically for KDF.
///
/// # Example
/// ```rust
/// use tachyon::derive_key;
///
/// let master = [0u8; 32];
/// let session_key = derive_key("session-2024", &master);
/// let db_key = derive_key("database-encryption", &master);
/// assert_ne!(session_key, db_key);
/// ```
#[must_use]
pub fn derive_key(
    context: &str,
    master_key: &[u8; crate::kernels::constants::HASH_SIZE],
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    hash_full_internal(
        context.as_bytes(),
        TachyonDomain::KeyDerivation.to_u64(),
        Some(*master_key),
        0,
    )
}

// =============================================================================
// INTERNAL / FALLBACK
// =============================================================================

/// Internal: Full entry point with domain, key, and seed.
#[allow(unsafe_code)]
pub fn hash_full_internal(
    input: &[u8],
    domain_id: u64,
    key: Option<[u8; crate::kernels::constants::HASH_SIZE]>,
    seed: u64,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    // Parallel/Merkle path for large inputs (>= CHUNK_SIZE)
    if input.len() >= CHUNK_SIZE {
        use crate::Hasher;
        #[allow(clippy::expect_used)] // Infallible API; panics if CPU features missing
        let mut hasher = Hasher::new_full(domain_id, seed).expect("CPU features missing");
        if let Some(k) = key {
            hasher.set_key(&k);
        }
        hasher.update(input);
        return hasher.finalize();
    }

    // For inputs < CHUNK_SIZE, we use the dispatcher's wrapper directly
    // to ensure bit-identical results with the streaming hasher (which uses the kernel).
    let kernel = dispatcher::get_best_kernel();
    kernel(input, domain_id, seed, key.as_ref())
}
