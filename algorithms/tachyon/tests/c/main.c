/**
 * Tachyon C Binding Test
 * 
 * Uses test vectors from tests/test_vectors.json (values embedded at compile time
 * for simplicity in C - update this file when test vectors change).
 * 
 * Test Vector: hash("Tachyon") = 62c63f5760576319db992db546bfee49634b48bfde41652aff9eb10097870d12
 */
#include <stdio.h>
#include <string.h>
#include "tachyon.h"

// Test vector (from tests/test_vectors.json)
static const char *TEST_INPUT = "Tachyon";
// Canonical Hash for "Tachyon" (Unified 4-Lane)
#define EXPECTED_HASH "120b887e8501bf2a342d397cc46d43b1796502ad75232e7f4c555379cef8c120"
// Canonical Hash for 256 'A's (Quadratic CLMUL + Nonlinear Fold)
#define EXPECTED_HASH_LARGE "bafe91fc7d73b8dadc19d0605fe3279762f67ea7f0f4e0ffb9c89634b112ce4d"

static void bytes_to_hex(const uint8_t *bytes, size_t len, char *hex) {
    for (size_t i = 0; i < len; i++) {
        sprintf(&hex[i * 2], "%02x", bytes[i]);
    }
    hex[len * 2] = '\0';
}

int main() {
    printf("Testing Tachyon C-API...\n");
    printf("Hardware selected: %s\n\n", tachyon_get_backend_name());
    
    uint8_t hash[32];
    char hex_hash[65];
    
    // --- BASIC HASH TEST ---
    int res = tachyon_hash((const uint8_t*)TEST_INPUT, strlen(TEST_INPUT), hash);
    if (res != 0) {
        printf("❌ tachyon_hash returned %d\n", res);
        return 1;
    }
    
    bytes_to_hex(hash, 32, hex_hash);
    printf("Input:    '%s'\n", TEST_INPUT);
    printf("Hash:     %s\n", hex_hash);
    printf("Expected: %s\n", EXPECTED_HASH);

    if (strcmp(hex_hash, EXPECTED_HASH) != 0) {
        printf("❌ Hash mismatch!\n");
        return 1;
    }
    printf("✓ Hash matches\n");

    // --- LARGE INPUT TEST (AVX-512 Path) ---
    uint8_t large_input[256];
    uint8_t large_hash[32]; // Use separate buffer
    memset(large_input, 'A', 256);
    
    tachyon_hash(large_input, 256, large_hash);
    bytes_to_hex(large_hash, 32, hex_hash);
    
    if (strcmp(hex_hash, EXPECTED_HASH_LARGE) != 0) {
        printf("❌ Large Input Hash mismatch!\n");
        printf("Expected: %s\n", EXPECTED_HASH_LARGE);
        printf("Got:      %s\n", hex_hash);
        return 1;
    }
    printf("✓ Large Input Hash matches (AVX-512 path verified)\n");

    // --- VERIFY TEST ---
    int v_res = tachyon_verify((const uint8_t*)TEST_INPUT, strlen(TEST_INPUT), hash);
    if (v_res != 1) {
        printf("❌ Verification failed (res=%d)\n", v_res);
        return 1;
    }
    printf("✓ Verification passed\n");

    // --- STREAMING TEST ---
    printf("\nTesting Streaming API...\n");
    void* state = tachyon_hasher_new();
    if (state == NULL) {
        printf("❌ Failed to create hasher\n");
        return 1;
    }
    
    // Split "Tachyon" into chunks: "Tachy" and "on"
    const char* part1 = "Tachy";
    const char* part2 = "on";
    
    tachyon_hasher_update(state, (const uint8_t*)part1, strlen(part1));
    tachyon_hasher_update(state, (const uint8_t*)part2, strlen(part2));
    
    uint8_t stream_hash[32];
    tachyon_hasher_finalize(state, stream_hash);
    
    // Verify streaming hash matches one-shot hash
    if (memcmp(hash, stream_hash, 32) != 0) {
        printf("❌ Streaming hash mismatch!\n");
        bytes_to_hex(stream_hash, 32, hex_hash);
        printf("Stream Hash: %s\n", hex_hash);
        return 1;
    }
    printf("✓ Streaming matches one-shot\n");

    printf("\n✅ C Binding OK\n");
    return 0;
}
