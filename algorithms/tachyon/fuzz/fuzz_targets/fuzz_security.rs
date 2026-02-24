#![no_main]

use libfuzzer_sys::fuzz_target;
use tachyon::{derive_key, hash_keyed, hash_with_domain, verify_mac, TachyonDomain};

fuzz_target!(|data: &[u8]| {
    // =============================================================================
    // PREPARATION
    // =============================================================================

    // Split input to get specific parameters
    let mut key = [0u8; 32];
    let msg_start = if data.len() >= 32 {
        key.copy_from_slice(&data[0..32]);
        32
    } else {
        0
    };
    let msg = &data[msg_start..];

    // =============================================================================
    // 1. KEYED HASHING (MAC)
    // =============================================================================

    let mac = hash_keyed(msg, &key);

    // Positive case: Correct key must verify
    assert!(
        verify_mac(msg, &key, &mac),
        "MAC verification failed with correct key"
    );

    // Negative case: Wrong key must fail
    let mut wrong_key = key;
    wrong_key[0] ^= 0xFF; // Flip bits

    assert!(
        !verify_mac(msg, &wrong_key, &mac),
        "MAC verification succeeded with wrong key"
    );

    // =============================================================================
    // 2. KEY DERIVATION (KDF)
    // =============================================================================

    if let Ok(context_str) = std::str::from_utf8(msg) {
        let derived = derive_key(context_str, &key);

        // Determinism Check
        let derived2 = derive_key(context_str, &key);
        assert_eq!(derived, derived2, "KDF not deterministic");

        // Context Separation Check
        let context_modified = format!("{context_str}x");
        let derived_mod = derive_key(&context_modified, &key);
        assert_ne!(derived, derived_mod, "KDF collision on different context");
    }

    // =============================================================================
    // 3. DOMAIN SEPARATION
    // =============================================================================

    let d1 = hash_with_domain(msg, TachyonDomain::Generic);
    let d2 = hash_with_domain(msg, TachyonDomain::FileChecksum);

    assert_ne!(d1, d2, "Domain separation failed (Generic vs FileChecksum)");
});
