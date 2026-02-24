#!/usr/bin/env python3
"""
Tachyon constant generator.

Derives all cryptographic constants except golden ratio from a single rule:

    constant = floor(frac(ln(p)) * 2^64)

where p is a prime number and frac(x) = x - floor(x).

Usage:
    python3 scripts/generate_constants.py

Requirements: Python 3 (no external dependencies, uses built-in decimal module)
"""

from decimal import Decimal, getcontext

# 80 digits of precision — far more than the 20 needed for 64-bit constants
getcontext().prec = 80

TWO_64 = Decimal(2) ** 64


def frac_ln_prime(p: int) -> int:
    """Compute floor(frac(ln(p)) * 2^64) for a prime p."""
    ln_p = Decimal(p).ln()
    frac_part = ln_p - int(ln_p)
    return int(frac_part * TWO_64)


def to_hex(value: int) -> str:
    """Format a 64-bit integer as a Rust-style hex literal with underscores."""
    h = f"{value:016X}"
    return f"0x{h[0:4]}_{h[4:8]}_{h[8:12]}_{h[12:16]}"


# Assignment: consecutive primes, partitioned by purpose
INIT_PRIMES = [2, 3, 5, 7, 11, 13, 17]          # C0-C3, C5-C7 (C4 = Golden Ratio)
WHITENING_PRIMES = [19, 23]                     # WHITENING0, WHITENING1
KEY_MULT_PRIME = 29                             # KEY_SCHEDULE_MULT
CLMUL_PRIME = 31                                # CLMUL_CONSTANT

# 32 Lane offsets - starting from prime 37
LANE_PRIMES = [
    37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103,
    107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191
]

ALL_PRIMES = INIT_PRIMES + WHITENING_PRIMES + [KEY_MULT_PRIME, CLMUL_PRIME] + LANE_PRIMES


def main():
    print("// =============================================================================")
    print("// AUTO-GENERATED — do not edit by hand")
    print("// Derivation: floor(frac(ln(p)) * 2^64), p = prime")
    print("// Verify:     python3 scripts/generate_constants.py")
    print("// =============================================================================")
    print()

    # Initialization constants C0-C7
    print("// INITIALIZATION CONSTANTS: frac(ln(p)) for consecutive primes")
    print("// C4 is reserved for the Golden Ratio (not derived from ln)")
    labels = [
        ("C0", INIT_PRIMES[0]),
        ("C1", INIT_PRIMES[1]),
        ("C2", INIT_PRIMES[2]),
        ("C3", INIT_PRIMES[3]),
        # C4 = Golden Ratio, skipped
        ("C5", INIT_PRIMES[4]),
        ("C6", INIT_PRIMES[5]),
        ("C7", INIT_PRIMES[6]),
    ]
    for name, p in labels:
        val = frac_ln_prime(p)
        print(f"pub const {name}: u64 = {to_hex(val)}; // ln({p})")
    print()

    # Whitening
    print("// WHITENING CONSTANTS")
    for i, p in enumerate(WHITENING_PRIMES):
        val = frac_ln_prime(p)
        print(f"pub const WHITENING{i}: u64 = {to_hex(val)}; // ln({p})")
    print()

    # Lane offsets
    print(f"// LANE OFFSETS: 32 unique offsets for full track diversification")
    print("pub const LANE_OFFSETS: [u64; 32] = [")
    for p in LANE_PRIMES:
        val = frac_ln_prime(p)
        print(f"    {to_hex(val)}, // ln({p})")
    print("];")
    print()

    # Key schedule multiplier
    val = frac_ln_prime(KEY_MULT_PRIME)
    print(f"// KEY SCHEDULE MULTIPLIER: frac(ln({KEY_MULT_PRIME}))")
    print(f"pub const KEY_SCHEDULE_MULT: u64 = {to_hex(val)}; // ln({KEY_MULT_PRIME})")
    print()

    # CLMUL constant
    val = frac_ln_prime(CLMUL_PRIME)
    print(f"// CLMUL CONSTANT: frac(ln({CLMUL_PRIME}))")
    print(f"pub const CLMUL_CONSTANT: u64 = {to_hex(val)}; // ln({CLMUL_PRIME})")
    print()

    # Verification table
    print("// ---- Verification table ----")
    print("// Prime -> hex value")
    for p in sorted(ALL_PRIMES):
        val = frac_ln_prime(p)
        print(f"//   ln({p:3d}) -> {to_hex(val)}")


if __name__ == "__main__":
    main()
