#!/bin/bash

echo "Testing standard terminal-based interactive mode:"
cargo run --release -- -i "test"

echo ""
echo "Testing GPU-accelerated interactive mode:"
cargo run --release --features wgpu -- -i --wgpu "test"