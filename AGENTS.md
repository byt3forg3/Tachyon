# Tachyon — Agent Instructions

**PROJECT:** AVX-512 + VAES hash function | **STATUS:** Experimental, not audited | **GOAL:** Max AVX-512 efficiency 

---

## CRITICAL CONSTRAINTS (DO NOT VIOLATE)

- **RUST = MAIN, C = REFERENCE:** Rust (`algorithms/tachyon/src/`) is source of truth. C (`c-reference/`) is for SMHasher/external tools only. C-Rust parity is mandatory.
- **ROUNDS:** STRICTLY 10 AES rounds. WHY: Complete Davies-Meyer diffusion.
- **MEMORY:** ZERO allocations in hot paths (compress, finalize).
- **SIMD:** Explicit AVX-512 vectors only. DO NOT auto-vectorize scalar fallback loops.
- **LINTS:** `unwrap_used`, `expect_used`, `unsafe_code` are workspace-denied. CI runs `clippy -- -D warnings` (all warnings = errors!).

---

## ALGORITHM SCOPE

**Tachyon Core** (`algorithms/tachyon/`) is the only implemented algorithm. It is crypto-hardened (Davies-Meyer, 10 rounds AES), intended for deduplication and integrity checks, and remains experimental with stable output.

---

## VERIFICATION WORKFLOW (BEFORE ANY CHANGE)

```bash
cargo test --release --workspace --lib --bins --tests  # Skips doctests (fail with panic=abort). Compiles C (AVX-512/AES-NI/Portable), tests Rust-C parity (0-4096B + 10MB), bindings vs test_vectors.json, validates all FFIs

cd verification && ./setup_testing.sh  # For algorithm changes (SMHasher/PractRand)
cargo run --release --example quality_suite  # Quick quality check (Chi-Squared, Avalanche, BIC)

# Fuzzing (continuous security testing via Docker)
docker build -f Dockerfile.fuzz -t tachyon-fuzz .
docker run -it --rm tachyon-fuzz cargo fuzz run fuzz_security    # MAC/KDF/domain separation
docker run -it --rm tachyon-fuzz cargo fuzz run fuzz_streaming   # Streaming equivalence
```

---

## CRITICAL FILES (READ THESE FIRST)

1. **`algorithms/tachyon/src/kernels/constants.rs`**: All crypto constants (ROUNDS, primes, φ). Mathematical ground truth.
2. **`algorithms/tachyon/src/kernels/avx512/compress.rs`**: Core Davies-Meyer + AVX-512 compression logic (~298 lines).
3. **`algorithms/tachyon/tests/test_vectors.json`**: Source of truth for hash outputs. If unchanged when it should change → regression!
4. **`algorithms/tachyon/tests/c_reference_cross_arch.rs`**: Exhaustive Rust-C parity tests (compiles C backends, tests 0-10MB).

---

## CODE STYLE (FOLLOW EXISTING PATTERNS)

- **Module docs:** `//!` with description + technical details
- **Major sections:** three-line `// =============================================================================` banners at module scope
- **Subsections and ordered steps:** single-line `// ── 1. Step title ───────────` banners
- **Implementation details:** plain `// Comment` lines below the relevant subsection
- **Doc comments:** `///` for public functions/types
- **Inline comments:** Explain WHY, not just WHAT
- **SAFETY comments:** `// SAFETY: ...` required for ALL `unsafe` blocks

---

## COMMON PITFALLS

❌ Changing ROUNDS/chunks without full SMHasher/PractRand verification
❌ Adding allocations in hot paths
❌ Using `unwrap()`, `expect()`, or unjustified `unsafe`
❌ Adding `unsafe` code without `// SAFETY:` comments explaining why it's safe (Rust best practice, especially for crypto code)
❌ Breaking Rust-C parity "to make code cleaner"
❌ Modifying `test_vectors.json` without understanding it's the source of truth
❌ Adding `#[allow(clippy::...)]` to silence lints — Clippy rules are intentionally strict! Only allow in core code for absolute necessity (e.g., `unsafe_code` for SIMD intrinsics). Tests can be more relaxed.
❌ Adding `-march=native` to C reference Makefile — Would auto-vectorize portable fallback and break cross-compilation. Rust uses `target-cpu=native` (`.cargo/config.toml`), C must NOT!

---

## WORKSPACE STRUCTURE (QUICK NAV)

```
algorithms/tachyon/src/kernels/
├── constants.rs        # Mathematical truth (start here)
├── avx512/compress.rs  # Core logic (Davies-Meyer + SIMD)
├── aesni/              # Fallback for non-AVX-512 and short path
└── portable/           # Scalar fallback

algorithms/tachyon/c-reference/
├── tachyon_avx512.c    # AVX-512 backend
├── tachyon_aesni.c     # AES-NI backend
├── tachyon_portable.c  # Portable backend
└── tachyon_dispatcher.c # Runtime CPU dispatch

algorithms/tachyon/tests/
├── test_vectors.json           # Source of truth
└── c_reference_cross_arch.rs   # Rust-C parity tests
```

---

## ARCHITECTURE NOTES

- **Dual-path routing (64-byte threshold):**
  - `< 64 bytes`: **Short path (ALWAYS AES-NI based)** — 512-bit state, low-latency. Used even in AVX-512 mode!
  - `≥ 64 bytes`: Bulk path (AVX-512 if available, else AES-NI) — 4096-bit state, 32 parallel tracks via 8 accumulators
- **Block size:** 512 bytes per compression cycle
- **Intrinsics:** `_mm512_aesenc_epi128` (VAES), `VPCLMULQDQ` required
- **Constants:** Derived from `frac(ln(prime)) * 2^64` or Golden Ratio (φ) — verify with `scripts/generate_constants.py`
- **Parallelism:** Merkle tree structure enables multi-core without changing output
- **Timing:** Hash computation is NOT constant-time (data-dependent branches exist). Only verification (`verify()`, `verify_mac()`) is constant-time.
- **no_std support:** Library supports `no_std` + `alloc` for embedded/bare-metal. Default features: `std` + `digest-trait` + `multithread` (multithread requires std).
- **Backend hierarchy:** AVX-512 is primary target (designed for). AES-NI and Portable are fallbacks that emulate AVX-512 algorithm (byte-identical outputs). Portable is a standalone implementation for non-x86 (ARM, etc.) or CPUs without AES-NI — NOT a simple fallback!

---

## WHEN IN DOUBT

1. Read `constants.rs` first (mathematical ground truth)
2. Check `test_vectors.json` (expected outputs)
3. Run `cargo test --release --workspace --lib --bins --tests` (catches 99% of issues, skips doctests)
4. Ask before changing ROUNDS, chunk sizes, or core crypto logic
