//! Tests for the `digest` trait integration.
#![cfg(feature = "digest-trait")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Verifies that `TachyonHasher` implements the `Digest` trait correctly and can be used in generic contexts.

use crypto_common::Key;
use tachyon::digest::{Digest, KeyInit};
use tachyon::Hasher;

// Helper functions moved out of test body to satisfy `items_after_statements`
fn hash_generic<D: Digest>(input: &[u8]) -> Vec<u8> {
    let mut h = D::new();
    h.update(input);
    h.finalize().to_vec()
}

fn hash_keyed_generic<D: Digest + KeyInit>(key: &[u8], input: &[u8]) -> Vec<u8> {
    let key_arr = Key::<D>::try_from(key).expect("Key length mismatch");
    let mut h = <D as KeyInit>::new(&key_arr);
    h.update(input);
    h.finalize().to_vec()
}

#[test]
fn test_digest_trait_usage() {
    // 1. Standard Usage (Direct)
    let mut hasher = Hasher::new().expect("Hardware support required for test");
    hasher.update(b"test");
    let res1 = hasher.finalize();

    // 2. Generic Usage (via Trait)
    let res2 = hash_generic::<Hasher>(b"test");
    assert_eq!(res1, res2.as_slice());

    // 3. Keyed Usage (via KeyInit Trait)
    let key = [0x42u8; 32];
    let res_keyed = hash_keyed_generic::<Hasher>(&key, b"test");

    // Compare with native keyed API
    let mut native_keyed = Hasher::new().expect("Hardware support required for test");
    native_keyed.set_key(&key);
    native_keyed.update(b"test");
    let res_native = native_keyed.finalize();

    assert_eq!(res_keyed.as_slice(), res_native);
}
