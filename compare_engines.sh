#!/bin/bash

echo "=== Rayon Engine ==="
./target/release/ccms "claude" --engine rayon -n 5

echo -e "\n=== Tokio Engine ==="
./target/release/ccms "claude" --engine tokio -n 5

echo -e "\n=== Smol Engine ==="
./target/release/ccms "claude" --engine smol -n 5