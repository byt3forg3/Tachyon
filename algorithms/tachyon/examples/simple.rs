//! Tachyon Basic Example
//!
//! Minimal usage: `let hash = tachyon::hash(&data);`

#![allow(clippy::pedantic, clippy::nursery)]

fn main() {
    // Zero boilerplate:
    let data = b"Hello, World!";
    let hash = tachyon::hash(data);

    println!("Data: {:?}", String::from_utf8_lossy(data));
    println!("Hash: {}", hex::encode(hash));
}
