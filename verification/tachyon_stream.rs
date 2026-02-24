//! # `PractRand` Stream Generator
//!
//! High-performance stream generator for `PractRand` testing.
//!
//! This binary generates a continuous stream of binary data by hashing an
//! incrementing 64-bit counter with the Tachyon hash function.

use std::io::{self, Write};

/// Entry point for the `PractRand` stream generator.
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Default to 64 bytes (AVX-512) if no argument is provided
    let mode = args.get(1).map_or("64", String::as_str);

    let mut counter: u64 = 0;
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout.lock());

    loop {
        // Determine input size based on mode
        let size = match mode {
            "cyclic" => {
                // Cycle through 16, 32, 64, 128 bytes
                match counter % 4 {
                    0 => 16,
                    1 => 32,
                    2 => 64,
                    _ => 128,
                }
            }
            s => s.parse::<usize>().unwrap_or(64),
        };

        let mut input = vec![0u8; size];
        let counter_bytes = counter.to_le_bytes();

        // Fill input with the counter (repeatedly if needed)
        for (i, item) in input.iter_mut().enumerate() {
            *item = counter_bytes[i % 8];
        }

        let hash = tachyon::hash(&input);

        if handle.write_all(&hash).is_err() {
            break;
        }

        counter = counter.wrapping_add(1);
    }
}
