//! Language Binding Tests
//!
//! Integration tests for C, Python, Node.js, Go, and Java bindings.
//! Uses centralized test vectors for consistency across all languages.

#![allow(clippy::pedantic, clippy::nursery)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Once;

// =============================================================================
// TEST VECTORS
// =============================================================================

/// Canonical test input used across all binding tests.
pub const TEST_INPUT: &str = "Tachyon";

/// Expected hash of TEST_INPUT (hex-encoded).
/// NOTE: This constant is currently UNUSED. Binding tests use test_vectors.json instead.
#[allow(dead_code)]
pub const EXPECTED_HASH: &str = "62c63f5760576319db992db546bfee49634b48bfde41652aff9eb10097870d12";

// =============================================================================
// SHARED INFRASTRUCTURE
// =============================================================================

/// Ensures `cargo build --release` runs only once across all tests.
static BUILD_ONCE: Once = Once::new();

fn ensure_release_build() {
    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .status()
            .expect("Failed to run cargo build");
        assert!(status.success(), "Cargo build failed");
    });
}

/// Result of running a binding test.
struct TestResult {
    success: bool,
    stdout: String,
    stderr: String,
}

impl TestResult {
    fn from_output(output: Output) -> Self {
        Self {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    fn assert_success(&self, test_name: &str) {
        if !self.success {
            eprintln!("\n{}", "=".repeat(60));
            eprintln!("❌ {} FAILED", test_name);
            eprintln!("{}", "=".repeat(60));
            eprintln!("STDOUT:\n{}", self.stdout);
            eprintln!("STDERR:\n{}", self.stderr);
            eprintln!("{}\n", "=".repeat(60));
            panic!("{} test failed", test_name);
        }
    }
}

/// Checks if a tool is available in PATH.
fn tool_available(cmd: &str, arg: &str) -> bool {
    Command::new(cmd).arg(arg).output().is_ok()
}

/// Prints a skip warning visible even when cargo captures output.
fn warn_skip(tool: &str) {
    let msg = format!(
        "\x1b[33m\n[SKIP] {} not found. Skipping integration test.\x1b[0m\n",
        tool
    );
    if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(msg.as_bytes());
    } else {
        println!("{msg}");
    }
}

/// RAII struct to clean up files when they go out of scope.
struct FileCleanup(&'static str);

impl Drop for FileCleanup {
    fn drop(&mut self) {
        if Path::new(self.0).exists() {
            let _ = std::fs::remove_file(self.0);
        }
    }
}

// =============================================================================
// BINDING TESTS
// =============================================================================

#[test]
fn test_c_binding() {
    if !tool_available("gcc", "--version") {
        warn_skip("GCC");
        return;
    }

    // Ensure cleanup happens even if test panics
    let _cleanup = FileCleanup("tests/c/test_c");

    ensure_release_build();

    // Compile
    let compile = Command::new("gcc")
        .args([
            "-o",
            "tests/c/test_c",
            "tests/c/main.c",
            "-L",
            "../../target/release",
            "-l:libtachyon.so",
            "-Wl,-rpath,$ORIGIN/../../../../target/release",
            "-I",
            ".",
            "-I",
            "../../bindings/c",
        ])
        .output()
        .expect("Failed to compile C test");

    TestResult::from_output(compile).assert_success("C compilation");

    // Run
    let run = Command::new("./tests/c/test_c")
        .env("LD_LIBRARY_PATH", "../../target/release")
        .output()
        .expect("Failed to run C test");

    let result = TestResult::from_output(run);
    println!("{}", result.stdout);
    result.assert_success("C binding");
}

#[test]
fn test_python_binding() {
    if !tool_available("python3", "--version") {
        warn_skip("Python3");
        return;
    }

    ensure_release_build();

    let run = Command::new("python3")
        .arg("tests/python/test.py")
        .output()
        .expect("Failed to run Python test");

    let result = TestResult::from_output(run);
    println!("{}", result.stdout);
    result.assert_success("Python binding");
}

#[test]
fn test_go_binding() {
    if !tool_available("go", "version") {
        warn_skip("Go");
        return;
    }

    ensure_release_build();

    let run = Command::new("go")
        .args(["run", "."])
        .current_dir("tests/go")
        .current_dir("tests/go")
        .env("CGO_LDFLAGS", "-L../../../../target/release -ltachyon")
        .env("CGO_CFLAGS", "-I../../../../")
        .env("LD_LIBRARY_PATH", "../../../../target/release")
        .output()
        .expect("Failed to run Go test");

    let result = TestResult::from_output(run);
    println!("{}", result.stdout);
    result.assert_success("Go binding");
}

#[test]
fn test_node_binding() {
    if !tool_available("node", "--version") {
        warn_skip("Node.js");
        return;
    }

    // Build node addon
    let build = Command::new("cargo")
        .args(["build", "--release", "-p", "tachyon-node"])
        .output()
        .expect("Failed to build tachyon-node");

    TestResult::from_output(build).assert_success("Node addon build");

    // Copy .so to .node
    let lib_name = if cfg!(target_os = "macos") {
        "libtachyon_node.dylib"
    } else {
        "libtachyon_node.so"
    };

    let src = Path::new("../../target/release").join(lib_name);
    let dst = Path::new("../../bindings/node/tachyon.node");

    std::fs::copy(&src, dst).unwrap_or_else(|e| panic!("Failed to copy {src:?} to {dst:?}: {e}"));

    // Run test
    let run = Command::new("node")
        .arg("tests/node/test.js")
        .output()
        .expect("Failed to run Node test");

    let result = TestResult::from_output(run);
    println!("{}", result.stdout);
    result.assert_success("Node.js binding");
}

#[test]
fn test_java_binding() {
    if !tool_available("javac", "--version") {
        warn_skip("Java SDK (javac)");
        return;
    }

    // Build Java JNI library
    let build = Command::new("cargo")
        .args(["build", "--release", "-p", "tachyon-java"])
        .output()
        .expect("Failed to build tachyon-java");

    TestResult::from_output(build).assert_success("Java JNI build");

    // Copy JNI lib to target/release (so Tachyon.java finds it via user.dir)
    let _ = std::fs::create_dir_all("target/release");
    let lib_name = if cfg!(target_os = "macos") {
        "libtachyon_java.dylib"
    } else {
        "libtachyon_java.so"
    };
    std::fs::copy(
        Path::new("../../target/release").join(lib_name),
        Path::new("target/release").join(lib_name),
    )
    .unwrap_or_else(|e| panic!("Failed to copy java lib: {e}"));

    // Create classes directory
    let _ = std::fs::create_dir_all("target/java_classes");

    // Compile Java
    let compile = Command::new("javac")
        .args([
            "-d",
            "target/java_classes",
            "../../bindings/java/com/tachyon/Tachyon.java",
            "tests/java/Test.java",
        ])
        .output()
        .expect("Failed to run javac");

    TestResult::from_output(compile).assert_success("Java compilation");

    // Run Java
    let run = Command::new("java")
        .args(["-cp", "target/java_classes", "Test"])
        .output()
        .expect("Failed to run java");

    let result = TestResult::from_output(run);
    println!("{}", result.stdout);
    result.assert_success("Java binding");
}

// =============================================================================
// TEST VECTOR VERIFICATION
// =============================================================================

/// Verifies that the test vectors are correct by computing them with Rust.
#[test]
fn verify_test_vectors() {
    use std::fs::File;
    use std::io::BufReader;

    #[derive(serde::Deserialize)]
    struct TestVectors {
        vectors: Vec<Vector>,
    }

    #[derive(serde::Deserialize)]
    struct Vector {
        name: String,
        input: String,
        hash: String,
    }

    let file = File::open("tests/test_vectors.json").expect("Failed to open test vectors");
    let reader = BufReader::new(file);
    let data: TestVectors = serde_json::from_reader(reader).expect("Failed to parse JSON");

    for vec in data.vectors {
        let input_bytes = match vec.input.as_str() {
            "HUGE_1MB" => vec![0x41u8; 1024 * 1024],
            "LARGE_1KB" => vec![0x41u8; 1024],
            "MEDIUM_256_A" => vec![0x41u8; 256],
            "EXACT_64_ZERO" => vec![0x00u8; 64],
            "EXACT_512_ONE" => vec![0x01u8; 512],
            "UNALIGNED_63_TWO" => vec![0x02u8; 63],
            other => other.as_bytes().to_vec(),
        };
        let hash = tachyon::hash(&input_bytes);
        let hex = hex::encode(hash);

        if hex != vec.hash {
            eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
            eprintln!("║  Hash Mismatch for vector '{}'", vec.name);
            eprintln!("╠════════════════════════════════════════════════════════════════╣");
            eprintln!("║  Input: {:?} ({} bytes)", vec.input, input_bytes.len());
            eprintln!("║");
            eprintln!("║  OLD (expected in test_vectors.json):");
            eprintln!("║    {}", vec.hash);
            eprintln!("║");
            eprintln!("║  NEW (current hash output):");
            eprintln!("║    {}", hex);
            eprintln!("║");
            eprintln!("║   To update test_vectors.json, run:");
            eprintln!("║     cargo run --example generate_test_vectors > tests/test_vectors.json");
            eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
            panic!("Hash mismatch detected - see details above");
        }
    }
}
