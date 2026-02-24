#include "../../c-reference/tachyon_impl.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * Unified Tachyon Test Wrapper
 * 
 * Supports both One-shot and Streaming API tests with varying parameters.
 * Reads from stdin and outputs hex hash to stdout.
 * 
 * Protocol:
 * 1. [1 byte] Mode
 *    - 0x00..0x04: One-shot (Standard, Seeded, Keyed, Domain, Full)
 *    - 0x10..0x14: Streaming (Standard, Seeded, Keyed, Domain, Full)
 * 2. [Optional Parameters] 
 *    - Seed: 8 bytes LE
 *    - Domain: 8 bytes LE
 *    - Key: 32 bytes
 * 3. [Rest] Input Data
 */

static void read_exact(void *buf, size_t len) {
    if (fread(buf, 1, len, stdin) != len) {
        fprintf(stderr, "Failed to read parameters from stdin\n");
        exit(1);
    }
}

static void bytes_to_hex(const uint8_t *bytes, size_t len) {
    for (size_t i = 0; i < len; i++) {
        printf("%02x", bytes[i]);
    }
}

int main() {
    uint8_t mode_byte;
    if (fread(&mode_byte, 1, 1, stdin) != 1) return 0;

    int is_streaming = (mode_byte & 0x10) != 0;
    int mode = mode_byte & 0x0F;

    uint64_t seed = 0;
    uint64_t domain = 0;
    uint8_t key[32] = {0};
    int has_key = 0;

    // Read parameters based on specific mode bits (or range)
    if (mode == 1) { // Seeded
        read_exact(&seed, 8);
    } else if (mode == 2) { // Keyed
        read_exact(key, 32);
        has_key = 1;
    } else if (mode == 3) { // Domain
        read_exact(&domain, 8);
    } else if (mode == 4) { // Full
        read_exact(&domain, 8);
        read_exact(&seed, 8);
        read_exact(key, 32);
        has_key = 1;
    }

    uint8_t hash[32];

    if (is_streaming) {
        // --- STREAMING PATH ---
        tachyon_state_t *s = tachyon_hasher_new_full(domain, seed, has_key ? key : NULL);
        if (!s) return 1;

        uint8_t buffer[65536];
        while (1) {
            size_t n = fread(buffer, 1, sizeof(buffer), stdin);
            if (n == 0) break;
            tachyon_hasher_update(s, buffer, n);
        }
        tachyon_hasher_finalize(s, hash);
    } else {
        // --- ONE-SHOT PATH ---
        size_t capacity = 1024 * 1024;
        size_t len = 0;
        uint8_t *buffer = (uint8_t*)malloc(capacity);
        if (!buffer) return 1;

        while (1) {
            if (len + 65536 > capacity) {
                capacity *= 2;
                uint8_t *new_buf = (uint8_t*)realloc(buffer, capacity);
                if (!new_buf) { free(buffer); return 1; }
                buffer = new_buf;
            }
            size_t n = fread(buffer + len, 1, 65536, stdin);
            if (n == 0) break;
            len += n;
        }

        int res = 0;
        switch (mode) {
            case 0: res = tachyon_hash(buffer, len, hash); break;
            case 1: res = tachyon_hash_seeded(buffer, len, seed, hash); break;
            case 2: res = tachyon_hash_keyed(buffer, len, key, hash); break;
            case 3: res = tachyon_hash_with_domain(buffer, len, domain, hash); break;
            case 4: res = tachyon_hash_full(buffer, len, domain, seed, key, hash); break;
            default: fprintf(stderr, "Unknown one-shot mode %d\n", mode); free(buffer); return 1;
        }

        free(buffer);
        if (res != 0) return 1;
    }

    bytes_to_hex(hash, 32);
    return 0;
}
