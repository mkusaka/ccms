#!/bin/bash

echo "=== Final Performance Comparison ==="
echo "Testing all optimizations against baseline..."
echo

# Build everything
echo "Building all versions..."
cargo build --release --features "async" --quiet
cargo build --release --features "async sonic jemalloc" --quiet

# Test 1: Original Rayon (baseline)
echo "1. Baseline Rayon implementation:"
hyperfine -w 2 -r 5 --style basic \
    './target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine rayon'

# Test 2: Original Tokio (without optimizations)
echo -e "\n2. Original Tokio implementation (no optimizations):"
# We need to build without sonic and jemalloc
cargo build --release --features "async" --quiet
cp target/release/ccms target/release/ccms_original
hyperfine -w 2 -r 5 --style basic \
    './target/release/ccms_original "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine tokio'

# Test 3: Optimized Tokio (with sonic-rs + jemalloc)
echo -e "\n3. Optimized Tokio (sonic-rs + jemalloc + other optimizations):"
cargo build --release --features "async sonic jemalloc" --quiet
hyperfine -w 2 -r 5 --style basic \
    './target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine tokio'

# Test 4: Worker Pool V4
echo -e "\n4. Worker Pool Pattern V4:"
./target/release/test_v4 > /dev/null 2>&1  # Just to make sure it's built
cat > src/bin/bench_v4.rs << 'EOF'
use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_async_engine_v4::OptimizedAsyncSearchEngineV4;

#[tokio::main]
async fn main() -> Result<()> {
    let pattern = "~/.claude/projects/**/*.jsonl";
    let query = parse_query("claude")?;
    let options = SearchOptions::default();
    let engine = OptimizedAsyncSearchEngineV4::new(options);
    let (_results, _duration, total) = engine.search(pattern, query).await?;
    println!("Found {} results", total);
    Ok(())
}
EOF
cargo build --release --features "async sonic jemalloc" --bin bench_v4 --quiet
hyperfine -w 2 -r 5 --style basic './target/release/bench_v4'

# Summary comparison
echo -e "\n=== Final Summary ==="
echo "Running head-to-head comparison of all versions..."
hyperfine -w 3 -r 10 --export-json final_results.json \
    './target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine rayon' \
    './target/release/ccms_original "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine tokio' \
    './target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine tokio' \
    './target/release/bench_v4' \
    --names "Rayon (baseline)","Tokio (original)","Tokio (optimized)","Tokio V4 (worker pool)"

# Calculate improvements
echo -e "\n=== Performance Improvements ==="
python3 << 'EOF'
import json

with open('final_results.json', 'r') as f:
    data = json.load(f)

results = data['results']
baseline_mean = results[0]['mean']  # Rayon
tokio_original_mean = results[1]['mean']
tokio_optimized_mean = results[2]['mean']
tokio_v4_mean = results[3]['mean']

print(f"Baseline (Rayon):        {baseline_mean*1000:.1f} ms")
print(f"Tokio Original:          {tokio_original_mean*1000:.1f} ms")
print(f"Tokio Optimized:         {tokio_optimized_mean*1000:.1f} ms")
print(f"Tokio V4 Worker Pool:    {tokio_v4_mean*1000:.1f} ms")
print()

tokio_vs_rayon = ((tokio_original_mean - baseline_mean) / baseline_mean) * 100
print(f"Original Tokio vs Rayon: {tokio_vs_rayon:+.1f}%")

optimized_improvement = ((tokio_original_mean - tokio_optimized_mean) / tokio_original_mean) * 100
print(f"Optimized vs Original Tokio: {optimized_improvement:+.1f}% improvement")

v4_vs_optimized = ((tokio_optimized_mean - tokio_v4_mean) / tokio_optimized_mean) * 100
print(f"V4 vs Optimized Tokio: {v4_vs_optimized:+.1f}% improvement")

total_improvement = ((tokio_original_mean - tokio_v4_mean) / tokio_original_mean) * 100
print(f"\nTotal Tokio improvement: {total_improvement:+.1f}%")

final_vs_rayon = ((tokio_v4_mean - baseline_mean) / baseline_mean) * 100
print(f"Final Tokio V4 vs Rayon: {final_vs_rayon:+.1f}%")
EOF