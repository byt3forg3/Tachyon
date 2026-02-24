#!/usr/bin/env bash
# =============================================================================
# sync_c.sh — Sync C-Reference sources into SMHasher and rebuild
# Run from: anywhere (uses absolute paths relative to script location)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TACHYON_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CREF="$TACHYON_ROOT/algorithms/tachyon/c-reference"
SMHASHER_TACHYON="$TACHYON_ROOT/verification/smhasher/tachyon"
SMHASHER_BUILD="$TACHYON_ROOT/verification/smhasher/build"

if [ ! -d "$SMHASHER_TACHYON" ]; then
    echo "❌ smhasher/tachyon/ not found. Run setup_testing.sh first (option 1)."
    exit 1
fi

echo " Syncing C-Reference sources into smhasher/tachyon/..."
for f in tachyon.h tachyon_impl.h tachyon_dispatcher.c \
          tachyon_portable.c tachyon_aesni.c tachyon_avx512.c README.md; do
    cp "$CREF/$f" "$SMHASHER_TACHYON/$f"
    echo "   copied: $f"
done

echo " Rebuilding SMHasher..."
cd "$SMHASHER_BUILD"
make -j$(nproc)

echo "✅ Done. Run: $TACHYON_ROOT/verification/smhasher/build/SMHasher Tachyon"
