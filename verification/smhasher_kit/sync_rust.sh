#!/usr/bin/env bash
# =============================================================================
# sync_rust.sh — Rebuild libtachyon from Rust and relink SMHasher
# Run from: anywhere (uses absolute paths relative to script location)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TACHYON_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SMHASHER_BUILD="$TACHYON_ROOT/verification/smhasher/build"

if [ ! -d "$SMHASHER_BUILD" ]; then
    echo "❌ smhasher/build/ not found. Run setup_testing.sh first (option 2)."
    exit 1
fi

echo " Building Tachyon Rust library..."
cd "$TACHYON_ROOT"
RUSTFLAGS="-C target-cpu=native" cargo build --release --lib

echo " Copying libtachyon.a to smhasher_kit/..."
cp "$TACHYON_ROOT/target/release/libtachyon.a" "$SCRIPT_DIR/libtachyon.a"

echo " Relinking SMHasher..."
cd "$SMHASHER_BUILD"
make -j$(nproc)

echo "✅ Done. Run: $SMHASHER_BUILD/SMHasher Tachyon"
