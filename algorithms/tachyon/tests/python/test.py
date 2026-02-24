#!/usr/bin/env python3
"""
Tachyon Python Binding Test

Loads test vectors from the central JSON file to ensure consistency
across all language bindings.
"""
import json
import os
import sys

# Add the wrapper directory to python path
sys.path.append(os.path.join(os.getcwd(), "../../bindings/python"))

try:
    import tachyon
except ImportError as e:
    print(f"Failed to import tachyon wrapper: {e}")
    sys.exit(1)


def load_test_vectors():
    """Load test vectors from the central JSON file."""
    vectors_path = os.path.join(os.getcwd(), "tests/test_vectors.json")
    with open(vectors_path, "r") as f:
        return json.load(f)


def test_wrapper():
    print("Testing Tachyon Python Wrapper...")
    
    vectors = load_test_vectors()
    
    for vec in vectors["vectors"]:
        name = vec["name"]
        print(f"\n[Test Case: {name}]")

        # Expand placeholders to actual data
        if vec["input"] == "LARGE_1KB":
            data = bytes([0x41] * 1024)  # 1KB of 'A'
        elif vec["input"] == "MEDIUM_256_A":
            data = bytes([0x41] * 256)   # 256 bytes of 'A'
        elif vec["input"] == "HUGE_1MB":
            data = bytes([0x41] * (1024 * 1024))  # 1MB of 'A'
        elif vec["input"] == "EXACT_64_ZERO":
            data = bytes([0x00] * 64)
        elif vec["input"] == "EXACT_512_ONE":
            data = bytes([0x01] * 512)
        elif vec["input"] == "UNALIGNED_63_TWO":
            data = bytes([0x02] * 63)
        else:
            data = vec["input"].encode("utf-8")
        expected = vec["hash"]
        
        # 1. Byte Hash
        h = tachyon.hash(data)
        hex_hash = h.hex()
        print(f"  Input len: {len(data)}")
        print(f"  Hash:      {hex_hash}")
        print(f"  Expected:  {expected}")
        
        if hex_hash != expected:
            print(f"❌ Mismatch for '{name}'!")
            sys.exit(1)
        print("  ✓ Hash matches")
            
        # 2. Verify (Secure API)
        if not tachyon.verify(data, h):
            print(f"❌ Verification failed for '{name}'!")
            sys.exit(1)
        print("  ✓ Verification passed")
            
        # 3. Bad Verify
        bad_hash = bytearray(h)
        bad_hash[0] ^= 0xFF
        if tachyon.verify(data, bytes(bad_hash)):
            print(f"❌ Bad verification succeeded for '{name}' (SHOULD FAIL)!")
            sys.exit(1)
        print("  ✓ Bad hash rejected")

    # 4. Seeded Hashing
    print("\n[Test Case: Seeded Hashing]")
    data = b"Seeded Data"
    seed1 = 12345
    seed2 = 67890
    
    h1 = tachyon.hash_seeded(data, seed1)
    h2 = tachyon.hash_seeded(data, seed2)
    h3 = tachyon.hash_seeded(data, seed1)
    
    if h1 == h2:
        print("❌ Seeded hash collision (different seeds produced same hash)!")
        sys.exit(1)
    if h1 != h3:
        print("❌ Seeded hash non-deterministic (same seed produced different hash)!")
        sys.exit(1)
        
    print(f"  Seed {seed1}: {h1.hex()}")
    print(f"  Seed {seed2}: {h2.hex()}")
    print(f"  ✓ Seed changes output")
    print(f"  ✓ Output is deterministic")
        
    print("\n✅ Python Wrapper OK (All vectors passed)")


if __name__ == "__main__":
    test_wrapper()
