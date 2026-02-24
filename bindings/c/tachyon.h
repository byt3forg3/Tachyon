/**
 * @file tachyon.h
 * @brief Tachyon Hash Function - C API
 *
 * High-performance cryptographically hardened hash using AVX-512 + VAES.
 *
 * @example
 * ```c
 * #include "tachyon.h"
 *
 * uint8_t hash[32];
 * const char* data = "Hello, World!";
 * int res = tachyon_hash((const uint8_t*)data, strlen(data), hash);
 * if (res == 0) {
 *     // Success
 * }
 * ```
 */

#ifndef TACHYON_H
#define TACHYON_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * DOMAIN CONSTANTS
 * ============================================================================ */

#define TACHYON_DOMAIN_GENERIC           0
#define TACHYON_DOMAIN_FILE_CHECKSUM     1
#define TACHYON_DOMAIN_KEY_DERIVATION    2
#define TACHYON_DOMAIN_MESSAGE_AUTH      3
#define TACHYON_DOMAIN_DATABASE_INDEX    4
#define TACHYON_DOMAIN_CONTENT_ADDRESSED 5

/* ============================================================================
 * ONE-SHOT API
 * ============================================================================ */

/**
 * @brief Compute Tachyon hash of a buffer.
 *
 * @param input_ptr  Pointer to input data.
 * @param input_len  Length of input in bytes.
 * @param output_ptr Pointer to 32-byte output buffer (caller-allocated).
 *
 * @return 0 on success, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_hash(const uint8_t *input_ptr, size_t input_len, uint8_t *output_ptr);

/**
 * @brief Compute Tachyon hash with a seed.
 *
 * @param input_ptr  Pointer to input data.
 * @param input_len  Length of input in bytes.
 * @param seed       64-bit seed value.
 * @param output_ptr Pointer to 32-byte output buffer (caller-allocated).
 *
 * @return 0 on success, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_hash_seeded(const uint8_t *input_ptr, size_t input_len, uint64_t seed, uint8_t *output_ptr);

/**
 * @brief Verify hash in constant time (timing-attack resistant).
 *
 * Use for password verification, API key validation, etc.
 *
 * @param input_ptr Pointer to input data.
 * @param input_len Length of input in bytes.
 * @param hash_ptr  Pointer to expected 32-byte hash.
 *
 * @return 1 if match, 0 if mismatch, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_verify(const uint8_t *input_ptr, size_t input_len, const uint8_t *hash_ptr);

/**
 * @brief Hash with domain separation.
 *
 * @param input_ptr  Pointer to input data.
 * @param input_len  Length of input in bytes.
 * @param domain     Domain ID (use TACHYON_DOMAIN_* constants).
 * @param output_ptr Pointer to 32-byte output buffer.
 *
 * @return 0 on success, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_hash_with_domain(const uint8_t *input_ptr, size_t input_len, uint64_t domain, uint8_t *output_ptr);

/**
 * @brief Compute keyed hash (MAC).
 *
 * @param input_ptr  Pointer to input data.
 * @param input_len  Length of input in bytes.
 * @param key_ptr    Pointer to 32-byte key.
 * @param output_ptr Pointer to 32-byte output buffer.
 *
 * @return 0 on success, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_hash_keyed(const uint8_t *input_ptr, size_t input_len, const uint8_t *key_ptr, uint8_t *output_ptr);

/**
 * @brief Verify keyed hash (MAC) in constant time.
 *
 * @param input_ptr Pointer to input data.
 * @param input_len Length of input in bytes.
 * @param key_ptr   Pointer to 32-byte key.
 * @param hash_ptr  Pointer to expected 32-byte MAC.
 *
 * @return 1 if match, 0 if mismatch, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_verify_mac(const uint8_t *input_ptr, size_t input_len, const uint8_t *key_ptr, const uint8_t *hash_ptr);

/**
 * @brief Derive key from context string and key material.
 *
 * @param context_ptr      Pointer to context string (UTF-8).
 * @param context_len      Length of context string.
 * @param key_material_ptr Pointer to 32-byte key material.
 * @param output_ptr       Pointer to 32-byte derived key output.
 *
 * @return 0 on success, -1 on null pointer, -2 on internal error.
 */
int32_t tachyon_derive_key(const uint8_t *context_ptr, size_t context_len, const uint8_t *key_material_ptr, uint8_t *output_ptr);

/**
 * @brief Get the name of the hardware backend currently in use.
 *
 * @return String name of the backend (AVX-512, AES-NI, or Portable).
 */
const char* tachyon_get_backend_name(void);

/* ============================================================================
 * STREAMING API
 * ============================================================================ */

/**
 * @brief Create a new streaming hasher.
 *
 * @return Opaque pointer to hasher state, or NULL on error.
 *         Must be freed with tachyon_hasher_finalize() or tachyon_hasher_free().
 */
void* tachyon_hasher_new(void);

/**
 * @brief Create a new streaming hasher with domain separation.
 *
 * @param domain Domain ID (use TACHYON_DOMAIN_* constants).
 *
 * @return Opaque pointer to hasher state, or NULL on error.
 */
void* tachyon_hasher_new_with_domain(uint64_t domain);

/**
 * @brief Create a new streaming hasher with a seed.
 *
 * @param seed 64-bit seed value.
 *
 * @return Opaque pointer to hasher state, or NULL on error.
 */
void* tachyon_hasher_new_seeded(uint64_t seed);

/**
 * @brief Add data to the hasher.
 *
 * @param state Hasher state from tachyon_hasher_new().
 * @param data  Pointer to input data.
 * @param len   Length of data in bytes.
 */
void tachyon_hasher_update(void* state, const uint8_t* data, size_t len);

/**
 * @brief Finalize and get hash. Frees the hasher state.
 *
 * @param state   Hasher state (consumed, do not use after this call).
 * @param out_ptr Pointer to 32-byte output buffer.
 */
void tachyon_hasher_finalize(void* state, uint8_t* out_ptr);

/**
 * @brief Free hasher without finalizing (if needed).
 *
 * @param state Hasher state to free.
 */
void tachyon_hasher_free(void* state);

#ifdef __cplusplus
}
#endif

#endif /* TACHYON_H */
