//! Check Command
//!
//! Verify checksums from file (like sha256sum -c).

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

// =============================================================================
// CHECK
// =============================================================================

/// Verify checksums from a checksum file.
pub fn check_mode(checksum_file: &PathBuf) -> Result<()> {
    let file = File::open(checksum_file)
        .with_context(|| format!("Failed to open: {}", checksum_file.display()))?;

    let reader = BufReader::new(file);
    let mut total = 0;
    let mut failed = 0;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Format: "hash  filename" (two spaces)
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        if parts.len() != 2 {
            eprintln!("Warning: Invalid format: {}", line);
            continue;
        }

        let expected_hash = parts[0].trim();
        let file_path = parts[1].trim();
        total += 1;

        match std::fs::File::open(file_path) {
            Ok(mut file) => {
                let mut hasher = match tachyon::Hasher::new() {
                    Ok(h) => h,
                    Err(e) => {
                        println!("{}: FAILED (CPU Error: {})", file_path, e);
                        failed += 1;
                        continue;
                    }
                };

                let mut buffer = [0u8; 128 * 1024];
                let mut error = None;

                loop {
                    match std::io::Read::read(&mut file, &mut buffer) {
                        Ok(0) => break,
                        Ok(n) => hasher.update(&buffer[..n]),
                        Err(e) => {
                            error = Some(e);
                            break;
                        }
                    }
                }

                if let Some(e) = error {
                    println!("{}: FAILED (Read Error: {})", file_path, e);
                    failed += 1;
                    continue;
                }

                let actual_hash = hex::encode(hasher.finalize());

                if actual_hash == expected_hash {
                    println!("{}: OK", file_path);
                } else {
                    println!("{}: FAILED", file_path);
                    failed += 1;
                }
            }
            Err(e) => {
                println!("{}: FAILED ({})", file_path, e);
                failed += 1;
            }
        }
    }

    println!();
    if failed == 0 {
        println!("All {} checksums verified", total);
    } else {
        eprintln!("WARNING: {} of {} checksums did NOT match", failed, total);
        std::process::exit(1);
    }

    Ok(())
}
