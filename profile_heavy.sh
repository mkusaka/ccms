#!/bin/bash

echo "Heavy profiling for Tokio vs Rayon..."

# Build with profiling features
cargo build --release --features "async profiling"

# Create multiple large test files
TEST_DIR="/tmp/heavy_profile_test"
mkdir -p "$TEST_DIR"

echo "Creating 100 test files with 5000 lines each..."
for f in {1..100}; do
    > "$TEST_DIR/test_$f.jsonl"
    for i in {1..5000}; do
        echo '{"type":"user","message":{"role":"user","content":"Performance test message '$i' in file '$f' with complex regex patterns and various keywords like error, warning, debug, async, tokio, performance optimization and benchmarking"},"uuid":"'$f'-'$i'","timestamp":"2024-01-01T00:00:'$(printf "%02d" $((i % 60)))'Z","sessionId":"session-'$f'","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}' >> "$TEST_DIR/test_$f.jsonl"
    done
done

echo ""
echo "Profiling Rayon..."
time ./target/release/perf_test rayon "$TEST_DIR/*.jsonl" "error OR warning OR performance" --profile

echo ""
echo "Profiling Tokio..."
time ./target/release/perf_test tokio "$TEST_DIR/*.jsonl" "error OR warning OR performance" --profile

echo ""
echo "Profile files created:"
ls -la profile_*.*

echo ""
echo "Cleanup: rm -rf $TEST_DIR"