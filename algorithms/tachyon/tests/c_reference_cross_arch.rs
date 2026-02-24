//! C-Reference Cross-Architecture Verification
//!
//! Verifies that the Rust implementation exactly matches the C backends
//! (AVX-512, AES-NI, Portable) across boundary spaces.

#![allow(clippy::expect_used)]
use std::io::Write;
use std::process::Command;

// =============================================================================
// CONSTANTS
// =============================================================================

const MAX_EXHAUSTIVE_LEN: usize = 4096;
const LARGE_TEST_SIZES: [usize; 5] = [
    1024 * 1024,             // 1MB
    1024 * 1024 + 1,         // 1MB + 1
    2 * 1024 * 1024,         // 2MB
    5 * 1024 * 1024 + 12345, // Arbitrary Large
    10 * 1024 * 1024,        // 10MB
];

/// Sizes that exercise streaming-specific boundaries:
/// short-path edge, first chunk boundary (256 KiB), and one multi-chunk input.
const STREAMING_BOUNDARY_SIZES: [usize; 6] = [
    0,
    1,
    63,
    64,
    256 * 1024,     // exactly one chunk
    256 * 1024 + 1, // one chunk + 1 byte (forces tree path)
];

struct Backends {
    avx: String,
    aes: String,
    port: String,
}

// =============================================================================
// VERIFICATION RUNNER
// =============================================================================

#[test]
fn test_exhaustive_cross_arch() {
    println!("Compiling C Reference (Auto-Detect / AVX-512)...");
    let bin_avx512 = compile_c_binary("tachyon_dispatcher_auto", &[]);

    println!("Compiling C Reference (Forced AES-NI)...");
    let bin_aesni = compile_c_binary("tachyon_dispatcher_aesni", &["-DFORCE_AESNI"]);

    println!("Compiling C Reference (Forced Portable)...");
    let bin_portable = compile_c_binary("tachyon_dispatcher_portable", &["-DFORCE_PORTABLE"]);

    let backends = Backends {
        avx: bin_avx512,
        aes: bin_aesni,
        port: bin_portable,
    };

    println!("Starting Exhaustive One-Shot Verification (0..{MAX_EXHAUSTIVE_LEN} bytes)...");

    for size in 0..=MAX_EXHAUSTIVE_LEN {
        if size % 100 == 0 {
            print!("\rTesting size: {size}/{MAX_EXHAUSTIVE_LEN}");
            let _ = std::io::stdout().flush();
        }
        verify_consistency(size, &backends);
    }
    println!("\nExhaustive one-shot loop passed.");

    println!("Starting Large Input One-Shot Verification...");
    for &size in &LARGE_TEST_SIZES {
        print!("Testing one-shot size: {size} bytes... ");
        let _ = std::io::stdout().flush();
        verify_consistency(size, &backends);
        println!("OK");
    }

    println!("Starting Streaming API Verification (boundary sizes)...");
    for &size in &STREAMING_BOUNDARY_SIZES {
        print!("Testing streaming size: {size} bytes... ");
        let _ = std::io::stdout().flush();
        verify_streaming_consistency(size, &backends);
        println!("OK");
    }

    println!("Starting Streaming API Verification (large inputs)...");
    for &size in &LARGE_TEST_SIZES {
        print!("Testing streaming size: {size} bytes... ");
        let _ = std::io::stdout().flush();
        verify_streaming_consistency(size, &backends);
        println!("OK");
    }

    let _ = std::fs::remove_file(&backends.avx);
    let _ = std::fs::remove_file(&backends.aes);
    let _ = std::fs::remove_file(&backends.port);
}

// =============================================================================
// C COMPILATION HELPERS
// =============================================================================

fn compile_c_binary(bin_name: &str, extra_flags: &[&str]) -> String {
    let avx_flags = [
        "-mavx512f",
        "-mavx512bw",
        "-mvaes",
        "-mvpclmulqdq",
        "-maes",
        "-msse4.1",
        "-mpclmul",
    ];
    let aes_flags = ["-maes", "-msse4.1", "-mpclmul"];

    let base_args = ["-O3", "-Wall", "-Wextra", "-Wno-unused-function"];

    let compile_obj = |src: &str, obj: &str, simd: &[&str]| {
        let mut cmd = Command::new("gcc");
        cmd.args(base_args).arg("-c").arg(src).arg("-o").arg(obj);
        cmd.args(simd);
        cmd.args(extra_flags);
        let status = cmd
            .status()
            .unwrap_or_else(|_| panic!("Failed to run gcc for {src}"));
        assert!(status.success(), "Failed to compile {src}");
    };

    let obj_avx = format!("{bin_name}_avx512.o");
    let obj_aes = format!("{bin_name}_aesni.o");
    let obj_port = format!("{bin_name}_port.o");
    let obj_disp = format!("{bin_name}_disp.o");
    let obj_wrap = format!("{bin_name}_wrap.o");

    compile_obj("c-reference/tachyon_avx512.c", &obj_avx, &avx_flags);
    compile_obj("c-reference/tachyon_aesni.c", &obj_aes, &aes_flags);
    compile_obj("c-reference/tachyon_portable.c", &obj_port, &[]);
    compile_obj("c-reference/tachyon_dispatcher.c", &obj_disp, &[]);
    compile_obj("tests/c/test_wrapper.c", &obj_wrap, &[]);

    let mut cmd = Command::new("gcc");
    cmd.arg("-o").arg(bin_name);
    cmd.args([&obj_avx, &obj_aes, &obj_port, &obj_disp, &obj_wrap]);
    let status = cmd
        .status()
        .unwrap_or_else(|_| panic!("Failed to link {bin_name}"));
    assert!(status.success(), "Failed to link {bin_name}");

    let _ = std::fs::remove_file(&obj_avx);
    let _ = std::fs::remove_file(&obj_aes);
    let _ = std::fs::remove_file(&obj_port);
    let _ = std::fs::remove_file(&obj_disp);
    let _ = std::fs::remove_file(&obj_wrap);

    format!("./{bin_name}")
}

// =============================================================================
// CONSISTENCY CHECKS
// =============================================================================

fn verify_consistency(size: usize, backends: &Backends) {
    let mut data = vec![0u8; size];
    #[allow(clippy::cast_possible_truncation)]
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = (i.wrapping_mul(37) ^ (i >> 8)) as u8;
    }

    // Standard
    let rust_std = tachyon::hash(&data);
    let rust_std_hex = hex::encode(rust_std);
    check_all_backends(backends, &data, &[0], &rust_std_hex, "Standard", size);

    // Seeded
    let seed = 0xDEAD_BEEF;
    let rust_seeded = tachyon::hash_seeded(&data, seed);
    let rust_seeded_hex = hex::encode(rust_seeded);
    let mut params_seeded = vec![1u8];
    params_seeded.extend_from_slice(&seed.to_le_bytes());
    check_all_backends(
        backends,
        &data,
        &params_seeded,
        &rust_seeded_hex,
        "Seeded",
        size,
    );

    // Keyed
    let mut key = [0u8; 32];
    #[allow(clippy::cast_possible_truncation)]
    for (i, k) in key.iter_mut().enumerate() {
        *k = (i + 1) as u8;
    }
    let rust_keyed = tachyon::hash_keyed(&data, &key);
    let rust_keyed_hex = hex::encode(rust_keyed);
    let mut params_keyed = vec![2u8];
    params_keyed.extend_from_slice(&key);
    check_all_backends(
        backends,
        &data,
        &params_keyed,
        &rust_keyed_hex,
        "Keyed",
        size,
    );

    // Domain
    let domain: u64 = 1;
    let rust_domain = tachyon::hash_with_domain(&data, tachyon::TachyonDomain::FileChecksum);
    let rust_domain_hex = hex::encode(rust_domain);
    let mut params_domain = vec![3u8];
    params_domain.extend_from_slice(&domain.to_le_bytes());
    check_all_backends(
        backends,
        &data,
        &params_domain,
        &rust_domain_hex,
        "Domain",
        size,
    );
}

// =============================================================================
// STREAMING CONSISTENCY CHECKS
// =============================================================================
// Mode bytes: 0x10 (std), 0x11 (seeded), 0x12 (keyed), 0x13 (domain)

fn verify_streaming_consistency(size: usize, backends: &Backends) {
    let mut data = vec![0u8; size];
    #[allow(clippy::cast_possible_truncation)]
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = (i.wrapping_mul(37) ^ (i >> 8)) as u8;
    }

    // Standard streaming
    let rust_std = {
        let mut h = tachyon::Hasher::new().expect("Hasher instantiation failed");
        h.update(&data);
        h.finalize()
    };
    let rust_std_hex = hex::encode(rust_std);
    check_all_backends(
        backends,
        &data,
        &[0x10],
        &rust_std_hex,
        "Streaming-Standard",
        size,
    );

    // Seeded streaming
    let seed: u64 = 0xDEAD_BEEF;
    let rust_seeded = {
        let mut h = tachyon::Hasher::new_full(0, seed).expect("Hasher instantiation failed");
        h.update(&data);
        h.finalize()
    };
    let rust_seeded_hex = hex::encode(rust_seeded);
    let mut params_seeded = vec![0x11u8];
    params_seeded.extend_from_slice(&seed.to_le_bytes());
    check_all_backends(
        backends,
        &data,
        &params_seeded,
        &rust_seeded_hex,
        "Streaming-Seeded",
        size,
    );

    // Keyed streaming
    let mut key = [0u8; 32];
    #[allow(clippy::cast_possible_truncation)]
    for (i, k) in key.iter_mut().enumerate() {
        *k = (i + 1) as u8;
    }
    let rust_keyed = {
        let mut h = tachyon::Hasher::new().expect("Hasher instantiation failed");
        h.set_key(&key);
        h.update(&data);
        h.finalize()
    };
    let rust_keyed_hex = hex::encode(rust_keyed);
    let mut params_keyed = vec![0x12u8];
    params_keyed.extend_from_slice(&key);
    check_all_backends(
        backends,
        &data,
        &params_keyed,
        &rust_keyed_hex,
        "Streaming-Keyed",
        size,
    );

    // Domain streaming
    let domain: u64 = tachyon::TachyonDomain::FileChecksum.to_u64();
    let rust_domain = {
        let mut h = tachyon::Hasher::new_with_domain(domain).expect("Hasher instantiation failed");
        h.update(&data);
        h.finalize()
    };
    let rust_domain_hex = hex::encode(rust_domain);
    let mut params_domain = vec![0x13u8];
    params_domain.extend_from_slice(&domain.to_le_bytes());
    check_all_backends(
        backends,
        &data,
        &params_domain,
        &rust_domain_hex,
        "Streaming-Domain",
        size,
    );
}

// =============================================================================
// SUBPROCESS EXECUTION
// =============================================================================

fn check_all_backends(
    backends: &Backends,
    data: &[u8],
    params: &[u8],
    expected_hex: &str,
    mode_name: &str,
    size: usize,
) {
    let c_avx = run_c_hash(&backends.avx, params, data);
    assert_eq!(
        expected_hex, c_avx,
        "Mismatch Rust vs C-AVX512 [{mode_name}] size={size}"
    );

    let c_aes = run_c_hash(&backends.aes, params, data);
    assert_eq!(
        expected_hex, c_aes,
        "Mismatch Rust vs C-AESNI [{mode_name}] size={size}"
    );

    let c_port = run_c_hash(&backends.port, params, data);
    assert_eq!(
        expected_hex, c_port,
        "Mismatch Rust vs C-PORTABLE [{mode_name}] size={size}"
    );
}

fn run_c_hash(bin_path: &str, params: &[u8], data: &[u8]) -> String {
    let mut child = Command::new(bin_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| panic!("Failed to start C binary: {bin_path}"));

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(params);
        let _ = stdin.write_all(data);
    }

    let output = child
        .wait_with_output()
        .unwrap_or_else(|_| panic!("Failed to read stdout from {bin_path}"));
    assert!(output.status.success(), "Binary {bin_path} crashed");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
