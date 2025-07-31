#!/bin/bash

# Compare Rayon vs Tokio on real Claude session data

echo "=== CCMS Performance Comparison on Real Data ==="
echo ""

# Default pattern for Claude sessions
PATTERN="${1:-~/.claude/projects/*/*.jsonl}"
QUERY="${2:-claude}"

echo "Pattern: $PATTERN"
echo "Query: $QUERY"
echo ""

# Build with async features
echo "Building..."
cargo build --release --features async

echo ""
echo "=== Rayon Engine ==="
time ./target/release/ccms "$PATTERN" "$QUERY" | tail -5

echo ""
echo "=== Tokio Engine (Optimized) ==="
time ./target/release/perf_test tokio "$PATTERN" "$QUERY"

echo ""
echo "To test with different queries:"
echo "  ./compare_real_data.sh \"pattern\" \"query\""
echo ""
echo "Examples:"
echo "  ./compare_real_data.sh \"~/.claude/projects/*/*.jsonl\" \"error\""
echo "  ./compare_real_data.sh \"~/.claude/projects/*/*.jsonl\" \"function AND async\""
echo "  ./compare_real_data.sh \"~/.claude/projects/*/*.jsonl\" \"/[Ee]rror.*[0-9]+/\""