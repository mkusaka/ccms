#!/bin/bash

echo "Building the application..."
cargo build --release

echo ""
echo "Running interactive mode with sample search..."
echo "Use your mouse to click on search results!"
echo ""

# Run with a simple search that should return results
./target/release/ccms -i "user OR assistant"