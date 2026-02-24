#!/bin/bash

# Tachyon PractRand Test Helper
# This script compiles the stream generator and pipes it into PractRand.

# 1. Build the stream generator via Cargo
echo " Building Tachyon stream generator via Cargo..."
cd ../..
cargo build --release --bin tachyon_stream

if [ $? -ne 0 ]; then
    echo "❌ Build failed."
    exit 1
fi

GENERATOR="../../target/release/tachyon_stream"
cd verification/practrand_kit

# 2. Check if PractRand is installed
if ! command -v RNG_test &> /dev/null
then
    echo "⚠️  RNG_test (PractRand) not found."
    echo "Please download it or use the new setup script:"
    echo "  cd .. && bash setup_testing.sh"
    echo ""
    echo "Choose option [3] there to setup PractRand only."
    exit 1
fi

# 3. Choose Test Mode
echo ""
echo "Select Hardware Path to test:"
echo "1) AES-NI Path (16 Bytes Input)"
echo "2) AVX-512 Path (64 Bytes Input) - Default"
echo "3) Cyclic Mode (16, 32, 64, 128 Bytes mixed)"
echo "4) Custom Size"
read -p "Selection [1-4, Default=2]: " choice

MODE="64"
case $choice in
    1) MODE="16" ;;
    2) MODE="64" ;;
    3) MODE="cyclic" ;;
    4) read -p "Enter Size in Bytes: " MODE ;;
    *) MODE="64" ;;
esac

# 4. Start the test
LOGFILE="practrand_results.txt"
echo "--- Test Start: $(date) | Mode: $MODE ---" >> $LOGFILE
echo " Starting PractRand test (pipe) in mode: $MODE..."
echo " Results are being saved to: $LOGFILE"
echo "Press Ctrl+C to stop."
$GENERATOR $MODE | RNG_test stdin64 -multithreaded 2>&1 | tee -a $LOGFILE
