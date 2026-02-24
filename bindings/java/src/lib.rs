//! Tachyon Java JNI Bindings
//!
//! Native implementation for com.tachyon.Tachyon class.

use jni::objects::{JByteArray, JClass};
use jni::sys::{jboolean, jbyteArray, jlong};
use jni::JNIEnv;

// =============================================================================
// ONE-SHOT API
// =============================================================================

/// Compute Tachyon hash of input data.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHash(
    env: JNIEnv,
    _class: JClass,
    input: jbyteArray,
) -> jbyteArray {
    let input_obj = JByteArray::from_raw(input);
    let input_bytes = env.convert_byte_array(&input_obj).unwrap_or_default();
    let hash = tachyon::hash(&input_bytes);
    let output = env.byte_array_from_slice(&hash).unwrap();
    output.into_raw()
}

/// Compute Tachyon hash with a seed.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHashSeeded(
    env: JNIEnv,
    _class: JClass,
    input: jbyteArray,
    seed: jlong,
) -> jbyteArray {
    let input_obj = JByteArray::from_raw(input);
    let input_bytes = env.convert_byte_array(&input_obj).unwrap_or_default();
    let hash = tachyon::hash_seeded(&input_bytes, seed as u64);
    let output = env.byte_array_from_slice(&hash).unwrap();
    output.into_raw()
}

/// Verify hash in constant time.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeVerify(
    env: JNIEnv,
    _class: JClass,
    input: jbyteArray,
    expected_hash: jbyteArray,
) -> jboolean {
    let input_obj = JByteArray::from_raw(input);
    let expected_obj = JByteArray::from_raw(expected_hash);

    let input_bytes = env.convert_byte_array(&input_obj).unwrap_or_default();
    let expected_bytes = env.convert_byte_array(&expected_obj).unwrap_or_default();

    if expected_bytes.len() != 32 {
        return 0;
    }

    let mut fixed_hash = [0u8; 32];
    fixed_hash.copy_from_slice(&expected_bytes);

    if tachyon::verify(&input_bytes, &fixed_hash) {
        1
    } else {
        0
    }
}

/// Compute hash with domain separation.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHashWithDomain(
    env: JNIEnv,
    _class: JClass,
    input: jbyteArray,
    domain: jni::sys::jbyte,
) -> jbyteArray {
    let input_obj = JByteArray::from_raw(input);
    let input_bytes = env.convert_byte_array(&input_obj).unwrap_or_default();

    let tachyon_domain = match domain as u8 {
        0 => tachyon::TachyonDomain::Generic,
        1 => tachyon::TachyonDomain::FileChecksum,
        2 => tachyon::TachyonDomain::KeyDerivation,
        3 => tachyon::TachyonDomain::MessageAuth,
        4 => tachyon::TachyonDomain::DatabaseIndex,
        5 => tachyon::TachyonDomain::ContentAddressed,
        _ => tachyon::TachyonDomain::Generic,
    };

    let hash = tachyon::hash_with_domain(&input_bytes, tachyon_domain);
    let output = env.byte_array_from_slice(&hash).unwrap();
    output.into_raw()
}

/// Compute keyed hash (MAC).
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHashKeyed(
    env: JNIEnv,
    _class: JClass,
    input: jbyteArray,
    key: jbyteArray,
) -> jbyteArray {
    let input_obj = JByteArray::from_raw(input);
    let key_obj = JByteArray::from_raw(key);

    let input_bytes = env.convert_byte_array(&input_obj).unwrap_or_default();
    let key_bytes = env.convert_byte_array(&key_obj).unwrap_or_default();

    if key_bytes.len() != 32 {
        return env.byte_array_from_slice(&[0u8; 32]).unwrap().into_raw();
    }

    let mut fixed_key = [0u8; 32];
    fixed_key.copy_from_slice(&key_bytes);

    let mac = tachyon::hash_keyed(&input_bytes, &fixed_key);
    let output = env.byte_array_from_slice(&mac).unwrap();
    output.into_raw()
}

/// Verify keyed hash (MAC) in constant time.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeVerifyMac(
    env: JNIEnv,
    _class: JClass,
    input: jbyteArray,
    key: jbyteArray,
    expected_mac: jbyteArray,
) -> jboolean {
    let input_obj = JByteArray::from_raw(input);
    let key_obj = JByteArray::from_raw(key);
    let mac_obj = JByteArray::from_raw(expected_mac);

    let input_bytes = env.convert_byte_array(&input_obj).unwrap_or_default();
    let key_bytes = env.convert_byte_array(&key_obj).unwrap_or_default();
    let mac_bytes = env.convert_byte_array(&mac_obj).unwrap_or_default();

    if key_bytes.len() != 32 || mac_bytes.len() != 32 {
        return 0;
    }

    let mut fixed_key = [0u8; 32];
    let mut fixed_mac = [0u8; 32];
    fixed_key.copy_from_slice(&key_bytes);
    fixed_mac.copy_from_slice(&mac_bytes);

    if tachyon::verify_mac(&input_bytes, &fixed_key, &fixed_mac) {
        1
    } else {
        0
    }
}

/// Derive cryptographic key from material.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeDeriveKey(
    env: JNIEnv,
    _class: JClass,
    context: jbyteArray,
    key_material: jbyteArray,
) -> jbyteArray {
    let context_obj = JByteArray::from_raw(context);
    let material_obj = JByteArray::from_raw(key_material);

    let context_bytes = env.convert_byte_array(&context_obj).unwrap_or_default();
    let material_bytes = env.convert_byte_array(&material_obj).unwrap_or_default();

    if material_bytes.len() != 32 {
        return env.byte_array_from_slice(&[0u8; 32]).unwrap().into_raw();
    }

    let context_str = std::str::from_utf8(&context_bytes).unwrap_or("");
    let mut fixed_material = [0u8; 32];
    fixed_material.copy_from_slice(&material_bytes);

    let derived = tachyon::derive_key(context_str, &fixed_material);
    let output = env.byte_array_from_slice(&derived).unwrap();
    output.into_raw()
}

// =============================================================================
// STREAMING API
// =============================================================================

/// Create new streaming hasher.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHasherNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    match tachyon::Hasher::new() {
        Ok(hasher) => Box::into_raw(Box::new(hasher)) as jlong,
        Err(_) => 0,
    }
}

/// Create new streaming hasher with domain separation.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHasherNewWithDomain(
    _env: JNIEnv,
    _class: JClass,
    domain: jni::sys::jbyte,
) -> jlong {
    let tachyon_domain = match domain as u8 {
        0 => tachyon::TachyonDomain::Generic,
        1 => tachyon::TachyonDomain::FileChecksum,
        2 => tachyon::TachyonDomain::KeyDerivation,
        3 => tachyon::TachyonDomain::MessageAuth,
        4 => tachyon::TachyonDomain::DatabaseIndex,
        5 => tachyon::TachyonDomain::ContentAddressed,
        _ => tachyon::TachyonDomain::Generic,
    };

    match tachyon::Hasher::new_with_domain(tachyon_domain.to_u64()) {
        Ok(hasher) => Box::into_raw(Box::new(hasher)) as jlong,
        Err(_) => 0,
    }
}

/// Create new streaming hasher with seed.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHasherNewSeeded(
    _env: JNIEnv,
    _class: JClass,
    seed: jlong,
) -> jlong {
    match tachyon::Hasher::new_full(0, seed as u64) {
        Ok(hasher) => Box::into_raw(Box::new(hasher)) as jlong,
        Err(_) => 0,
    }
}

/// Update hasher with data.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
/// It casts the `state` jlong to a raw pointer to `Hasher`.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHasherUpdate(
    env: JNIEnv,
    _class: JClass,
    state: jlong,
    data: jbyteArray,
) {
    if state == 0 {
        return;
    }
    let hasher = &mut *(state as *mut tachyon::Hasher);
    let data_obj = JByteArray::from_raw(data);
    let data_bytes = env.convert_byte_array(&data_obj).unwrap_or_default();
    hasher.update(&data_bytes);
}

/// Finalize hasher and return hash. Frees the hasher.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
/// It allows the `Box` from `state` to be dropped, freeing memory.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHasherFinalize(
    env: JNIEnv,
    _class: JClass,
    state: jlong,
) -> jbyteArray {
    if state == 0 {
        return std::ptr::null_mut();
    }
    let hasher = Box::from_raw(state as *mut tachyon::Hasher);
    let hash = hasher.finalize();
    let output = env.byte_array_from_slice(&hash).unwrap();
    output.into_raw()
}

/// Free hasher without finalizing.
/// # Safety
///
/// This function is called from Java and expects valid JNI environment and object pointers.
/// It dereferences raw pointers provided by the JNI runtime.
/// It manually drops the `Hasher` pointed to by `state`.
#[no_mangle]
pub unsafe extern "system" fn Java_com_tachyon_Tachyon_nativeHasherFree(
    _env: JNIEnv,
    _class: JClass,
    state: jlong,
) {
    if state != 0 {
        drop(Box::from_raw(state as *mut tachyon::Hasher));
    }
}
