import com.tachyon.Tachyon;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

// Domain separation tests for Java bindings
public class TestDomainSeparation {
    
    private static int testsPassed = 0;
    private static int testsFailed = 0;
    
    private static void assertTrue(boolean condition, String message) {
        if (!condition) {
            System.err.println("❌ FAILED: " + message);
            testsFailed++;
            throw new AssertionError(message);
        }
        testsPassed++;
    }
    
    private static void assertFalse(boolean condition, String message) {
        assertTrue(!condition, message);
    }
    
    private static void assertEquals(int expected, int actual, String message) {
        if (expected != actual) {
            System.err.println("❌ FAILED: " + message + " (expected " + expected + ", got " + actual + ")");
            testsFailed++;
            throw new AssertionError(message);
        }
        testsPassed++;
    }
    
    private static void testConstants() {
        System.out.println("Testing constants...");
        assertEquals(0, Tachyon.DOMAIN_GENERIC, "DOMAIN_GENERIC should be 0");
        assertEquals(1, Tachyon.DOMAIN_FILE_CHECKSUM, "DOMAIN_FILE_CHECKSUM should be 1");
        assertEquals(2, Tachyon.DOMAIN_KEY_DERIVATION, "DOMAIN_KEY_DERIVATION should be 2");
        assertEquals(3, Tachyon.DOMAIN_MESSAGE_AUTH, "DOMAIN_MESSAGE_AUTH should be 3");
        assertEquals(4, Tachyon.DOMAIN_DATABASE_INDEX, "DOMAIN_DATABASE_INDEX should be 4");
        assertEquals(5, Tachyon.DOMAIN_CONTENT_ADDRESSED, "DOMAIN_CONTENT_ADDRESSED should be 5");
        System.out.println("✓ Constants defined correctly");
    }
    
    private static void testHashWithDomain() {
        System.out.println("Testing hashWithDomain...");
        byte[] data = "test data".getBytes(StandardCharsets.UTF_8);
        
        // Different domains produce different hashes
        byte[] h0 = Tachyon.hashWithDomain(data, Tachyon.DOMAIN_GENERIC);
        byte[] h1 = Tachyon.hashWithDomain(data, Tachyon.DOMAIN_FILE_CHECKSUM);
        byte[] h2 = Tachyon.hashWithDomain(data, Tachyon.DOMAIN_KEY_DERIVATION);
        
        assertEquals(32, h0.length, "Hash length should be 32");
        assertEquals(32, h1.length, "Hash length should be 32");
        assertEquals(32, h2.length, "Hash length should be 32");
        assertFalse(Arrays.equals(h0, h1), "Different domains should produce different hashes");
        assertFalse(Arrays.equals(h1, h2), "Different domains should produce different hashes");
        assertFalse(Arrays.equals(h0, h2), "Different domains should produce different hashes");
        
        // Same domain produces same hash
        byte[] h0Again = Tachyon.hashWithDomain(data, Tachyon.DOMAIN_GENERIC);
        assertTrue(Arrays.equals(h0, h0Again), "Same domain should produce same hash");
        
        System.out.println("✓ hashWithDomain works correctly");
    }
    
    private static void testHashKeyed() {
        System.out.println("Testing hashKeyed...");
        byte[] data = "message".getBytes(StandardCharsets.UTF_8);
        byte[] key = new byte[32];
        Arrays.fill(key, (byte) 'k');
        
        byte[] mac = Tachyon.hashKeyed(data, key);
        assertEquals(32, mac.length, "MAC length should be 32");
        
        // Different keys produce different MACs
        byte[] key2 = new byte[32];
        Arrays.fill(key2, (byte) 'x');
        byte[] mac2 = Tachyon.hashKeyed(data, key2);
        assertFalse(Arrays.equals(mac, mac2), "Different keys should produce different MACs");
        
        // Same key + data = same MAC
        byte[] macAgain = Tachyon.hashKeyed(data, key);
        assertTrue(Arrays.equals(mac, macAgain), "Same key and data should produce same MAC");
        
        System.out.println("✓ hashKeyed works correctly");
    }
    
    private static void testVerifyMac() {
        System.out.println("Testing verifyMac...");
        byte[] data = "authenticate this".getBytes(StandardCharsets.UTF_8);
        byte[] key = new byte[32];
        Arrays.fill(key, (byte) 's');
        
        byte[] mac = Tachyon.hashKeyed(data, key);
        
        // Correct MAC verifies
        assertTrue(Tachyon.verifyMac(data, key, mac), "Valid MAC should verify");
        
        // Wrong MAC fails
        byte[] wrongMac = new byte[32];
        Arrays.fill(wrongMac, (byte) 'x');
        assertFalse(Tachyon.verifyMac(data, key, wrongMac), "Wrong MAC should not verify");
        
        // Wrong key fails
        byte[] wrongKey = new byte[32];
        Arrays.fill(wrongKey, (byte) 'w');
        assertFalse(Tachyon.verifyMac(data, wrongKey, mac), "Wrong key should not verify");
        
        // Wrong data fails
        byte[] wrongData = "different data".getBytes(StandardCharsets.UTF_8);
        assertFalse(Tachyon.verifyMac(wrongData, key, mac), "Wrong data should not verify");
        
        System.out.println("✓ verifyMac works correctly");
    }
    
    private static void testDeriveKey() {
        System.out.println("Testing deriveKey...");
        byte[] masterKey = new byte[32];
        Arrays.fill(masterKey, (byte) 'm');
        
        // Different contexts produce different keys
        byte[] k1 = Tachyon.deriveKey("app-v1".getBytes(StandardCharsets.UTF_8), masterKey);
        byte[] k2 = Tachyon.deriveKey("app-v2".getBytes(StandardCharsets.UTF_8), masterKey);
        byte[] k3 = Tachyon.deriveKey("database".getBytes(StandardCharsets.UTF_8), masterKey);
        
        assertEquals(32, k1.length, "Derived key length should be 32");
        assertEquals(32, k2.length, "Derived key length should be 32");
        assertEquals(32, k3.length, "Derived key length should be 32");
        assertFalse(Arrays.equals(k1, k2), "Different contexts should produce different keys");
        assertFalse(Arrays.equals(k2, k3), "Different contexts should produce different keys");
        assertFalse(Arrays.equals(k1, k3), "Different contexts should produce different keys");
        
        // Same context produces same key
        byte[] k1Again = Tachyon.deriveKey("app-v1".getBytes(StandardCharsets.UTF_8), masterKey);
        assertTrue(Arrays.equals(k1, k1Again), "Same context should produce same key");
        
        System.out.println("✓ deriveKey works correctly");
    }
    
    private static void testStreamingWithDomain() {
        System.out.println("Testing streaming with domain...");
        byte[] data = "streaming test data".getBytes(StandardCharsets.UTF_8);
        
        // Hash with domain
        Tachyon.Hasher hasher1 = Tachyon.newHasherWithDomain(Tachyon.DOMAIN_FILE_CHECKSUM);
        hasher1.update(Arrays.copyOfRange(data, 0, 10));
        hasher1.update(Arrays.copyOfRange(data, 10, data.length));
        byte[] h1 = hasher1.digest();
        
        assertEquals(32, h1.length, "Hash length should be 32");
        
        // Different domain produces different hash
        Tachyon.Hasher hasher2 = Tachyon.newHasherWithDomain(Tachyon.DOMAIN_KEY_DERIVATION);
        hasher2.update(Arrays.copyOfRange(data, 0, 10));
        hasher2.update(Arrays.copyOfRange(data, 10, data.length));
        byte[] h2 = hasher2.digest();
        
        assertFalse(Arrays.equals(h1, h2), "Different domains should produce different hashes");
        
        // No domain (default)
        Tachyon.Hasher hasher3 = Tachyon.newHasher();
        hasher3.update(data);
        byte[] h3 = hasher3.digest();
        
        assertFalse(Arrays.equals(h3, h1), "Default domain should differ from explicit domains");
        assertFalse(Arrays.equals(h3, h2), "Default domain should differ from explicit domains");
        
        System.out.println("✓ Streaming with domain works correctly");
    }

    private static void testHashSeeded() {
        System.out.println("Testing hashSeeded...");
        byte[] data = "Seeded Data".getBytes(StandardCharsets.UTF_8);
        long seed1 = 12345L;
        long seed2 = 67890L;

        byte[] h1 = Tachyon.hashSeeded(data, seed1);
        byte[] h2 = Tachyon.hashSeeded(data, seed2);
        byte[] h3 = Tachyon.hashSeeded(data, seed1);

        assertEquals(32, h1.length, "Hash length should be 32");

        // Different seeds produce different hashes
        assertFalse(Arrays.equals(h1, h2), "Different seeds should produce different hashes");

        // Same seed produces same hash
        assertTrue(Arrays.equals(h1, h3), "Same seed should produce same hash");

        // Streaming seeded
        Tachyon.Hasher hasher = Tachyon.newHasherSeeded(seed1);
        hasher.update(data);
        byte[] sh1 = hasher.digest();
        
        assertTrue(Arrays.equals(h1, sh1), "Streaming seeded hash should match oneshot seeded hash");

        System.out.println("✓ hashSeeded works correctly");
    }
    
    private static void testErrorHandling() {
        System.out.println("Testing error handling...");
        
        // Invalid domain
        try {
            Tachyon.hashWithDomain("test".getBytes(), (byte) 99);
            throw new AssertionError("Should have thrown exception for invalid domain");
        } catch (IllegalArgumentException e) {
            assertTrue(e.getMessage().contains("Domain must be 0-5"), "Should check domain range");
        }
        
        // Wrong key size
        try {
            Tachyon.hashKeyed("data".getBytes(), "short".getBytes());
            throw new AssertionError("Should have thrown exception for wrong key size");
        } catch (IllegalArgumentException e) {
            assertTrue(e.getMessage().contains("32 bytes"), "Should check key size");
        }
        
        // Wrong MAC size
        try {
            byte[] key = new byte[32];
            Tachyon.verifyMac("data".getBytes(), key, "short".getBytes());
            throw new AssertionError("Should have thrown exception for wrong MAC size");
        } catch (IllegalArgumentException e) {
            assertTrue(e.getMessage().contains("32 bytes"), "Should check MAC size");
        }
        
        System.out.println("✓ Error handling works correctly");
    }
    
    public static void main(String[] args) {
        System.out.println("Testing Java bindings domain separation...\n");
        
        try {
            testConstants();
            testHashWithDomain();
            testHashKeyed();
            testVerifyMac();
            testDeriveKey();
            testStreamingWithDomain();
            testHashSeeded();
            testErrorHandling();
            
            System.out.println("\n✅ All Java tests passed!");
            System.out.println("Tests passed: " + testsPassed);
            System.out.println("Tests failed: " + testsFailed);
        } catch (Exception e) {
            System.err.println("\n❌ Test suite failed!");
            System.err.println("Tests passed: " + testsPassed);
            System.err.println("Tests failed: " + testsFailed);
            e.printStackTrace();
            System.exit(1);
        }
    }
}
