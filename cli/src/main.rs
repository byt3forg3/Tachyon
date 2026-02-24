//! Tachyon CLI
//!
//! High-performance hash command-line tool.

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{check_mode, hash_files, Algorithm};
use std::path::PathBuf;

// =============================================================================
// CLI DEFINITION
// =============================================================================

#[derive(Parser)]
#[command(name = "tachyon")]
#[command(about = "Fast hash function using AVX-512 + VAES", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Files to hash (if no subcommand)
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Hashing algorithm to use
    #[arg(short, long, value_enum, default_value_t = Algorithm::Tachyon)]
    algo: Algorithm,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify checksums from file (like sha256sum -c)
    Check {
        #[arg(value_name = "FILE")]
        checksum_file: PathBuf,
    },
}

// =============================================================================
// ENTRY POINT
// =============================================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Check { checksum_file }) => check_mode(checksum_file)?,
        None => {
            if cli.files.is_empty() {
                eprintln!("Error: No files specified");
                eprintln!("Usage: tachyon [FILE]... or tachyon --help");
                std::process::exit(1);
            }

            hash_files(&cli.files, cli.algo)?;
        }
    }

    Ok(())
}
