/**
 * Tachyon Hash Function - Java Bindings.
 *
 * <p>High-performance cryptographic hash function using AVX-512 + VAES.
 *
 * <p>Example usage:
 * <pre>{@code
 * byte[] data = "Hello, World!".getBytes(StandardCharsets.UTF_8);
 * byte[] hash = Tachyon.hash(data);
 *
 * // Verify in constant time
 * boolean valid = Tachyon.verify(data, hash);
 *
 * // Streaming for large data
 * Hasher hasher = Tachyon.newHasher();
 * hasher.update("chunk 1".getBytes());
 * hasher.update("chunk 2".getBytes());
 * byte[] result = hasher.digest();
 * }</pre>
 */
package com.tachyon;

import java.nio.file.Path;
import java.nio.file.Paths;

/**
 * Tachyon Hash Function.
 *
 * <p>Provides high-performance hashing and constant-time verification.
 */
public class Tachyon {

    // Domain separation constants (aligned with Rust definitions)
    public static final byte DOMAIN_GENERIC = 0;
    public static final byte DOMAIN_FILE_CHECKSUM = 1;
    public static final byte DOMAIN_KEY_DERIVATION = 2;
    public static final byte DOMAIN_MESSAGE_AUTH = 3;
    public static final byte DOMAIN_DATABASE_INDEX = 4;
    public static final byte DOMAIN_CONTENT_ADDRESSED = 5;

    // Load native library
    static {
        try {
            String os = System.getProperty("os.name").toLowerCase();
            String libName = "libtachyon_java.so";
            if (os.contains("mac")) {
                libName = "libtachyon_java.dylib";
            } else if (os.contains("win")) {
                libName = "tachyon_java.dll";
            }

            Path libPath = Paths.get(System.getProperty("user.dir"))
                .resolve("target/release")
                .resolve(libName)
                .toAbsolutePath();

            System.load(libPath.toString());
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Failed to load Tachyon native library: " + e.getMessage());
            throw e;
        }
    }

    // Native methods
    private static native byte[] nativeHash(byte[] input);
    private static native byte[] nativeHashSeeded(byte[] input, long seed);
    private static native boolean nativeVerify(byte[] input, byte[] expectedHash);
    private static native byte[] nativeHashWithDomain(byte[] input, byte domain);
    private static native byte[] nativeHashKeyed(byte[] input, byte[] key);
    private static native boolean nativeVerifyMac(byte[] input, byte[] key, byte[] expectedMac);
    private static native byte[] nativeDeriveKey(byte[] context, byte[] keyMaterial);
    private static native long nativeHasherNew();
    private static native long nativeHasherNewSeeded(long seed);
    private static native long nativeHasherNewWithDomain(byte domain);
    private static native void nativeHasherUpdate(long state, byte[] data);
    private static native byte[] nativeHasherFinalize(long state);
    private static native void nativeHasherFree(long state);

    // =========================================================================
    // ONE-SHOT API
    // =========================================================================

    /**
     * Compute the Tachyon hash of input data.
     *
     * @param input the data to hash
     * @return 32-byte hash
     * @throws IllegalArgumentException if input is null
     */
    public static byte[] hash(byte[] input) {
        if (input == null) {
            throw new IllegalArgumentException("Input cannot be null");
        }
        return nativeHash(input);
    }

    /**
     * Compute the Tachyon hash of input data with a seed.
     *
     * @param input the data to hash
     * @param seed the 64-bit seed
     * @return 32-byte hash
     * @throws IllegalArgumentException if input is null
     */
    public static byte[] hashSeeded(byte[] input, long seed) {
        if (input == null) {
            throw new IllegalArgumentException("Input cannot be null");
        }
        return nativeHashSeeded(input, seed);
    }

    /**
     * Verify data matches expected hash in constant time.
     *
     * <p>This method is timing-attack resistant and should be used for
     * password verification, API key validation, etc.
     *
     * @param input the data to verify
     * @param expectedHash the expected 32-byte hash
     * @return true if hash matches, false otherwise
     * @throws IllegalArgumentException if input or expectedHash is null
     */
    public static boolean verify(byte[] input, byte[] expectedHash) {
        if (input == null || expectedHash == null) {
            throw new IllegalArgumentException("Input and expectedHash cannot be null");
        }
        return nativeVerify(input, expectedHash);
    }

    /**
     * Compute hash with domain separation.
     *
     * @param input the data to hash
     * @param domain the domain value (0-5)
     * @return 32-byte hash
     * @throws IllegalArgumentException if input is null or domain is invalid
     */
    public static byte[] hashWithDomain(byte[] input, byte domain) {
        if (input == null) {
            throw new IllegalArgumentException("Input cannot be null");
        }
        if (domain < 0 || domain > 5) {
            throw new IllegalArgumentException("Domain must be 0-5");
        }
        return nativeHashWithDomain(input, domain);
    }

    /**
     * Compute keyed hash (MAC).
     *
     * @param input the data to hash
     * @param key the 32-byte key
     * @return 32-byte MAC
     * @throws IllegalArgumentException if input or key is null, or key is not 32 bytes
     */
    public static byte[] hashKeyed(byte[] input, byte[] key) {
        if (input == null || key == null) {
            throw new IllegalArgumentException("Input and key cannot be null");
        }
        if (key.length != 32) {
            throw new IllegalArgumentException("Key must be exactly 32 bytes");
        }
        return nativeHashKeyed(input, key);
    }

    /**
     * Verify keyed hash (MAC) in constant time.
     *
     * @param input the data to verify
     * @param key the 32-byte key
     * @param expectedMac the expected 32-byte MAC
     * @return true if MAC matches, false otherwise
     * @throws IllegalArgumentException if any argument is null or wrong size
     */
    public static boolean verifyMac(byte[] input, byte[] key, byte[] expectedMac) {
        if (input == null || key == null || expectedMac == null) {
            throw new IllegalArgumentException("Arguments cannot be null");
        }
        if (key.length != 32) {
            throw new IllegalArgumentException("Key must be exactly 32 bytes");
        }
        if (expectedMac.length != 32) {
            throw new IllegalArgumentException("Expected MAC must be exactly 32 bytes");
        }
        return nativeVerifyMac(input, key, expectedMac);
    }

    /**
     * Derive cryptographic key from material.
     *
     * @param context the context string
     * @param keyMaterial the 32-byte key material
     * @return 32-byte derived key
     * @throws IllegalArgumentException if any argument is null or wrong size
     */
    public static byte[] deriveKey(byte[] context, byte[] keyMaterial) {
        if (context == null || keyMaterial == null) {
            throw new IllegalArgumentException("Arguments cannot be null");
        }
        if (keyMaterial.length != 32) {
            throw new IllegalArgumentException("Key material must be exactly 32 bytes");
        }
        return nativeDeriveKey(context, keyMaterial);
    }

    // =========================================================================
    // STREAMING API
    // =========================================================================

    /**
     * Create a new streaming hasher.
     *
     * @return new Hasher instance
     * @throws RuntimeException if hasher creation fails
     */
    public static Hasher newHasher() {
        return new Hasher(null, null);
    }

    /**
     * Create a new streaming hasher with domain separation.
     *
     * @param domain the domain value (0-5)
     * @return new Hasher instance
     * @throws IllegalArgumentException if domain is invalid
     * @throws RuntimeException if hasher creation fails
     */
    public static Hasher newHasherWithDomain(byte domain) {
        if (domain < 0 || domain > 5) {
            throw new IllegalArgumentException("Domain must be 0-5");
        }
        return new Hasher(domain, null);
    }

    /**
     * Create a new streaming hasher with a seed.
     *
     * @param seed the 64-bit seed
     * @return new Hasher instance
     * @throws RuntimeException if hasher creation fails
     */
    public static Hasher newHasherSeeded(long seed) {
        return new Hasher(null, seed);
    }

    /**
     * Streaming hasher for large data.
     *
     * <p>Example:
     * <pre>{@code
     * Hasher hasher = Tachyon.newHasher();
     * hasher.update("chunk 1".getBytes());
     * hasher.update("chunk 2".getBytes());
     * byte[] hash = hasher.digest();
     * }</pre>
     */
    public static class Hasher implements AutoCloseable {
        private long state;
        private boolean closed = false;



        private Hasher(Byte domain, Long seed) {
            if (seed != null) {
                this.state = nativeHasherNewSeeded(seed);
            } else if (domain != null) {
                this.state = nativeHasherNewWithDomain(domain);
            } else {
                this.state = nativeHasherNew();
            }
            if (this.state == 0) {
                throw new RuntimeException("Failed to create hasher");
            }
        }

        /**
         * Add data to the hasher.
         *
         * @param data bytes to add
         * @throws IllegalStateException if already closed
         * @throws IllegalArgumentException if data is null
         */
        public synchronized void update(byte[] data) {
            if (closed) {
                throw new IllegalStateException("Hasher already closed");
            }
            if (data == null) {
                throw new IllegalArgumentException("Data cannot be null");
            }
            nativeHasherUpdate(state, data);
        }

        /**
         * Compute and return the final hash.
         *
         * @return 32-byte hash
         * @throws IllegalStateException if already closed
         */
        public synchronized byte[] digest() {
            if (closed) {
                throw new IllegalStateException("Hasher already closed");
            }
            closed = true;
            byte[] result = nativeHasherFinalize(state);
            state = 0;
            return result;
        }

        /**
         * Close and release resources.
         */
        @Override
        public synchronized void close() {
            if (state != 0 && !closed) {
                nativeHasherFree(state);
                state = 0;
                closed = true;
            }
        }
    }
}
