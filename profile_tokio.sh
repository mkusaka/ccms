#!/bin/bash

echo "Profiling Tokio implementation..."

# Build with profiling features
cargo build --release --features "async profiling"

# Create a controlled test file for consistent profiling
TEST_FILE="/tmp/profile_test.jsonl"
echo "Creating test file with 10000 lines..."
> "$TEST_FILE"
for i in {1..10000}; do
    echo '{"type":"user","message":{"role":"user","content":"Test message number '$i' with keywords like error, warning, performance, debug, and tokio async implementation details"},"uuid":"test-'$i'","timestamp":"2024-01-01T00:00:'$(printf "%02d" $((i % 60)))'Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}' >> "$TEST_FILE"
done

echo ""
echo "Running tokio profiler..."
./target/release/perf_test tokio "$TEST_FILE" "error OR warning" --profile

echo ""
echo "Profile files created:"
ls -la profile_tokio.*

echo ""
echo "To view flamegraph: open profile_tokio.svg"