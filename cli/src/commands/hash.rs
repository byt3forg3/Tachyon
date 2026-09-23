//! Hash Command
//!
//! File hashing with automatic parallelization via Rayon.

use anyhow::{Context, Result};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

// =============================================================================
// HASHING
// =============================================================================

/// Hash files (Rayon parallelizes automatically when beneficial).
pub fn hash_files(files: &[PathBuf]) -> Result<()> {
    let results = Mutex::new(Vec::with_capacity(files.len()));
    let errors = Mutex::new(Vec::new());

    files.par_iter().for_each(|file_path| {
        let result = (|| -> Result<String> {
            let mut file = std::fs::File::open(file_path)
                .with_context(|| format!("Failed to open: {}", file_path.display()))?;

            let mut hasher = tachyon::Hasher::new();
            let mut buffer = [0u8; 128 * 1024]; // 128 KB buffer

            loop {
                let n = std::io::Read::read(&mut file, &mut buffer)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }

            let hash = hasher.finalize();
            Ok(hex::encode(hash))
        })();

        match result {
            Ok(hex_hash) => {
                results.lock().unwrap().push((file_path.clone(), hex_hash));
            }
            Err(e) => {
                errors.lock().unwrap().push((file_path.clone(), e));
            }
        }
    });

    // Print in original order
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(path, _)| files.iter().position(|p| p == path).unwrap_or(usize::MAX));

    for (file_path, hex_hash) in results {
        println!("{}  {}", hex_hash, file_path.display());
    }

    let errors = errors.into_inner().unwrap();
    for (file_path, error) in &errors {
        eprintln!("Error: {}: {}", file_path.display(), error);
    }

    if !errors.is_empty() {
        anyhow::bail!("Failed to hash {} file(s)", errors.len());
    }

    Ok(())
}
