#!/bin/bash
set -e

# Configuration
WRAPPER_SRC="testu01_stdin.c"
WRAPPER_BIN="testu01_stdin"
STREAM_BIN="../../target/release/tachyon_stream"

# 1. Check Dependencies
if ! ldconfig -p | grep -q libtestu01; then
    echo " Warning: libtestu01 not found in standard library path."
    echo " Ensure you have TestU01 installed (e.g. from source or package manager)."
    echo " Trying to compile anyway..."
fi

# 2. Build Rust Stream Generator
echo " Building Tachyon stream generator..."
(cd ../.. && cargo build --release --bin tachyon_stream --quiet)

if [ ! -f "$STREAM_BIN" ]; then
    echo "❌ Error: Stream generator binary not found ($STREAM_BIN)"
    exit 1
fi

# 3. Compile C Wrapper
echo " Compiling TestU01 wrapper..."
gcc -O3 -march=native -I/usr/local/include -L/usr/local/lib "$WRAPPER_SRC" -o "$WRAPPER_BIN" -ltestu01 -lprobdist -lmylib -lm

if [ $? -ne 0 ]; then
    echo "❌ Error: Compilation failed. Make sure TestU01 is installed."
    exit 1
fi

# 4. Mode Selection
echo ""
echo "Select Hardware / Generator Mode:"
echo "1) AES-NI Path (16 Bytes Input)"
echo "2) AVX-512 Path (64 Bytes Input) - Default"
echo "3) Cyclic Mode (16, 32, 64, 128 Bytes mixed)"
read -p "Selection [1-3, Default=2]: " choice

MODE="64"
case $choice in
    1) MODE="16" ;;
    2) MODE="64" ;;
    3) MODE="cyclic" ;;
    *) MODE="64" ;;
esac

# 5. Run BigCrush
echo ""
echo " Starting TestU01 BigCrush... (This will take a LONG time!)"
echo "   Mode: $MODE"
LOGFILE="bigcrush_results_$MODE.txt"
echo "--- Test Start: $(date) | Mode: $MODE ---" >> $LOGFILE
echo "   Output is being saved to: $LOGFILE"
echo ""

$STREAM_BIN "$MODE" | ./$WRAPPER_BIN | tee -a $LOGFILE
