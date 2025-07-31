#!/bin/bash

# Performance comparison script for Rayon vs Tokio

echo "=== CCMS Performance Test: Rayon vs Tokio ==="
echo ""

# Create test data directory
TEST_DIR="/tmp/ccms_perf_test"
mkdir -p "$TEST_DIR"

echo "1. Creating test data..."

# Create small files (100 lines each, 50 files)
echo "   Creating 50 small files (100 lines each)..."
for i in {1..50}; do
    cat > "$TEST_DIR/small_$i.jsonl" << EOF
$(for j in {1..100}; do
    echo '{"type":"user","message":{"role":"user","content":"Performance test message '$j' in file '$i' with some additional text to make it realistic"},"uuid":"'$i'-'$j'","timestamp":"2024-01-01T00:00:'$(printf "%02d" $((j % 60)))'Z","sessionId":"perf-session-'$i'","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}'
done)
EOF
done

# Create medium files (1000 lines each, 10 files)
echo "   Creating 10 medium files (1000 lines each)..."
for i in {1..10}; do
    cat > "$TEST_DIR/medium_$i.jsonl" << EOF
$(for j in {1..1000}; do
    echo '{"type":"user","message":{"role":"user","content":"Medium file test message '$j' in file '$i' with longer content to simulate real usage patterns in production environments"},"uuid":"med-'$i'-'$j'","timestamp":"2024-01-01T00:00:'$(printf "%02d" $((j % 60)))'Z","sessionId":"med-session-'$i'","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}'
done)
EOF
done

# Create one large file (50000 lines)
echo "   Creating 1 large file (50000 lines)..."
cat > "$TEST_DIR/large.jsonl" << EOF
$(for j in {1..50000}; do
    echo '{"type":"user","message":{"role":"user","content":"Large file test message '$j' with extensive content including various keywords like error, warning, debug, info, and performance metrics"},"uuid":"large-'$j'","timestamp":"2024-01-01T00:00:'$(printf "%02d" $((j % 60)))'Z","sessionId":"large-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}'
done)
EOF

echo ""
echo "2. Building with async features..."
cargo build --release --features async

echo ""
echo "=== Running Performance Tests ==="
echo ""

# Function to run test and measure time
run_test() {
    local name=$1
    local pattern=$2
    local query=$3
    local engine=$4
    
    echo -n "   $name: "
    
    if [ "$engine" = "tokio" ]; then
        # For tokio, we need a small wrapper
        time -p cargo run --release --features async --bin ccms -- \
            --engine tokio \
            "$pattern" "$query" 2>&1 | grep -E "(real|found)"
    else
        # Default rayon
        time -p cargo run --release --bin ccms -- \
            "$pattern" "$query" 2>&1 | grep -E "(real|found)"
    fi
}

echo "3. Test 1: Single Small File"
echo "   Pattern: $TEST_DIR/small_1.jsonl"
echo "   Query: 'message'"
echo ""
echo "   Rayon:"
run_test "Rayon" "$TEST_DIR/small_1.jsonl" "message" "rayon"
echo ""
echo "   Tokio:"
run_test "Tokio" "$TEST_DIR/small_1.jsonl" "message" "tokio"

echo ""
echo "4. Test 2: Multiple Small Files"
echo "   Pattern: $TEST_DIR/small_*.jsonl"
echo "   Query: 'test AND message'"
echo ""
echo "   Rayon:"
run_test "Rayon" "$TEST_DIR/small_*.jsonl" "test AND message" "rayon"
echo ""
echo "   Tokio:"
run_test "Tokio" "$TEST_DIR/small_*.jsonl" "test AND message" "tokio"

echo ""
echo "5. Test 3: Medium Files"
echo "   Pattern: $TEST_DIR/medium_*.jsonl"
echo "   Query: 'production'"
echo ""
echo "   Rayon:"
run_test "Rayon" "$TEST_DIR/medium_*.jsonl" "production" "rayon"
echo ""
echo "   Tokio:"
run_test "Tokio" "$TEST_DIR/medium_*.jsonl" "production" "tokio"

echo ""
echo "6. Test 4: Large File"
echo "   Pattern: $TEST_DIR/large.jsonl"
echo "   Query: 'error OR warning'"
echo ""
echo "   Rayon:"
run_test "Rayon" "$TEST_DIR/large.jsonl" "error OR warning" "rayon"
echo ""
echo "   Tokio:"
run_test "Tokio" "$TEST_DIR/large.jsonl" "error OR warning" "tokio"

echo ""
echo "7. Test 5: Complex Query on All Files"
echo "   Pattern: $TEST_DIR/*.jsonl"
echo "   Query: '(error OR warning) AND performance'"
echo ""
echo "   Rayon:"
run_test "Rayon" "$TEST_DIR/*.jsonl" "(error OR warning) AND performance" "rayon"
echo ""
echo "   Tokio:"
run_test "Tokio" "$TEST_DIR/*.jsonl" "(error OR warning) AND performance" "tokio"

echo ""
echo "=== Cleanup ==="
echo "Test data is in: $TEST_DIR"
echo "To remove: rm -rf $TEST_DIR"
echo ""
echo "=== Test Complete ==="