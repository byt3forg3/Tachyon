#!/bin/bash
set -e

# Tracking variables for final instructions
RAN_SMHASHER=0
RAN_PRACTRAND=0
RAN_TESTU01=0

# Configuration
SMHASHER_REPO="https://github.com/rurban/smhasher.git"
PRACTRAND_REPO="https://github.com/rurban/PractRand.git"
SMHASHER_DIR="smhasher"
PRACTRAND_DIR="PractRand"
TESTU01_DIR="TestU01-2009"
# Absolute path to project root (computed once, stays valid after any cd)
TACHYON_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=================================================="
echo "   Tachyon Unified Testing Suite Setup           "
echo "=================================================="

# =============================================================================
# SMHASHER — VARIANT A: Pure C Reference (for SMHasher upstream PR)
# =============================================================================
# Copies sources from algorithms/tachyon/c-reference/ into smhasher/tachyon/

function setup_smhasher_c() {
    echo ""
    echo "--- [ Setting up SMHasher (Pure C / PR-ready) ] ---"

    # Dependency check for C variant
    for cmd in cmake git g++ make; do
        if ! command -v $cmd &> /dev/null; then
            echo "❌ Error: $cmd is not installed."
            exit 1
        fi
    done

    if [ ! -d "$SMHASHER_DIR" ]; then
        echo " Cloning SMHasher (rurban fork)..."
        git clone --depth 1 "$SMHASHER_REPO" "$SMHASHER_DIR"
    fi

    cd "$SMHASHER_DIR"
    git submodule update --init --recursive

    # -------------------------------------------------------------------------
    # Copy C-Reference sources into smhasher/tachyon/
    # -------------------------------------------------------------------------
    CREF="$(cd "$TACHYON_ROOT/algorithms/tachyon/c-reference" && pwd)"
    TACHYON_DEST="tachyon"

    echo " Copying Tachyon C sources into ${TACHYON_DEST}/..."
    mkdir -p "$TACHYON_DEST"
    for f in tachyon.h tachyon_impl.h tachyon_dispatcher.c \
              tachyon_portable.c tachyon_aesni.c tachyon_avx512.c README.md; do
        cp "$CREF/$f" "$TACHYON_DEST/$f"
    done
    echo "   ✅ Sources copied to smhasher/${TACHYON_DEST}/"

    # Glue wrapper at root level (like other hash .cpp files in SMHasher)
    cp "$TACHYON_ROOT/verification/smhasher_kit/Tachyon_glue.cpp" "./Tachyon.cpp"

    # -------------------------------------------------------------------------
    # Patch CMakeLists.txt
    # -------------------------------------------------------------------------
    if ! grep -q "TACHYON_SRC" CMakeLists.txt; then
        echo " - Patching CMakeLists.txt..."

        CMAKE_INJECTION='
# --- Tachyon Integration Start ---
# Pure C implementation — no Rust/cargo required.
# Runtime dispatcher selects AVX-512, AES-NI, or portable via CPUID.
set(TACHYON_SRC
    tachyon/tachyon_dispatcher.c
    tachyon/tachyon_portable.c
    tachyon/tachyon_aesni.c
    tachyon/tachyon_avx512.c
)
set_source_files_properties(tachyon/tachyon_avx512.c
    PROPERTIES COMPILE_FLAGS "-mavx512f -mavx512bw -mvaes -mvpclmulqdq -maes -msse4.1 -mpclmul")
set_source_files_properties(tachyon/tachyon_aesni.c
    PROPERTIES COMPILE_FLAGS "-maes -msse4.1 -mpclmul")
# --- Tachyon Integration End ---
'
        awk -v blk="$CMAKE_INJECTION" \
            '/^set\(BLAKE3_SRC/ { print blk } { print }' \
            CMakeLists.txt > CMakeLists.txt.tmp && mv CMakeLists.txt.tmp CMakeLists.txt

        # Add TACHYON_SRC into the SMHasherSupport library (only in the add_library block)
        sed -i '/add_library(SMHasherSupport/,/)/{s|${BLAKE3_SRC}|${BLAKE3_SRC}\n  ${TACHYON_SRC}|}' CMakeLists.txt

        # Add Tachyon.cpp to the SMHasher executable
        sed -i 's/add_executable(SMHasher main\.cpp/add_executable(SMHasher main.cpp Tachyon.cpp/' CMakeLists.txt

        # Make tachyon/ headers findable from Tachyon.cpp
        sed -i '/add_executable(SMHasher/a target_include_directories(SMHasher PRIVATE tachyon)' CMakeLists.txt
    else
        echo " - CMakeLists.txt already patched, skipping."
    fi

    # -------------------------------------------------------------------------
    # Patch main.cpp
    # -------------------------------------------------------------------------
    if ! grep -q "Tachyon_Hash" main.cpp; then
        echo " - Patching main.cpp..."
        sed -i '/#include "Platform.h"/a #include "tachyon/tachyon.h"\nvoid Tachyon_Hash(const void * key, int len, uint32_t seed, void * out);' main.cpp
        sed -i '/blake3c_test,/a\  { Tachyon_Hash, 256, 0xE9BBF229, "Tachyon", "Tachyon 256-bit C", GOOD, {} },' main.cpp
    else
        echo " - main.cpp already patched, skipping."
    fi

    # -------------------------------------------------------------------------
    # Build
    # -------------------------------------------------------------------------
    echo " Building SMHasher..."
    mkdir -p build && cd build
    cmake -DCMAKE_BUILD_TYPE=Release ..
    make -j$(nproc)
    cd ../..
    echo "✅ SMHasher set up successfully with Tachyon (Pure C)."
    RAN_SMHASHER=1
}

# =============================================================================
# SMHASHER — VARIANT B: Rust / libtachyon (full optimized Rust build)
# =============================================================================
# Builds libtachyon.a via cargo and links it as an external library.
# Uses the Tachyon Rust implementation with native CPU optimizations.

function setup_smhasher_rust() {
    echo ""
    echo "--- [ Setting up SMHasher (Rust / libtachyon) ] ---"

    # Dependency check for Rust variant
    for cmd in cmake git g++ cargo make; do
        if ! command -v $cmd &> /dev/null; then
            echo "❌ Error: $cmd is not installed."
            exit 1
        fi
    done

    if [ ! -d "$SMHASHER_DIR" ]; then
        echo " Cloning SMHasher (rurban fork)..."
        git clone --depth 1 "$SMHASHER_REPO" "$SMHASHER_DIR"
    fi

    cd "$SMHASHER_DIR"
    git submodule update --init --recursive

    # -------------------------------------------------------------------------
    # Build libtachyon.a from Rust
    # -------------------------------------------------------------------------
    TACHYON_LIB_DIR="$(cd ../smhasher_kit && pwd)"
    echo " Building Tachyon Rust library (RUSTFLAGS=-C target-cpu=native)..."
    (cd "$TACHYON_ROOT" && RUSTFLAGS="-C target-cpu=native" cargo build --release --lib)
    cp "$TACHYON_ROOT/target/release/libtachyon.a" "$TACHYON_LIB_DIR/libtachyon.a"
    echo "   ✅ libtachyon.a ready in smhasher_kit/"

    # Copy header and glue wrapper
    cp "$TACHYON_ROOT/bindings/c/tachyon.h" "./tachyon.h"
    cp "$TACHYON_ROOT/verification/smhasher_kit/Tachyon_glue.cpp" "./Tachyon.cpp"

    # -------------------------------------------------------------------------
    # Patch CMakeLists.txt
    # -------------------------------------------------------------------------
    if ! grep -q "TachyonRust" CMakeLists.txt; then
        echo " - Patching CMakeLists.txt..."
        TACHYON_LIB_ABS="$TACHYON_LIB_DIR/libtachyon.a"
        CMAKE_INJECTION="
# --- Tachyon Integration Start ---
add_library(TachyonRust STATIC IMPORTED GLOBAL)
set_target_properties(TachyonRust PROPERTIES IMPORTED_LOCATION \"${TACHYON_LIB_ABS}\")
# --- Tachyon Integration End ---
"
        awk -v blk="$CMAKE_INJECTION" \
            '/^add_executable\(SMHasher/ { print blk } { print }' \
            CMakeLists.txt > CMakeLists.txt.tmp && mv CMakeLists.txt.tmp CMakeLists.txt

        # Add Tachyon.cpp to the executable
        sed -i 's/add_executable(SMHasher main\.cpp/add_executable(SMHasher main.cpp Tachyon.cpp/' CMakeLists.txt

        # Link against the imported Rust library
        sed -i '/target_link_libraries(SMHasher/s/$/ TachyonRust/' CMakeLists.txt
    else
        echo " - CMakeLists.txt already patched, skipping."
    fi

    # -------------------------------------------------------------------------
    # Patch main.cpp
    # -------------------------------------------------------------------------
    if ! grep -q "Tachyon_Hash" main.cpp; then
        echo " - Patching main.cpp..."
        sed -i '/#include "Platform.h"/a #include "tachyon.h"\nvoid Tachyon_Hash(const void * key, int len, uint32_t seed, void * out);' main.cpp
        sed -i '/{ MurmurHash3_x86_32,/i \  { Tachyon_Hash, 256, 0xE9BBF229, "Tachyon", "Tachyon 256-bit Rust", GOOD, {} },' main.cpp
    else
        echo " - main.cpp already patched, skipping."
    fi

    # -------------------------------------------------------------------------
    # Build
    # -------------------------------------------------------------------------
    echo " Building SMHasher..."
    mkdir -p build && cd build
    cmake -DCMAKE_BUILD_TYPE=Release ..
    make -j$(nproc)
    cd ../..
    echo "✅ SMHasher set up successfully with Tachyon (Rust)."
    RAN_SMHASHER=1
}

# =============================================================================
# PRACTRAND
# =============================================================================

function setup_practrand() {
    echo ""
    echo "--- [ Setting up PractRand ] ---"

    for cmd in cmake git g++ make; do
        if ! command -v $cmd &> /dev/null; then
            echo "❌ Error: $cmd is not installed."
            exit 1
        fi
    done

    if [ ! -d "$PRACTRAND_DIR" ]; then
        echo " Cloning PractRand..."
        git clone --depth 1 "$PRACTRAND_REPO" "$PRACTRAND_DIR"
    fi

    cd "$PRACTRAND_DIR"
    echo " Building PractRand (RNG_test)..."
    g++ -O3 -Iinclude src/*.cpp src/RNGs/other/*.cpp src/RNGs/*.cpp tools/RNG_test.cpp -o RNG_test -lpthread

    echo " Installing RNG_test to /usr/local/bin (requires sudo)..."
    if sudo cp RNG_test /usr/local/bin/; then
        echo "✅ PractRand installed successfully."
    else
        echo "  Could not copy to /usr/local/bin. Binary at: $(pwd)/RNG_test"
    fi
    cd ..
    RAN_PRACTRAND=1
}

# =============================================================================
# TESTU01
# =============================================================================

function setup_testu01() {
    echo ""
    echo "--- [ Setting up TestU01 ] ---"
    TESTU01_REPO="https://github.com/umontreal-simul/TestU01-2009.git"

    if [ ! -d "$TESTU01_DIR" ]; then
        echo " Cloning TestU01..."
        git clone --depth 1 "$TESTU01_REPO" "$TESTU01_DIR"
    fi

    cd "$TESTU01_DIR"
    echo " Building TestU01 (this may take a while)..."
    chmod +x configure install-sh missing mkinstalldirs
    ./configure CFLAGS="-std=gnu89 -g -O2 -Wno-error"
    make -j$(nproc)

    echo " Installing TestU01 (requires sudo)..."
    if sudo make install; then
        sudo ldconfig
        echo "✅ TestU01 installed successfully."
    else
        echo "❌ Error installing TestU01."
        exit 1
    fi
    cd ..
    RAN_TESTU01=1
}

# =============================================================================
# MENU
# =============================================================================

echo ""
echo "What would you like to set up?"
echo ""
echo "  SMHasher:"
echo "    1) SMHasher — Pure C reference  (no Rust needed, PR-ready)"
echo "    2) SMHasher — Rust / libtachyon (native optimized, requires cargo)"
echo ""
echo "  Full suites:"
echo "    3) Everything C   (SMHasher C   + PractRand + TestU01)"
echo "    4) Everything Rust (SMHasher Rust + PractRand + TestU01)"
echo ""
echo "  Other:"
echo "    5) PractRand only"
echo "    6) TestU01 only"
echo "    q) Quit"
echo ""
read -p "Selection [1-6, q]: " choice

case $choice in
    1) setup_smhasher_c ;;
    2) setup_smhasher_rust ;;
    3) setup_smhasher_c;    setup_practrand; setup_testu01 ;;
    4) setup_smhasher_rust; setup_practrand; setup_testu01 ;;
    5) setup_practrand ;;
    6) setup_testu01 ;;
    q) exit 0 ;;
    *) echo "Invalid choice." ;;
esac

echo ""
echo "=================================================="
echo "   Setup Finished!                                "
echo "=================================================="
echo ""
echo "  NEXT STEPS (Run your tests):"
echo "  ------------------------------------------------"

if [ "$RAN_SMHASHER" -eq 1 ]; then
    echo "  ▶ To run SMHasher (Avalanche, Speed, Quality):"
    echo "    cd smhasher/build && ./SMHasher Tachyon"
    echo ""
fi

if [ "$RAN_PRACTRAND" -eq 1 ]; then
    echo "  ▶ To run PractRand (Statistical anomalies):"
    echo "    cd practrand_kit && ./test_practrand.sh"
    echo ""
fi

if [ "$RAN_TESTU01" -eq 1 ]; then
    echo "  ▶ To run TestU01 (BigCrush linear bias test):"
    echo "    cd testu01_kit && ./run_bigcrush.sh"
    echo ""
fi

echo "Happy testing!"
echo "=================================================="
