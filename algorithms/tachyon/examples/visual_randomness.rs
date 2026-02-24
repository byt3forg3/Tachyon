//! Visual Randomness Generator example.
//!
//! This example generates two PPM images to visually verify both kernel paths:
//! 1. AES-NI Path (Small Input): Hashes 16-byte coordinates directly.
//! 2. AVX-512 Path (Large Input): Hashes 256-byte padded coordinates.
//!
//! Generates:
//! - `tachyon_randomness_aesni.bmp`
//! - `tachyon_randomness_avx512.bmp`

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::unnecessary_cast)]

use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> std::io::Result<()> {
    let width = 1024;
    let height = 1024;

    // --- 1. AES-NI PATH (Small Input) ---
    println!(" Generating AES-NI Randomness Map (16 bytes)...");
    generate_image("tachyon_randomness_aesni.bmp", width, height, |x, y| {
        let mut buf = [0u8; 16];
        buf[0..8].copy_from_slice(&(x as u64).to_le_bytes());
        buf[8..16].copy_from_slice(&(y as u64).to_le_bytes());
        tachyon::hash(&buf)
    })?;

    // --- 2. AVX-512 PATH (Large Input) ---
    println!(" Generating AVX-512 Randomness Map (256 bytes)...");
    generate_image("tachyon_randomness_avx512.bmp", width, height, |x, y| {
        let mut buf = [0u8; 256]; // Large enough to force AVX-512 path
                                  // Mix coordinates into the buffer multiple times to ensure non-trivial input
        buf[0..8].copy_from_slice(&(x as u64).to_le_bytes());
        buf[8..16].copy_from_slice(&(y as u64).to_le_bytes());
        buf[128..136].copy_from_slice(&(x as u64).to_le_bytes()); // Repeat in second lane
        buf[136..144].copy_from_slice(&(y as u64).to_le_bytes());
        tachyon::hash(&buf)
    })?;

    println!("✅ Done! Generated two images.");
    Ok(())
}

fn generate_image<F>(filename: &str, width: u32, height: u32, hasher: F) -> std::io::Result<()>
where
    F: Fn(u32, u32) -> [u8; 32],
{
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);

    // BMP Header
    let file_size = 54 + (width * height * 3); // 54B header + pixels (3 bytes/pixel)
    let reserved = 0;
    let offset = 54;
    let header_size = 40;
    let planes = 1u16;
    let bpp = 24u16; // 24 bits per pixel (RGB)
    let compression = 0;
    let image_size = width * height * 3;
    let x_ppm = 0;
    let y_ppm = 0;
    let colors_used = 0;
    let colors_important = 0;

    // Write File Header (14 bytes)
    writer.write_all(b"BM")?;
    writer.write_all(&(file_size as u32).to_le_bytes())?;
    writer.write_all(&(reserved as u32).to_le_bytes())?;
    writer.write_all(&(offset as u32).to_le_bytes())?;

    // Write Info Header (40 bytes)
    writer.write_all(&(header_size as u32).to_le_bytes())?;
    writer.write_all(&(width as i32).to_le_bytes())?;
    // Negative height for top-down image (standard for our looping)
    writer.write_all(&(-(height as i32)).to_le_bytes())?;
    writer.write_all(&planes.to_le_bytes())?;
    writer.write_all(&bpp.to_le_bytes())?;
    writer.write_all(&(compression as u32).to_le_bytes())?;
    writer.write_all(&(image_size as u32).to_le_bytes())?;
    writer.write_all(&(x_ppm as i32).to_le_bytes())?;
    writer.write_all(&(y_ppm as i32).to_le_bytes())?;
    writer.write_all(&(colors_used as u32).to_le_bytes())?;
    writer.write_all(&(colors_important as u32).to_le_bytes())?;

    for y in 0..height {
        for x in 0..width {
            let hash = hasher(x, y);

            // Use first 3 bytes as RGB
            let r = hash[0];
            let g = hash[1];
            let b = hash[2];

            writer.write_all(&[b, g, r])?;
        }
    }

    Ok(())
}
