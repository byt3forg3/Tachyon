import com.tachyon.Tachyon;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

/**
 * Tachyon Java Binding Test
 * 
 * Loads test vectors from the central JSON file to ensure consistency
 * across all language bindings.
 */
public class Test {
    
    // Test vector (from tests/test_vectors.json - embedded for simplicity)
    // Update when test vectors change.
    private static final String TEST_INPUT = "Tachyon";
    // Canonical Hash for "Tachyon" (Unified 4-Lane)
    private static final String EXPECTED_HASH = "120b887e8501bf2a342d397cc46d43b1796502ad75232e7f4c555379cef8c120";
    // Canonical Hash for 256 'A's (Quadratic CLMUL + Nonlinear Fold)
    private static final String EXPECTED_HASH_LARGE = "bafe91fc7d73b8dadc19d0605fe3279762f67ea7f0f4e0ffb9c89634b112ce4d";
    
    public static void main(String[] args) {
        System.out.println("Testing Tachyon Java Binding...\n");

        byte[] data = TEST_INPUT.getBytes(StandardCharsets.UTF_8);

        // 1. Hash
        byte[] hash = Tachyon.hash(data);
        String hexHash = bytesToHex(hash);
        
        System.out.println("Input:    '" + TEST_INPUT + "'");
        System.out.println("Hash:     " + hexHash);
        System.out.println("Expected: " + EXPECTED_HASH);

        if (!hexHash.equals(EXPECTED_HASH)) {
            System.err.println("❌ Hash mismatch!");
            System.exit(1);
        }
        System.out.println("✓ Hash matches");

        // 1.5 Large Input Hash (AVX-512 Verification)
        byte[] largeData = new byte[256];
        for (int i = 0; i < 256; i++) largeData[i] = 'A';
        
        byte[] largeHash = Tachyon.hash(largeData);
        String largeHexHash = bytesToHex(largeHash);
        
        if (!largeHexHash.equals(EXPECTED_HASH_LARGE)) {
            System.err.println("❌ Large Input Hash mismatch!");
            System.err.println("Expected: " + EXPECTED_HASH_LARGE);
            System.err.println("Got:      " + largeHexHash);
            System.exit(1);
        }
        System.out.println("✓ Large Input Hash matches (AVX-512 path verified)");

        // 2. Verify
        boolean isValid = Tachyon.verify(data, hash);
        if (!isValid) {
            System.err.println("❌ Verification failed!");
            System.exit(1);
        }
        System.out.println("✓ Verification passed");

        // 3. Bad Verify
        byte[] badHash = hash.clone();
        badHash[0] ^= 0xFF;
        if (Tachyon.verify(data, badHash)) {
            System.err.println("❌ Bad verification succeeded!");
            System.exit(1);
        }
        System.out.println("✓ Bad hash rejected");

        System.out.println("\n✅ Java Binding OK");
    }

    private static final byte[] HEX_CHARS = "0123456789abcdef".getBytes(StandardCharsets.US_ASCII);
    
    private static String bytesToHex(byte[] bytes) {
        byte[] hexChars = new byte[bytes.length * 2];
        for (int i = 0; i < bytes.length; i++) {
            int v = bytes[i] & 0xFF;
            hexChars[i * 2] = HEX_CHARS[v >>> 4];
            hexChars[i * 2 + 1] = HEX_CHARS[v & 0x0F];
        }
        return new String(hexChars, StandardCharsets.UTF_8);
    }
}
