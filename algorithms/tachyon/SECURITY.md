# Security Policy

## Experimental Status

> [!WARNING]
> **Experimental - Not Audited**  
> This hash function has not undergone formal cryptographic review. Use it for non-critical checksums, deduplication, and caching. **Do not use** for cryptographic signatures, password hashing, or security-critical applications. For production use, prefer audited alternatives like BLAKE3 or SHA-256.

**Tachyon is experimental and has not undergone formal security audit or cryptographic review.**

Do not use Tachyon for:
- Cryptographic signatures
- Password hashing
- Production message authentication (hash_keyed is experimental and unaudited)
- Security-critical applications requiring cryptographic guarantees
- Any use case requiring audited collision resistance

This hash function is designed for non-cryptographic use cases like checksums, deduplication, and performance-critical hashing in trusted environments.

## Hash Quality Verification

Tachyon has been validated against statistical test suites.
All test results are available in [`verification/.results/`](../../verification/.results/).

### Test Suite Results

**SMHasher**
Comprehensive hash quality verification covering avalanche, bit independence, and cyclic properties.

- **Avalanche:** All tests passed (ideal ~50% bit flip distribution)
- **Bit Independence:** No bias detected
- **Results:** [C Reference](../../verification/.results/smhasher_result_tachyon_v.0.1_c.txt) | [Rust Native](../../verification/.results/smhasher_result_tachyon_v.0.1_rust.txt)

**TestU01 BigCrush**
Academic standard for RNG quality (160 statistical tests).

- **159/160 tests:** Passed cleanly within [0.001, 0.9990] range
- **1 marginal p-value:** Test 50 (SampleProd, t=8) = 0.9991 (just above 0.9990 threshold)
- **Results:** [BigCrush Full Report](../../verification/.results/bigcrush_result_cyclic_tachyon_v.0.1_rust.txt)

**PractRand**
Long-range statistical anomaly detection for hash function quality.

- **No anomalies detected** till 1tb in test run.
- **Results:** [PractRand Output](../../verification/.results/practrand_result_cyclic_tachyon_v.0.1_rust.txt)

> [!NOTE]
> These tests validate statistical quality but do NOT constitute a cryptographic security audit.
> For production security-critical applications, use audited alternatives like BLAKE3 or SHA-256.

**Prerequisites to run tests locally:**
- `cmake`, `g++`, `git`, `make`
- Rust toolchain (optional, for Rust-native SMHasher builds)

**To configure and run the external suites:**
```bash
cd verification
./setup_testing.sh
```
This interactive script automates the installation and configuration of **SMHasher**, **PractRand**, and **TestU01**.

## Visual Randomness Test

The images below visualize hash outputs as bitmaps - each pixel represents hash output bytes.

| AES-NI (Small Inputs)                                 | AVX-512 (Large Inputs)                                  |
| :---:                                                 | :---:                                                   |
| ![AES-NI Randomness](tachyon_randomness_aesni.png)    | ![AVX-512 Randomness](tachyon_randomness_avx512.png)    |

### Security Architecture & Construction

For an in-depth look at Tachyon's cryptographically hardened design—including its Butterfly Network Diffusion, Quadratic CLMUL Hardening, Davies-Meyer Feed-Forward, and constant generation—please read our [Architecture Whitepaper](ARCHITECTURE.md).

## Cryptographic Features

> [!IMPORTANT]
> These features provide defense-in-depth for non-cryptographic use cases but are **NOT substitutes for audited cryptographic primitives**.

### Domain Separation (`hash_with_domain`)

Tachyon appends the input length and a **Domain Byte** to every padding block. This prevents length-extension attacks and ensures that inputs in different contexts yield distinct outputs.

**Predefined Domains:**
- `Generic` (0x00) - Default hashing
- `FileChecksum` (0x01) - File integrity verification
- `KeyDerivation` (0x02) - Experimental key derivation (unaudited)
- `MessageAuth` (0x03) - Experimental MAC (unaudited)
- `DatabaseIndex` (0x04) - Database keys and indexing
- `ContentAddressed` (0x05) - Content-addressable storage

**Custom Domains:**
- User-defined domains: `custom_domain(id)` with `id` ∈ [0, 65535]
- Format: `0x1000_0000_0000_0000 | id`
- Sentinel bit prevents collision with predefined domains

**Why Domain Separation Matters:**
Without domain separation, `Hash("data")` used as a file checksum could collide with `Hash("data")` used as a database key. Domain bytes cryptographically bind the hash output to its intended use case.

### Message Authentication Code (`hash_keyed`)

> [!CAUTION]
> **Experimental and unaudited.** Do NOT use for production security.

Tachyon supports keyed hashing by absorbing a 256-bit (32-byte) secret key prior to processing message data:

1. **Key Absorption:** The key is absorbed using 2 AES rounds during initialization (with per-lane offset differentiation and Golden Ratio masking), followed by 4 additional AES rounds during finalization using distinct permutation patterns
2. **Message Processing:** Standard hash pipeline with key-initialized state
3. **Finalization:** Domain byte set to `MessageAuth` (0x03)

**Security Properties:**
- Key never appears in output (one-way absorption)
- Different keys yield independent hash families
- Multi-round absorption provides strong key diffusion

**Use Case:**
Non-critical authentication in trusted environments where audited alternatives (HMAC-SHA256, BLAKE3-keyed) are unavailable.

### Constant-Time Verification (`verify`, `verify_mac`)

The Rust API provides `tachyon::verify(msg, expected_hash)` and `tachyon::verify_mac(msg, key, expected_hash)`. Both use constant-time comparisons (`subtle::ConstantTimeEq`) to mitigate timing side-channel attacks during hash validation.

**What is Constant-Time:**
- Hash verification (comparing outputs)
- MAC validation

**What is NOT Constant-Time:**
- Hash computation itself
- Input length checks
- Padding operations

**When to Use:**
- Verifying MACs of secret keys
- Checking hashes of sensitive data
- Any scenario where timing leaks matter

> [!WARNING]
> **Not Constant-Time:** The hash computation has input-dependent branching (dual-path routing, length checks) and data-dependent memory access patterns. Do NOT hash secret data where timing matters.

## Known Limitations

- No formal security analysis by cryptographers
- No collision resistance guarantees
- Hash computation is not constant-time (though verification is)
- Algorithm may change in future versions

## Reporting Issues

If you discover a hash quality problem or collision:

- Open a GitHub issue with reproduction steps
- For sensitive concerns: `260008633+byt3forg3@users.noreply.github.com`

## Disclaimer

Tachyon is provided "as is" without warranty of any kind. Use at your own risk.
