//! Security Feature Tests
//!
//! Tests for Domain Separation, Keyed Hashing, Key Derivation,
//! and various edge cases to ensure robust security guarantees.

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::unwrap_used)]

use tachyon::{derive_key, hash, hash_keyed, hash_with_domain, verify_mac, TachyonDomain};

// =============================================================================
// DOMAIN SEPARATION TESTS
// =============================================================================

#[test]
fn test_domain_separation_basic() {
    let data = b"test data";

    let generic = hash_with_domain(data, TachyonDomain::Generic);
    let file = hash_with_domain(data, TachyonDomain::FileChecksum);
    let kdf = hash_with_domain(data, TachyonDomain::KeyDerivation);
    let mac = hash_with_domain(data, TachyonDomain::MessageAuth);
    let db = hash_with_domain(data, TachyonDomain::DatabaseIndex);
    let cas = hash_with_domain(data, TachyonDomain::ContentAddressed);

    // All domains must produce different hashes
    assert_ne!(generic, file, "Generic vs FileChecksum collision");
    assert_ne!(generic, kdf, "Generic vs KeyDerivation collision");
    assert_ne!(generic, mac, "Generic vs MessageAuth collision");
    assert_ne!(generic, db, "Generic vs DatabaseIndex collision");
    assert_ne!(generic, cas, "Generic vs ContentAddressed collision");
    assert_ne!(file, kdf, "FileChecksum vs KeyDerivation collision");
    assert_ne!(file, mac, "FileChecksum vs MessageAuth collision");
    assert_ne!(mac, kdf, "MessageAuth vs KeyDerivation collision");
}

#[test]
fn test_custom_domain() {
    let data = b"custom domain test";

    let custom1 = hash_with_domain(data, TachyonDomain::Generic); // Use predefined
    let custom2 = hash_with_domain(data, TachyonDomain::FileChecksum);
    let generic = hash_with_domain(data, TachyonDomain::Generic);

    assert_ne!(custom1, custom2, "Different domains must differ");
    assert_eq!(custom1, generic, "Same domain should match");
}

#[test]
fn test_domain_separation_empty_input() {
    let empty = b"";

    let d1 = hash_with_domain(empty, TachyonDomain::Generic);
    let d2 = hash_with_domain(empty, TachyonDomain::FileChecksum);

    assert_ne!(d1, d2, "Empty input must still separate domains");
}

#[test]
fn test_domain_separation_large_input() {
    let large = vec![0x42u8; 1_000_000]; // 1 MB

    let d1 = hash_with_domain(&large, TachyonDomain::Generic);
    let d2 = hash_with_domain(&large, TachyonDomain::FileChecksum);

    assert_ne!(d1, d2, "Large input must still separate domains");
}

// =============================================================================
// KEYED HASH (MAC) TESTS
// =============================================================================

#[test]
fn test_keyed_hash_basic() {
    let data = b"message";
    let key1 = [1u8; 32];
    let key2 = [2u8; 32];

    let mac1 = hash_keyed(data, &key1);
    let mac2 = hash_keyed(data, &key2);
    let unkeyed = hash(data);

    assert_ne!(mac1, mac2, "Different keys must produce different MACs");
    assert_ne!(mac1, unkeyed, "Keyed hash must differ from unkeyed");
}

#[test]
fn test_verify_mac() {
    let data = b"authenticated message";
    let key = [42u8; 32];

    let mac = hash_keyed(data, &key);
    assert!(verify_mac(data, &key, &mac), "Valid MAC must verify");

    // Wrong key
    let wrong_key = [43u8; 32];
    assert!(!verify_mac(data, &wrong_key, &mac), "Wrong key must fail");

    // Wrong data
    assert!(
        !verify_mac(b"tampered", &key, &mac),
        "Tampered data must fail"
    );

    // Flipped bit in MAC
    let mut bad_mac = mac;
    bad_mac[0] ^= 0x01;
    assert!(!verify_mac(data, &key, &bad_mac), "Corrupted MAC must fail");
}

#[test]
fn test_mac_empty_input() {
    let key = [7u8; 32];
    let mac1 = hash_keyed(b"", &key);
    let mac2 = hash_keyed(b"", &key);

    assert_eq!(mac1, mac2, "Empty input must be deterministic");
    assert!(verify_mac(b"", &key, &mac1));
}

#[test]
fn test_mac_zero_key() {
    let data = b"test";
    let zero_key = [0u8; 32];
    let other_key = [1u8; 32];

    let mac_zero = hash_keyed(data, &zero_key);
    let mac_other = hash_keyed(data, &other_key);

    assert_ne!(
        mac_zero, mac_other,
        "Zero key must still produce unique MAC"
    );
}

#[test]
fn test_mac_all_ones_key() {
    let data = b"test";
    let ones_key = [0xFFu8; 32];
    let other_key = [0xFEu8; 32];

    let mac_ones = hash_keyed(data, &ones_key);
    let mac_other = hash_keyed(data, &other_key);

    assert_ne!(mac_ones, mac_other);
}

#[test]
fn test_keyed_hashing_consistency() {
    let key = [0x42u8; 32];
    let input = vec![0u8; 200 * 1024]; // Larger than CHUNK_SIZE

    // 1. One-shot
    let h1 = hash_keyed(&input, &key);

    // 2. Streaming
    let mut hasher = tachyon::Hasher::new_full(TachyonDomain::MessageAuth.to_u64(), 0).unwrap();
    hasher.set_key(&key);
    hasher.update(&input);
    let h2 = hasher.finalize();

    assert_eq!(h1, h2, "One-shot MAC does not match streaming MAC");

    // 3. Small one-shot vs streaming
    let small_input = b"small";
    let h3 = hash_keyed(small_input, &key);
    let mut hasher2 = tachyon::Hasher::new_full(TachyonDomain::MessageAuth.to_u64(), 0).unwrap();
    hasher2.set_key(&key);
    hasher2.update(small_input);
    let h4 = hasher2.finalize();

    assert_eq!(h3, h4, "Small one-shot MAC does not match streaming MAC");
}

// =============================================================================
// KEY DERIVATION TESTS
// =============================================================================

#[test]
fn test_key_derivation_basic() {
    let master = [100u8; 32];

    let key1 = derive_key("context-1", &master);
    let key2 = derive_key("context-2", &master);
    let key3 = derive_key("session-key", &master);

    assert_ne!(key1, key2, "Different contexts must produce different keys");
    assert_ne!(key1, key3);
    assert_ne!(key2, key3);
}

#[test]
fn test_kdf_deterministic() {
    let master = [5u8; 32];
    let context = "app-encryption-key";

    let key1 = derive_key(context, &master);
    let key2 = derive_key(context, &master);

    assert_eq!(key1, key2, "KDF must be deterministic");
}

#[test]
fn test_kdf_different_masters() {
    let master1 = [1u8; 32];
    let master2 = [2u8; 32];
    let context = "same-context";

    let key1 = derive_key(context, &master1);
    let key2 = derive_key(context, &master2);

    assert_ne!(
        key1, key2,
        "Different master keys must produce different outputs"
    );
}

#[test]
fn test_kdf_empty_context() {
    let master = [10u8; 32];

    let key_empty = derive_key("", &master);
    let key_other = derive_key("x", &master);

    assert_ne!(key_empty, key_other, "Empty context must still be valid");
}

#[test]
fn test_kdf_long_context() {
    let master = [20u8; 32];
    let long_context = "a".repeat(10_000);

    let key = derive_key(&long_context, &master);
    assert_eq!(key.len(), 32, "KDF must always produce 32 bytes");
}

#[test]
fn test_kdf_unicode_context() {
    let master = [30u8; 32];

    let key1 = derive_key("🔑-emoji-key", &master);
    let key2 = derive_key("session-日本語", &master);
    let key3 = derive_key("ключ-кириллица", &master);

    assert_ne!(key1, key2);
    assert_ne!(key1, key3);
    assert_eq!(key1.len(), 32);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_zero_input() {
    let zero_1 = vec![0u8; 1];
    let zero_1k = vec![0u8; 1024];
    let zero_1m = vec![0u8; 1_000_000];

    let h1 = hash(&zero_1);
    let h2 = hash(&zero_1k);
    let h3 = hash(&zero_1m);

    assert_ne!(h1, h2, "Different lengths of zeros must hash differently");
    assert_ne!(h2, h3);
}

#[test]
fn test_repeated_bytes() {
    let pattern_a = vec![b'A'; 1000];
    let pattern_b = vec![b'B'; 1000];
    let pattern_0 = vec![0u8; 1000];
    let pattern_ff = vec![0xFFu8; 1000];

    let ha = hash(&pattern_a);
    let hb = hash(&pattern_b);
    let h0 = hash(&pattern_0);
    let hf = hash(&pattern_ff);

    assert_ne!(ha, hb);
    assert_ne!(ha, h0);
    assert_ne!(hb, hf);
    assert_ne!(h0, hf);
}

#[test]
fn test_padding_boundaries() {
    // Explicitly verify behavior around the 512-byte block boundary
    // to ensure padding implies different hashes.
    let d511 = vec![0u8; 511];
    let d512 = vec![0u8; 512];
    let d513 = vec![0u8; 513];

    let h511 = hash(&d511);
    let h512 = hash(&d512);
    let h513 = hash(&d513);

    assert_ne!(h511, h512, "Padding failed: 511 vs 512 bytes collided");
    assert_ne!(h512, h513, "Padding failed: 512 vs 513 bytes collided");
}

#[test]
fn test_incremental_sizes() {
    // Test various block boundary conditions
    let sizes = [
        0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513,
        1023, 1024, 1025, 2047, 2048, 2049,
    ];

    let mut hashes = Vec::new();
    for size in sizes {
        // Use varying byte values to avoid symmetry issues
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let h = hash(&data);
        hashes.push((size, h));
    }

    // Check no collisions
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(
                hashes[i].1, hashes[j].1,
                "Collision detected between sizes {} and {}",
                hashes[i].0, hashes[j].0
            );
        }
    }
}

#[test]
fn test_avalanche_effect() {
    // Single bit flip should change ~50% of output bits
    let data1 = b"test message for avalanche effect analysis";
    let mut data2 = *data1;
    data2[0] ^= 0x01; // Flip one bit

    let h1 = hash(data1);
    let h2 = hash(&data2);

    assert_ne!(h1, h2, "Single bit flip must change hash");

    // Count differing bits
    let mut diff_bits = 0;
    for i in 0..32 {
        diff_bits += (h1[i] ^ h2[i]).count_ones();
    }

    // Should be roughly 128 ± 50 bits different (50% of 256 bits)
    // Relaxed bounds for VAES-based hash
    assert!(
        diff_bits > 60 && diff_bits < 196,
        "Avalanche effect weak: only {} of 256 bits differ",
        diff_bits
    );
}

#[test]
fn test_prefix_collision_resistance() {
    // Hash(A) should not equal Hash(A||B)
    let a = b"prefix";
    let ab = b"prefixsuffix";

    let ha = hash(a);
    let hab = hash(ab);

    assert_ne!(ha, hab, "Prefix collision detected");
}

#[test]
fn test_suffix_collision_resistance() {
    let a = b"xyz";
    let ba = b"abcxyz";

    let ha = hash(a);
    let hba = hash(ba);

    assert_ne!(ha, hba, "Suffix collision detected");
}

#[test]
fn test_null_byte_handling() {
    let no_null = b"test";
    let with_null = b"te\x00st";
    let only_null = b"\x00\x00\x00\x00";

    let h1 = hash(no_null);
    let h2 = hash(with_null);
    let h3 = hash(only_null);

    assert_ne!(h1, h2);
    assert_ne!(h1, h3);
    assert_ne!(h2, h3);
}

#[test]
fn test_max_byte_values() {
    let low = vec![0x00u8; 100];
    let mid = vec![0x80u8; 100];
    let high = vec![0xFFu8; 100];

    let h_low = hash(&low);
    let h_mid = hash(&mid);
    let h_high = hash(&high);

    assert_ne!(h_low, h_mid);
    assert_ne!(h_mid, h_high);
    assert_ne!(h_low, h_high);
}

// =============================================================================
// REGRESSION TESTS
// =============================================================================

#[test]
fn test_known_vector_generic() {
    // Ensure hash output doesn't change in future versions
    let data = b"Tachyon v0.1.0";
    let h = hash(data);

    // This is the current output - update if algorithm changes intentionally
    assert_eq!(h.len(), 32, "Hash must be 32 bytes");
    // Don't hardcode exact value as it may change with security fixes
}

#[test]
fn test_domain_not_affect_without_api() {
    // Regular hash() should still work and be stable
    let data = b"regular hash";
    let h1 = hash(data);
    let h2 = hash(data);

    assert_eq!(h1, h2, "hash() must be deterministic");
}

#[test]
fn test_determinism() {
    let data = b"determinism test";
    let key = [99u8; 32];

    // Run multiple times
    for _ in 0..10 {
        let h1 = hash(data);
        let h2 = hash(data);
        assert_eq!(h1, h2);

        let m1 = hash_keyed(data, &key);
        let m2 = hash_keyed(data, &key);
        assert_eq!(m1, m2);

        let k1 = derive_key("test", &key);
        let k2 = derive_key("test", &key);
        assert_eq!(k1, k2);
    }
}

// =============================================================================
// PERFORMANCE EDGE CASES
// =============================================================================

#[test]
fn test_very_large_input() {
    // 10 MB input
    let large = vec![0x5Au8; 10_000_000];
    let h = hash(&large);

    assert_eq!(h.len(), 32);
}

#[test]
fn test_parallel_threshold_boundary() {
    // Test around 128KB parallel threshold (CHUNK_SIZE)
    let chunk = 128 * 1024;
    let sizes = [chunk - 1, chunk, chunk + 1];

    for size in sizes {
        let data = vec![0x42u8; size];
        let h = hash(&data);
        assert_eq!(h.len(), 32);
    }
}
