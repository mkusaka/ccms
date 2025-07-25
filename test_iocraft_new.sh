#!/bin/bash

# Test script for iocraft interactive mode

echo "Testing iocraft interactive mode..."
echo "This will test:"
echo "1. Application starts without crashing"
echo "2. Query input works (typing 'a' should not crash)"
echo "3. Ctrl+C exits properly (with double-press confirmation)"
echo ""
echo "Starting in 3 seconds..."
sleep 3

# Build if not already built
if [ ! -f "./target/release/ccms" ]; then
    echo "Building release version..."
    cargo build --release
fi

echo ""
echo "Launching iocraft interactive mode..."
echo "Try:"
echo "  - Type 'a' or other characters to test query input"
echo "  - Press Ctrl+C once to see confirmation message"
echo "  - Press Ctrl+C again within 1 second to exit"
echo "  - Press Tab to cycle through role filters"
echo "  - Press 't' to toggle truncation"
echo "  - Press '?' for help"
echo ""

./target/release/ccms -i