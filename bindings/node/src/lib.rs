//! Tachyon Hash Function - Node.js Bindings
//!
//! High-performance cryptographically hardened hash using AVX-512 + VAES.
//!
//! Example (JavaScript):
//! ```javascript
//! const tachyon = require('tachyon');
//!
//! const hash = tachyon.hash(Buffer.from('Hello, World!'));
//!
//! // Streaming
//! const hasher = new tachyon.Hasher();
//! hasher.update(Buffer.from('chunk 1'));
//! hasher.update(Buffer.from('chunk 2'));
//! const result = hasher.finalize();
//! ```

#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Mutex;

// =============================================================================
// DOMAIN CONSTANTS (aligned with Rust definitions)
// =============================================================================

#[napi]
pub const DOMAIN_GENERIC: u8 = 0;

#[napi]
pub const DOMAIN_FILE_CHECKSUM: u8 = 1;

#[napi]
pub const DOMAIN_KEY_DERIVATION: u8 = 2;

#[napi]
pub const DOMAIN_MESSAGE_AUTH: u8 = 3;

#[napi]
pub const DOMAIN_DATABASE_INDEX: u8 = 4;

#[napi]
pub const DOMAIN_CONTENT_ADDRESSED: u8 = 5;

// =============================================================================
// ONE-SHOT API
// =============================================================================

/// Compute the Tachyon hash of input data.
///
/// @param input - Input data as Buffer
/// @returns 32-byte hash as Buffer
#[napi]
pub fn hash(input: Buffer) -> Buffer {
    let data: &[u8] = input.as_ref();
    let hash = tachyon::hash(data);
    Buffer::from(hash.as_slice())
}

/// Compute Tachyon hash with a seed.
///
/// @param input - Input data as Buffer
/// @param seed - 64-bit seed as BigInt
/// @returns 32-byte hash as Buffer
#[napi]
pub fn hash_seeded(input: Buffer, seed: BigInt) -> Buffer {
    let data: &[u8] = input.as_ref();
    let (_, s, _) = seed.get_u64();
    let hash = tachyon::hash_seeded(data, s);
    Buffer::from(hash.as_slice())
}

/// Verify data matches expected hash in constant time.
///
/// This function is timing-attack resistant.
///
/// @param input - Input data as Buffer
/// @param expectedHash - Expected 32-byte hash as Buffer
/// @returns true if hash matches, false otherwise
#[napi]
pub fn verify(input: Buffer, expected_hash: Buffer) -> bool {
    let data: &[u8] = input.as_ref();
    let hash_slice: &[u8] = expected_hash.as_ref();

    if hash_slice.len() != 32 {
        return false;
    }

    let mut fixed_hash = [0u8; 32];
    fixed_hash.copy_from_slice(hash_slice);

    tachyon::verify(data, &fixed_hash)
}

/// Compute hash with domain separation.
///
/// @param input - Input data as Buffer
/// @param domain - Domain value (0-5)
/// @returns 32-byte hash as Buffer
#[napi]
pub fn hash_with_domain(input: Buffer, domain: u8) -> Result<Buffer> {
    if domain > 5 {
        return Err(napi::Error::from_reason("Domain must be 0-5"));
    }
    let data: &[u8] = input.as_ref();
    let tachyon_domain = match domain {
        0 => tachyon::TachyonDomain::Generic,
        1 => tachyon::TachyonDomain::FileChecksum,
        2 => tachyon::TachyonDomain::KeyDerivation,
        3 => tachyon::TachyonDomain::MessageAuth,
        4 => tachyon::TachyonDomain::DatabaseIndex,
        5 => tachyon::TachyonDomain::ContentAddressed,
        _ => unreachable!(),
    };
    let hash = tachyon::hash_with_domain(data, tachyon_domain);
    Ok(Buffer::from(hash.as_slice()))
}

/// Compute keyed hash (MAC).
///
/// @param input - Input data as Buffer
/// @param key - 32-byte key as Buffer
/// @returns 32-byte MAC as Buffer
#[napi]
pub fn hash_keyed(input: Buffer, key: Buffer) -> Result<Buffer> {
    let data: &[u8] = input.as_ref();
    let key_slice: &[u8] = key.as_ref();

    if key_slice.len() != 32 {
        return Err(napi::Error::from_reason("Key must be exactly 32 bytes"));
    }

    let mut fixed_key = [0u8; 32];
    fixed_key.copy_from_slice(key_slice);

    let mac = tachyon::hash_keyed(data, &fixed_key);
    Ok(Buffer::from(mac.as_slice()))
}

/// Verify keyed hash (MAC) in constant time.
///
/// @param input - Input data as Buffer
/// @param key - 32-byte key as Buffer
/// @param expectedMac - Expected 32-byte MAC as Buffer
/// @returns true if MAC matches, false otherwise
#[napi]
pub fn verify_mac(input: Buffer, key: Buffer, expected_mac: Buffer) -> Result<bool> {
    let data: &[u8] = input.as_ref();
    let key_slice: &[u8] = key.as_ref();
    let mac_slice: &[u8] = expected_mac.as_ref();

    if key_slice.len() != 32 {
        return Err(napi::Error::from_reason("Key must be exactly 32 bytes"));
    }
    if mac_slice.len() != 32 {
        return Err(napi::Error::from_reason(
            "Expected MAC must be exactly 32 bytes",
        ));
    }

    let mut fixed_key = [0u8; 32];
    let mut fixed_mac = [0u8; 32];
    fixed_key.copy_from_slice(key_slice);
    fixed_mac.copy_from_slice(mac_slice);

    Ok(tachyon::verify_mac(data, &fixed_key, &fixed_mac))
}

/// Derive cryptographic key from material.
///
/// @param context - Context string as Buffer
/// @param keyMaterial - 32-byte key material as Buffer
/// @returns 32-byte derived key as Buffer
#[napi]
pub fn derive_key(context: Buffer, key_material: Buffer) -> Result<Buffer> {
    let ctx: &[u8] = context.as_ref();
    let material_slice: &[u8] = key_material.as_ref();

    if material_slice.len() != 32 {
        return Err(napi::Error::from_reason(
            "Key material must be exactly 32 bytes",
        ));
    }

    // Convert context bytes to UTF-8 string
    let ctx_str = std::str::from_utf8(ctx)
        .map_err(|_| napi::Error::from_reason("Context must be valid UTF-8"))?;

    let mut fixed_material = [0u8; 32];
    fixed_material.copy_from_slice(material_slice);

    let derived = tachyon::derive_key(ctx_str, &fixed_material);
    Ok(Buffer::from(derived.as_slice()))
}

// =============================================================================
// STREAMING API
// =============================================================================

/// Streaming hasher for large data.
///
/// @example
/// const hasher = new tachyon.Hasher();
/// hasher.update(Buffer.from('chunk 1'));
/// hasher.update(Buffer.from('chunk 2'));
/// const hash = hasher.finalize();
#[napi]
pub struct Hasher {
    inner: Mutex<Option<tachyon::Hasher>>,
}

#[napi]
impl Hasher {
    /// Create a new streaming hasher.
    ///
    /// @param domain - Optional domain value (0-5) for domain separation
    /// @param seed - Optional 64-bit seed as BigInt
    #[napi(constructor)]
    pub fn new(domain: Option<u8>, seed: Option<BigInt>) -> Result<Self> {
        let s = if let Some(big_int) = seed {
            let (_, val, _) = big_int.get_u64();
            val
        } else {
            0
        };

        if let Some(d) = domain {
            if d > 5 {
                return Err(napi::Error::from_reason("Domain must be 0-5"));
            }
            // Use new_full if seed is present or just to be safe (0 seed is default)
            let inner = tachyon::Hasher::new_full(d as u64, s)
                .map_err(|e| napi::Error::from_reason(format!("Failed to create hasher: {}", e)))?;
            Ok(Self {
                inner: Mutex::new(Some(inner)),
            })
        } else {
            // No domain provided.
            if s != 0 {
                let inner = tachyon::Hasher::new_full(0, s).map_err(|e| {
                    napi::Error::from_reason(format!("Failed to create hasher: {}", e))
                })?;
                Ok(Self {
                    inner: Mutex::new(Some(inner)),
                })
            } else {
                let inner = tachyon::Hasher::new().map_err(|e| {
                    napi::Error::from_reason(format!("Failed to create hasher: {}", e))
                })?;
                Ok(Self {
                    inner: Mutex::new(Some(inner)),
                })
            }
        }
    }

    /// Add data to the hasher.
    ///
    /// @param data - Input data as Buffer
    #[napi]
    pub fn update(&self, data: Buffer) -> Result<()> {
        let mut guard = self.inner.lock().unwrap();
        let hasher = guard
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Hasher already finalized"))?;
        hasher.update(data.as_ref());
        Ok(())
    }

    /// Finalize and return the hash.
    ///
    /// The hasher cannot be used after this call.
    ///
    /// @returns 32-byte hash as Buffer
    #[napi]
    pub fn finalize(&self) -> Result<Buffer> {
        let mut guard = self.inner.lock().unwrap();
        let hasher = guard
            .take()
            .ok_or_else(|| napi::Error::from_reason("Hasher already finalized"))?;
        let hash = hasher.finalize();
        Ok(Buffer::from(hash.as_slice()))
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new(None, None).expect("Failed to create hasher")
    }
}
