#!/usr/bin/env node
// Domain separation tests for Node.js bindings

const tachyon = require('./tachyon.node');

function assert(condition, message) {
    if (!condition) {
        throw new Error(`Assertion failed: ${message}`);
    }
}

function testConstants() {
    assert(tachyon.DOMAIN_GENERIC === 0, 'DOMAIN_GENERIC should be 0');
    assert(tachyon.DOMAIN_FILE_CHECKSUM === 1, 'DOMAIN_FILE_CHECKSUM should be 1');
    assert(tachyon.DOMAIN_KEY_DERIVATION === 2, 'DOMAIN_KEY_DERIVATION should be 2');
    assert(tachyon.DOMAIN_MESSAGE_AUTH === 3, 'DOMAIN_MESSAGE_AUTH should be 3');
    assert(tachyon.DOMAIN_DATABASE_INDEX === 4, 'DOMAIN_DATABASE_INDEX should be 4');
    assert(tachyon.DOMAIN_CONTENT_ADDRESSED === 5, 'DOMAIN_CONTENT_ADDRESSED should be 5');
    console.log('✓ Constants defined correctly');
}

function testHashWithDomain() {
    const data = Buffer.from('test data');

    // Different domains produce different hashes
    const h0 = tachyon.hashWithDomain(data, tachyon.DOMAIN_GENERIC);
    const h1 = tachyon.hashWithDomain(data, tachyon.DOMAIN_FILE_CHECKSUM);
    const h2 = tachyon.hashWithDomain(data, tachyon.DOMAIN_KEY_DERIVATION);

    assert(h0.length === 32, 'Hash length should be 32');
    assert(h1.length === 32, 'Hash length should be 32');
    assert(h2.length === 32, 'Hash length should be 32');
    assert(!h0.equals(h1), 'Different domains should produce different hashes');
    assert(!h1.equals(h2), 'Different domains should produce different hashes');
    assert(!h0.equals(h2), 'Different domains should produce different hashes');

    // Same domain produces same hash
    const h0Again = tachyon.hashWithDomain(data, tachyon.DOMAIN_GENERIC);
    assert(h0.equals(h0Again), 'Same domain should produce same hash');

    console.log('✓ hashWithDomain works correctly');
}

function testHashKeyed() {
    const data = Buffer.from('message');
    const key = Buffer.alloc(32, 'k');

    const mac = tachyon.hashKeyed(data, key);
    assert(mac.length === 32, 'MAC length should be 32');

    // Different keys produce different MACs
    const key2 = Buffer.alloc(32, 'x');
    const mac2 = tachyon.hashKeyed(data, key2);
    assert(!mac.equals(mac2), 'Different keys should produce different MACs');

    // Same key + data = same MAC
    const macAgain = tachyon.hashKeyed(data, key);
    assert(mac.equals(macAgain), 'Same key and data should produce same MAC');

    console.log('✓ hashKeyed works correctly');
}

function testVerifyMac() {
    const data = Buffer.from('authenticate this');
    const key = Buffer.alloc(32, 's'); // 32 bytes key

    const mac = tachyon.hashKeyed(data, key);

    // Correct MAC verifies
    assert(tachyon.verifyMac(data, key, mac) === true, 'Valid MAC should verify');

    // Wrong MAC fails
    const wrongMac = Buffer.alloc(32, 'x');
    assert(tachyon.verifyMac(data, key, wrongMac) === false, 'Wrong MAC should not verify');

    // Wrong key fails
    const wrongKey = Buffer.alloc(32, 'w'); // 32 bytes key
    assert(tachyon.verifyMac(data, wrongKey, mac) === false, 'Wrong key should not verify');

    // Wrong data fails
    assert(tachyon.verifyMac(Buffer.from('different data'), key, mac) === false, 'Wrong data should not verify');

    console.log('✓ verifyMac works correctly');
}

function testDeriveKey() {
    const masterKey = Buffer.alloc(32, 'm');

    // Different contexts produce different keys
    const k1 = tachyon.deriveKey(Buffer.from('app-v1'), masterKey);
    const k2 = tachyon.deriveKey(Buffer.from('app-v2'), masterKey);
    const k3 = tachyon.deriveKey(Buffer.from('database'), masterKey);

    assert(k1.length === 32, 'Derived key length should be 32');
    assert(k2.length === 32, 'Derived key length should be 32');
    assert(k3.length === 32, 'Derived key length should be 32');
    assert(!k1.equals(k2), 'Different contexts should produce different keys');
    assert(!k2.equals(k3), 'Different contexts should produce different keys');
    assert(!k1.equals(k3), 'Different contexts should produce different keys');

    // Same context produces same key
    const k1Again = tachyon.deriveKey(Buffer.from('app-v1'), masterKey);
    assert(k1.equals(k1Again), 'Same context should produce same key');

    console.log('✓ deriveKey works correctly');
}

function testStreamingWithDomain() {
    const data = Buffer.from('streaming test data');

    // Hash with domain
    const hasher1 = new tachyon.Hasher(tachyon.DOMAIN_FILE_CHECKSUM);
    hasher1.update(data.slice(0, 10));
    hasher1.update(data.slice(10));
    const h1 = hasher1.finalize();

    assert(h1.length === 32, 'Hash length should be 32');

    // Different domain produces different hash
    const hasher2 = new tachyon.Hasher(tachyon.DOMAIN_KEY_DERIVATION);
    hasher2.update(data.slice(0, 10));
    hasher2.update(data.slice(10));
    const h2 = hasher2.finalize();

    assert(!h1.equals(h2), 'Different domains should produce different hashes');

    // No domain (default)
    const hasher3 = new tachyon.Hasher();
    hasher3.update(data);
    const h3 = hasher3.finalize();

    assert(!h3.equals(h1), 'Default domain should differ from explicit domains');
    assert(!h3.equals(h2), 'Default domain should differ from explicit domains');

    console.log('✓ Streaming with domain works correctly');
}

function testErrorHandling() {
    try {
        // Invalid domain
        tachyon.hashWithDomain(Buffer.from('test'), 99);
        throw new Error('Should have thrown error for invalid domain');
    } catch (e) {
        assert(e.message.includes('Domain must be 0-5'), 'Should check domain range');
    }

    try {
        // Wrong key size
        tachyon.hashKeyed(Buffer.from('data'), Buffer.from('short'));
        throw new Error('Should have thrown error for wrong key size');
    } catch (e) {
        assert(e.message.includes('32 bytes'), 'Should check key size');
    }

    try {
        // Wrong MAC size
        const key = Buffer.alloc(32, 'k');
        tachyon.verifyMac(Buffer.from('data'), key, Buffer.from('short'));
        throw new Error('Should have thrown error for wrong MAC size');
    } catch (e) {
        assert(e.message.includes('32 bytes'), 'Should check MAC size');
    }

    console.log('✓ Error handling works correctly');
}

function testHashSeeded() {
    const data = Buffer.from('Seeded Data');
    const seed1 = BigInt(12345);
    const seed2 = BigInt(67890);

    // Hash output should be Buffer
    const h1 = tachyon.hashSeeded(data, seed1);
    const h2 = tachyon.hashSeeded(data, seed2);
    const h3 = tachyon.hashSeeded(data, seed1);

    assert(h1.length === 32, 'Hash length should be 32');
    assert(!h1.equals(h2), 'Different seeds should produce different hashes');
    assert(h1.equals(h3), 'Same seed should produce same hash');

    console.log('✓ hashSeeded works correctly');

    // Test streaming seeded
    // Note: Hasher constructor takes (domain, seed)
    // Pass null/undefined for domain to use default generic domain with seed
    const hasher1 = new tachyon.Hasher(null, seed1);
    hasher1.update(data);
    const sh1 = hasher1.finalize();
    assert(sh1.equals(h1), 'Streaming seeded hash should match oneshot seeded hash');

    console.log('✓ streaming seeded works correctly');
}

// Run all tests
console.log('Testing Node.js bindings domain separation...\n');
try {
    testConstants();
    testHashWithDomain();
    testHashKeyed();
    testVerifyMac();
    testDeriveKey();
    testStreamingWithDomain();
    testErrorHandling();
    testHashSeeded();
    console.log('\n✅ All Node.js tests passed!');
} catch (e) {
    console.error('\n❌ Test failed:', e.message);
    console.error(e.stack);
    process.exit(1);
}
