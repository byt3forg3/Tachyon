//! Official Test Vectors for Tachyon
//!
//! This test verifies the implementation against the canonical JSON test vectors.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
struct Vector {
    hash: String,
    input: String,
    name: String,
}

#[derive(Deserialize)]
struct TestVectors {
    vectors: Vec<Vector>,
}

#[test]
fn test_official_vectors() {
    let file = File::open("tests/test_vectors.json").expect("Failed to open test_vectors.json");
    let reader = BufReader::new(file);
    let data: TestVectors = serde_json::from_reader(reader).expect("Failed to parse JSON");

    println!("\n=== Verifying Official Test Vectors ===");

    for vector in data.vectors {
        let input_bytes = match vector.input.as_str() {
            "LARGE_1KB" => vec![b'A'; 1024],
            "MEDIUM_256_A" => vec![b'A'; 256],
            "HUGE_1MB" => vec![b'A'; 1024 * 1024],
            "EXACT_64_ZERO" => vec![0u8; 64],
            "EXACT_512_ONE" => vec![1u8; 512],
            "UNALIGNED_63_TWO" => vec![2u8; 63],
            val => val.as_bytes().to_vec(),
        };

        let hash = tachyon::hash(&input_bytes);
        let hex_hash = hex::encode(hash);

        assert_eq!(hex_hash, vector.hash, "Vector Mismatched: {}", vector.name);
        println!("✅ {:<16} | {}", vector.name, hex_hash);
    }
    println!("=======================================\n");
}
