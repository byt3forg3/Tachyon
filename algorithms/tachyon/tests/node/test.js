/**
 * Tachyon Node.js Binding Test
 * 
 * Loads test vectors from the central JSON file to ensure consistency
 * across all language bindings.
 */
const assert = require('assert');
const path = require('path');
const fs = require('fs');

// Load test vectors
const vectorsPath = path.join(__dirname, '../test_vectors.json');
const vectors = JSON.parse(fs.readFileSync(vectorsPath, 'utf8'));

// Load the native addon
const addonPath = path.join(__dirname, '../../../../bindings/node/tachyon.node');
// console.log(`Loading addon from: ${addonPath}`);

let tachyon;
try {
    tachyon = require(addonPath);
} catch (e) {
    console.error("Failed to load native addon:", e);
    process.exit(1);
}

console.log("Testing Tachyon Node.js Binding...\n");

for (const vec of vectors.vectors) {
    console.log(`\n[Test Case: ${vec.name}]`);

    // Expand placeholders to actual data
    let data;
    if (vec.input === "LARGE_1KB") {
        data = Buffer.alloc(1024, 0x41);  // 1KB of 'A'
    } else if (vec.input === "MEDIUM_256_A") {
        data = Buffer.alloc(256, 0x41);   // 256 bytes of 'A'
    } else if (vec.input === "HUGE_1MB") {
        data = Buffer.alloc(1024 * 1024, 0x41);  // 1MB of 'A'
    } else if (vec.input === "EXACT_64_ZERO") {
        data = Buffer.alloc(64, 0x00);
    } else if (vec.input === "EXACT_512_ONE") {
        data = Buffer.alloc(512, 0x01);
    } else if (vec.input === "UNALIGNED_63_TWO") {
        data = Buffer.alloc(63, 0x02);
    } else {
        data = Buffer.from(vec.input);
    }
    const expected = vec.hash;

    // 1. Hash
    const hash = tachyon.hash(data);
    console.log(`  Input len: ${data.length}`);
    console.log(`  Hash:      ${hash.toString('hex')}`);
    console.log(`  Expected:  ${expected}`);

    assert.strictEqual(hash.toString('hex'), expected, `Hash mismatch for ${vec.name}`);
    console.log("  ✓ Hash matches");

    // 2. Verify
    const isValid = tachyon.verify(data, hash);
    assert.strictEqual(isValid, true, `Verification failed for ${vec.name}`);
    console.log("  ✓ Verification passed");

    // 3. Bad Verify
    const badHash = Buffer.from(hash);
    badHash[0] ^= 0xFF;
    const isInvalid = tachyon.verify(data, badHash);
    assert.strictEqual(isInvalid, false, `Bad verification succeeded for ${vec.name}`);
    console.log("  ✓ Bad hash rejected");
}

console.log("\n✅ Node.js Binding OK (All vectors passed)");
