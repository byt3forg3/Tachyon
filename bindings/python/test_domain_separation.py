#!/usr/bin/env python3
"""Domain separation tests for Python bindings."""

import sys
sys.path.insert(0, '.')

import tachyon

def test_constants():
    """Verify domain constants."""
    assert tachyon.DOMAIN_GENERIC == 0
    assert tachyon.DOMAIN_FILE_CHECKSUM == 1
    assert tachyon.DOMAIN_KEY_DERIVATION == 2
    assert tachyon.DOMAIN_MESSAGE_AUTH == 3
    assert tachyon.DOMAIN_DATABASE_INDEX == 4
    assert tachyon.DOMAIN_CONTENT_ADDRESSED == 5
    print("✓ Constants defined correctly")

def test_hash_seeded():
    """Seeded hashing."""
    data = b"test data"

    # Different seeds produce different hashes
    h0 = tachyon.hash_seeded(data, 0)
    h1 = tachyon.hash_seeded(data, 1)
    h2 = tachyon.hash_seeded(data, 12345)

    assert len(h0) == 32
    assert len(h1) == 32
    assert len(h2) == 32
    assert h0 != h1
    assert h1 != h2
    assert h0 != h2

    # Same seed produces same hash
    h0_again = tachyon.hash_seeded(data, 0)
    assert h0 == h0_again

    # Different data with same seed produces different hash
    h_diff = tachyon.hash_seeded(b"different data", 0)
    assert h0 != h_diff

    print("✓ hash_seeded works correctly")

def test_hash_with_domain():
    """Domain-separated hashing."""
    data = b"test data"
    
    # Different domains produce different hashes
    h0 = tachyon.hash_with_domain(data, tachyon.DOMAIN_GENERIC)
    h1 = tachyon.hash_with_domain(data, tachyon.DOMAIN_FILE_CHECKSUM)
    h2 = tachyon.hash_with_domain(data, tachyon.DOMAIN_KEY_DERIVATION)
    
    assert len(h0) == 32
    assert len(h1) == 32
    assert len(h2) == 32
    assert h0 != h1
    assert h1 != h2
    assert h0 != h2
    
    # Same domain produces same hash
    h0_again = tachyon.hash_with_domain(data, tachyon.DOMAIN_GENERIC)
    assert h0 == h0_again
    
    print("✓ hash_with_domain works correctly")

def test_hash_keyed():
    """Keyed hashing (MAC)."""
    data = b"message"
    key = b"k" * 32
    
    mac = tachyon.hash_keyed(data, key)
    assert len(mac) == 32
    
    # Different keys produce different MACs
    key2 = b"x" * 32
    mac2 = tachyon.hash_keyed(data, key2)
    assert mac != mac2
    
    # Same key + data = same MAC
    mac_again = tachyon.hash_keyed(data, key)
    assert mac == mac_again
    
    print("✓ hash_keyed works correctly")

def test_verify_mac():
    """MAC verification."""
    data = b"authenticate this"
    key = b"s" * 32  # 32 bytes key
    
    mac = tachyon.hash_keyed(data, key)
    
    # Correct MAC verifies
    assert tachyon.verify_mac(data, key, mac) == True
    
    # Wrong MAC fails
    wrong_mac = b"x" * 32
    assert tachyon.verify_mac(data, key, wrong_mac) == False
    
    # Wrong key fails
    wrong_key = b"w" * 32  # 32 bytes key
    assert tachyon.verify_mac(data, wrong_key, mac) == False
    
    # Wrong data fails
    assert tachyon.verify_mac(b"different data", key, mac) == False
    
    print("✓ verify_mac works correctly")

def test_derive_key():
    """Key derivation."""
    master_key = b"m" * 32
    
    # Different contexts produce different keys
    k1 = tachyon.derive_key(b"app-v1", master_key)
    k2 = tachyon.derive_key(b"app-v2", master_key)
    k3 = tachyon.derive_key(b"database", master_key)
    
    assert len(k1) == 32
    assert len(k2) == 32
    assert len(k3) == 32
    assert k1 != k2
    assert k2 != k3
    assert k1 != k3
    
    # Same context produces same key
    k1_again = tachyon.derive_key(b"app-v1", master_key)
    assert k1 == k1_again
    
    print("✓ derive_key works correctly")

def test_streaming_with_domain():
    """Streaming with domain."""
    data = b"streaming test data"
    
    # Hash with domain
    hasher = tachyon.Hasher(domain=tachyon.DOMAIN_FILE_CHECKSUM)
    hasher.update(data[:10])
    hasher.update(data[10:])
    h1 = hasher.finalize()
    
    assert len(h1) == 32
    
    # Different domain produces different hash
    hasher2 = tachyon.Hasher(domain=tachyon.DOMAIN_KEY_DERIVATION)
    hasher2.update(data[:10])
    hasher2.update(data[10:])
    h2 = hasher2.finalize()
    
    assert h1 != h2
    
    # No domain (default)
    hasher3 = tachyon.Hasher()
    hasher3.update(data)
    h3 = hasher3.finalize()
    
    assert h3 != h1
    assert h3 != h2
    
    print("✓ Streaming with domain works correctly")

def test_error_handling():
    """Error handling."""
    try:
        # Invalid domain
        tachyon.hash_with_domain(b"test", 99)
        assert False, "Should have raised ValueError"
    except ValueError as e:
        assert "Domain must be 0-5" in str(e)
    
    try:
        # Wrong key size
        tachyon.hash_keyed(b"data", b"short_key")
        assert False, "Should have raised ValueError"
    except ValueError as e:
        assert "32 bytes" in str(e)
    
    try:
        # Wrong MAC size
        tachyon.verify_mac(b"data", b"k" * 32, b"short")
        assert False, "Should have raised ValueError"
    except ValueError as e:
        assert "32 bytes" in str(e)
    
    print("✓ Error handling works correctly")

if __name__ == "__main__":
    print("Testing Python bindings domain separation...")
    test_constants()
    test_hash_seeded()
    test_hash_with_domain()
    test_hash_keyed()
    test_verify_mac()
    test_derive_key()
    test_streaming_with_domain()
    test_error_handling()
    print("\n✅ All Python tests passed!")
