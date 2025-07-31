#!/bin/bash

# Simple performance test script

echo "=== Simple CCMS Performance Test ==="
echo ""

# Create a simple test file
TEST_FILE="/tmp/test_ccms.jsonl"

echo "Creating test file with 1000 lines..."
cat > "$TEST_FILE" << 'EOF'
EOF

for i in {1..1000}; do
    echo '{"type":"user","message":{"role":"user","content":"Test message number '$i' with some keywords like error, warning, performance, and debug information"},"uuid":"test-'$i'","timestamp":"2024-01-01T00:00:'$(printf "%02d" $((i % 60)))'Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}' >> "$TEST_FILE"
done

echo "Building with async features..."
cargo build --release --features async

echo ""
echo "=== Running Tests ==="
echo ""

echo "Test 1: Simple search for 'message'"
echo "--------------------------------"
echo "Rayon:"
time cargo run --release --features async --bin perf_test -- rayon "$TEST_FILE" "message"

echo ""
echo "Tokio:"
time cargo run --release --features async --bin perf_test -- tokio "$TEST_FILE" "message"

echo ""
echo "Test 2: Complex query 'error OR warning'"
echo "---------------------------------------"
echo "Rayon:"
time cargo run --release --features async --bin perf_test -- rayon "$TEST_FILE" "error OR warning"

echo ""
echo "Tokio:"
time cargo run --release --features async --bin perf_test -- tokio "$TEST_FILE" "error OR warning"

echo ""
echo "Test 3: AND query 'test AND performance'"
echo "---------------------------------------"
echo "Rayon:"
time cargo run --release --features async --bin perf_test -- rayon "$TEST_FILE" "test AND performance"

echo ""
echo "Tokio:"
time cargo run --release --features async --bin perf_test -- tokio "$TEST_FILE" "test AND performance"

echo ""
echo "Cleanup: rm $TEST_FILE"