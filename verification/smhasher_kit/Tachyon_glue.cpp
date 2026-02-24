/**
 * @file Tachyon_glue.cpp
 * @brief SMHasher integration wrapper for the Tachyon hash function.
 *
 * Copyright (c) byt3forg3 — 260008633+byt3forg3@users.noreply.github.com
 * Licensed under the MIT or Apache 2.0 License.
 *
 * Adapts Tachyon's C API to SMHasher's expected function signature.
 * The hash implementation lives in tachyon/ (pure C, CPUID-dispatched).
 */

#include "tachyon.h"

/**
 * @brief SMHasher-compatible hash function wrapper.
 *
 * @param key    Input data pointer.
 * @param len    Input length in bytes.
 * @param seed   32-bit seed (zero-extended to 64-bit for Tachyon).
 * @param out    Output buffer (must be at least 32 bytes).
 */
void Tachyon_Hash(const void* key, int len, uint32_t seed, void* out) {
    tachyon_hash_seeded(
        static_cast<const uint8_t*>(key),
        static_cast<size_t>(len),
        static_cast<uint64_t>(seed),
        static_cast<uint8_t*>(out)
    );
}
