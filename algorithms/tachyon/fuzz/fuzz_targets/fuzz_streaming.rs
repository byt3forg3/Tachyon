#![no_main]

use libfuzzer_sys::fuzz_target;
use tachyon::{hash_with_domain, Hasher, TachyonDomain};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    // Determine chunk size from first byte (1 to 255)
    let chunk_size = (data[0] as usize % 255) + 1;

    // Calculate one-shot hash as reference (uses Generic domain 0)
    let reference_hash = tachyon::hash(data);

    // Calculate streaming hash by splitting into arbitrary small chunks
    // 0 is the TachyonDomain::Generic ID
    let mut hasher = Hasher::new_full(0, 0).unwrap();

    // Chunk size is derived from second byte (1 to 255)
    let chunk_size = if data.len() > 1 {
        (data[1] as usize % 255) + 1
    } else {
        1
    };

    for chunk in data.chunks(chunk_size) {
        hasher.update(chunk);
    }

    let streaming_hash = hasher.finalize();

    // They must be identical
    assert_eq!(
        reference_hash, streaming_hash,
        "Streaming and One-Shot approaches differ!"
    );
});
