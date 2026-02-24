//! Hash Command
//!
//! File hashing with automatic parallelization via Rayon.

use anyhow::{Context, Result};
use clap::ValueEnum;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Algorithm {
    /// Standard Tachyon (Cryptographically hardened, 256-bit)
    Tachyon,
    /// Tachyon Zero (Extreme Performance, 256-bit)
    Zero,
}

enum HasherWrapper {
    Tachyon(tachyon::Hasher),
    Zero(Box<tachyon_zero::Hasher>),
}

impl HasherWrapper {
    fn new(algo: Algorithm) -> Result<Self> {
        match algo {
            Algorithm::Tachyon => {
                let h = tachyon::Hasher::new().map_err(|e| anyhow::anyhow!("{}", e))?;
                Ok(Self::Tachyon(h))
            }
            Algorithm::Zero => {
                let h = tachyon_zero::Hasher::new();
                Ok(Self::Zero(Box::new(h)))
            }
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Tachyon(h) => h.update(data),
            Self::Zero(h) => h.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            Self::Tachyon(h) => h.finalize().to_vec(),
            Self::Zero(h) => h.finalize().to_vec(),
        }
    }
}

/// Hash files (Rayon parallelizes automatically when beneficial).
pub fn hash_files(files: &[PathBuf], algo: Algorithm) -> Result<()> {
    let results = Mutex::new(Vec::with_capacity(files.len()));
    let errors = Mutex::new(Vec::new());

    files.par_iter().for_each(|file_path| {
        let result = (|| -> Result<String> {
            let mut file = std::fs::File::open(file_path)
                .with_context(|| format!("Failed to open: {}", file_path.display()))?;

            let mut hasher = HasherWrapper::new(algo)?;
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
