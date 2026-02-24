//! C-API Bindings
//!
//! Exposes Tachyon to C/C++ via FFI with pointer safety and panic boundaries.

#![allow(unsafe_code)]

use crate::oneshot;

use std::ptr;
use std::slice;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Domain constants for C API
pub const _TACHYON_DOMAIN_GENERIC: u64 = 0;
pub const _TACHYON_DOMAIN_FILE_CHECKSUM: u64 = 1;
pub const _TACHYON_DOMAIN_KEY_DERIVATION: u64 = 2;
pub const _TACHYON_DOMAIN_MESSAGE_AUTH: u64 = 3;
pub const _TACHYON_DOMAIN_DATABASE_INDEX: u64 = 4;
pub const _TACHYON_DOMAIN_CONTENT_ADDRESSED: u64 = 5;

// =============================================================================
// ONE-SHOT API
// =============================================================================

/// Compute Tachyon hash.
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes (may be null if `input_len == 0`)
/// - `output_ptr` must be valid for 32 writable bytes
///
/// # Returns
/// - `0`: Success
/// - `-1`: Null pointer
/// - `-2`: Panic (e.g. missing CPU features)
#[no_mangle]
pub unsafe extern "C" fn tachyon_hash(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
) -> i32 {
    if input_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let input = slice::from_raw_parts(input_ptr, input_len);
        let hash = oneshot::hash(input);
        std::ptr::copy_nonoverlapping(hash.as_ptr(), output_ptr, 32);
    });

    match result {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Compute Tachyon hash with a seed.
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes
/// - `output_ptr` must be valid for 32 writable bytes
///
/// # Returns
/// - `0`: Success
/// - `-1`: Null pointer
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_hash_seeded(
    input_ptr: *const u8,
    input_len: usize,
    seed: u64,
    output_ptr: *mut u8,
) -> i32 {
    if input_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let input = slice::from_raw_parts(input_ptr, input_len);
        let hash = oneshot::hash_seeded(input, seed);
        std::ptr::copy_nonoverlapping(hash.as_ptr(), output_ptr, 32);
    });

    match result {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Full hashing API with domain, seed, and optional key.
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes
/// - `output_ptr` must be valid for 32 writable bytes
/// - `key_ptr`, if non-null, must point to exactly 32 bytes
///
/// # Returns
/// - `0`: Success
/// - `-1`: Null pointer
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_hash_full(
    input_ptr: *const u8,
    input_len: usize,
    domain: u64,
    seed: u64,
    key_ptr: *const u8, // NULL for unkeyed
    output_ptr: *mut u8,
) -> i32 {
    if input_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let input = slice::from_raw_parts(input_ptr, input_len);
        let key = if key_ptr.is_null() {
            None
        } else {
            let k_slice = slice::from_raw_parts(key_ptr, 32);
            let mut k = [0u8; crate::kernels::constants::HASH_SIZE];
            k.copy_from_slice(k_slice);
            Some(k)
        };

        let hash = oneshot::hash_full_internal(input, domain, key, seed);
        std::ptr::copy_nonoverlapping(hash.as_ptr(), output_ptr, 32);
    });

    match result {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Compute hash with domain separation.
///
/// Convenience wrapper around `tachyon_hash_full` (seed=0, key=NULL).
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes
/// - `output_ptr` must be valid for 32 writable bytes
///
/// # Returns
/// - `0`: Success
/// - `-1`: Null pointer
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_hash_with_domain(
    input_ptr: *const u8,
    input_len: usize,
    domain: u64,
    output_ptr: *mut u8,
) -> i32 {
    tachyon_hash_full(
        input_ptr,
        input_len,
        domain,
        0,
        std::ptr::null(),
        output_ptr,
    )
}

/// Verify data matches expected hash in constant time.
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes
/// - `hash_ptr` must point to exactly 32 bytes
///
/// # Returns
/// - `1`: Match
/// - `0`: No match
/// - `-1`: Null pointer
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_verify(
    input_ptr: *const u8,
    input_len: usize,
    hash_ptr: *const u8,
) -> i32 {
    if input_ptr.is_null() || hash_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let input = slice::from_raw_parts(input_ptr, input_len);
        let hash_slice = slice::from_raw_parts(hash_ptr, 32);
        let mut hash = [0u8; crate::kernels::constants::HASH_SIZE];
        hash.copy_from_slice(hash_slice);
        oneshot::verify(input, &hash)
    });

    match result {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -2,
    }
}

/// Compute keyed hash (MAC).
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes
/// - `key_ptr` must point to exactly 32 bytes
/// - `output_ptr` must be valid for 32 writable bytes
///
/// # Returns
/// - `0`: Success
/// - `-1`: Null pointer
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_hash_keyed(
    input_ptr: *const u8,
    input_len: usize,
    key_ptr: *const u8,
    output_ptr: *mut u8,
) -> i32 {
    if input_ptr.is_null() || key_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let input = slice::from_raw_parts(input_ptr, input_len);
        let key_slice = slice::from_raw_parts(key_ptr, 32);
        let mut key = [0u8; crate::kernels::constants::HASH_SIZE];
        key.copy_from_slice(key_slice);
        let mac = oneshot::hash_keyed(input, &key);
        std::ptr::copy_nonoverlapping(mac.as_ptr(), output_ptr, 32);
    });

    match result {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Verify keyed hash (MAC) in constant time.
///
/// # Safety
/// - `input_ptr` must be valid for `input_len` bytes
/// - `key_ptr` must point to exactly 32 bytes
/// - `mac_ptr` must point to exactly 32 bytes
///
/// # Returns
/// - `1`: Match
/// - `0`: No match
/// - `-1`: Null pointer
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_verify_mac(
    input_ptr: *const u8,
    input_len: usize,
    key_ptr: *const u8,
    mac_ptr: *const u8,
) -> i32 {
    if input_ptr.is_null() || key_ptr.is_null() || mac_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let input = slice::from_raw_parts(input_ptr, input_len);
        let key_slice = slice::from_raw_parts(key_ptr, 32);
        let mac_slice = slice::from_raw_parts(mac_ptr, 32);
        let mut key = [0u8; crate::kernels::constants::HASH_SIZE];
        let mut mac = [0u8; crate::kernels::constants::HASH_SIZE];
        key.copy_from_slice(key_slice);
        mac.copy_from_slice(mac_slice);
        oneshot::verify_mac(input, &key, &mac)
    });

    match result {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -2,
    }
}

/// Derive cryptographic key from material.
///
/// # Safety
/// - `context_ptr` must be valid for `context_len` bytes of valid UTF-8
/// - `key_material_ptr` must point to exactly 32 bytes
/// - `output_ptr` must be valid for 32 writable bytes
///
/// # Returns
/// - `0`: Success
/// - `-1`: Null pointer or invalid UTF-8 context
/// - `-2`: Panic
#[no_mangle]
pub unsafe extern "C" fn tachyon_derive_key(
    context_ptr: *const u8,
    context_len: usize,
    key_material_ptr: *const u8,
    output_ptr: *mut u8,
) -> i32 {
    if context_ptr.is_null() || key_material_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(|| {
        let context_bytes = slice::from_raw_parts(context_ptr, context_len);
        let ctx_str = std::str::from_utf8(context_bytes).ok()?;
        let material_slice = slice::from_raw_parts(key_material_ptr, 32);
        let mut material = [0u8; crate::kernels::constants::HASH_SIZE];
        material.copy_from_slice(material_slice);
        let derived = oneshot::derive_key(ctx_str, &material);
        std::ptr::copy_nonoverlapping(derived.as_ptr(), output_ptr, 32);
        Some(())
    });

    match result {
        Ok(Some(())) => 0,
        Ok(None) => -1,
        Err(_) => -2,
    }
}

// =============================================================================
// STREAMING API
// =============================================================================

/// Opaque hasher handle for C.
pub struct TachyonHasherPtr(crate::streaming::TachyonHasher);

/// Create new hasher. Returns NULL if CPU unsupported.
/// Caller must free with `tachyon_hasher_free`.
#[no_mangle]
pub unsafe extern "C" fn tachyon_hasher_new() -> *mut TachyonHasherPtr {
    let Ok(hasher) = crate::streaming::TachyonHasher::new() else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(TachyonHasherPtr(hasher)))
}

/// Create new hasher with domain separation. Returns NULL if CPU unsupported.
/// Caller must free with `tachyon_hasher_free`.
#[no_mangle]
pub unsafe extern "C" fn tachyon_hasher_new_with_domain(domain: u64) -> *mut TachyonHasherPtr {
    let Ok(hasher) = crate::streaming::TachyonHasher::new_with_domain(domain) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(TachyonHasherPtr(hasher)))
}

/// Create new hasher with seed. Returns NULL if CPU unsupported.
/// Caller must free with `tachyon_hasher_free`.
#[no_mangle]
pub unsafe extern "C" fn tachyon_hasher_new_seeded(seed: u64) -> *mut TachyonHasherPtr {
    let Ok(hasher) = crate::streaming::TachyonHasher::new_full(0, seed) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(TachyonHasherPtr(hasher)))
}

/// Feed data into the hasher.
///
/// # Safety
/// - `state_ptr` must be a valid pointer obtained from `tachyon_hasher_new*`
/// - `data_ptr` must be valid for `len` bytes
#[no_mangle]
pub unsafe extern "C" fn tachyon_hasher_update(
    state_ptr: *mut TachyonHasherPtr,
    data_ptr: *const u8,
    len: usize,
) {
    if state_ptr.is_null() || data_ptr.is_null() {
        return;
    }
    let hasher = &mut (*state_ptr).0;
    let data = slice::from_raw_parts(data_ptr, len);
    hasher.update(data);
}

/// Finalize and write hash. Frees the hasher automatically — do not call `tachyon_hasher_free` after this.
///
/// # Safety
/// - `state_ptr` must be a valid pointer obtained from `tachyon_hasher_new*`
/// - `out_ptr` must be valid for 32 writable bytes
#[no_mangle]
pub unsafe extern "C" fn tachyon_hasher_finalize(
    state_ptr: *mut TachyonHasherPtr,
    out_ptr: *mut u8,
) {
    if state_ptr.is_null() || out_ptr.is_null() {
        return;
    }
    let ptr = Box::from_raw(state_ptr);
    let hash = ptr.0.finalize();
    ptr::copy_nonoverlapping(hash.as_ptr(), out_ptr, 32);
}

/// Free hasher without finalizing.
///
/// # Safety
/// - `state_ptr` must be a valid pointer obtained from `tachyon_hasher_new*`, or null
#[no_mangle]
pub unsafe extern "C" fn tachyon_hasher_free(state_ptr: *mut TachyonHasherPtr) {
    if !state_ptr.is_null() {
        let _ = Box::from_raw(state_ptr);
    }
}

/// Get the name of the active backend.
///
/// # Returns
/// A pointer to a static, null-terminated C string (e.g. `"AVX-512"`). Must NOT be freed by the caller.
///
/// # Safety
/// The returned pointer is always valid and statically allocated.
#[no_mangle]
pub unsafe extern "C" fn tachyon_get_backend_name() -> *const std::os::raw::c_char {
    let name = crate::active_backend();
    // Backend name strings are static and null-terminated
    name.as_ptr().cast::<std::os::raw::c_char>()
}
