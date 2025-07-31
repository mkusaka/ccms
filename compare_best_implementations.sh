#\!/bin/bash

echo "=== 最速実装の比較: Rayon vs Tokio V4 with all optimizations ==="
echo

# Ensure we have the latest builds with all optimizations
echo "Building with all optimizations..."
cargo build --release --features "async sonic jemalloc" --quiet

# Create a simple binary that uses V4 with all optimizations
cat > src/bin/best_tokio.rs << 'RUST_EOF'
use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_async_engine_v4::OptimizedAsyncSearchEngineV4;

#[tokio::main]
async fn main() -> Result<()> {
    let pattern = std::env::args().nth(1).unwrap_or_else(|| "~/.claude/projects/**/*.jsonl".to_string());
    let query_str = std::env::args().nth(2).unwrap_or_else(|| "claude".to_string());
    
    let query = parse_query(&query_str)?;
    let options = SearchOptions::default();
    
    let engine = OptimizedAsyncSearchEngineV4::new(options);
    let (_results, _duration, _total) = engine.search(&pattern, query).await?;
    
    Ok(())
}
RUST_EOF

cargo build --release --features "async sonic jemalloc" --bin best_tokio --quiet

echo "Running hyperfine comparison..."
echo

# Run detailed comparison
hyperfine -w 3 -r 20 --export-json best_comparison.json \
    './target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine rayon' \
    './target/release/best_tokio "~/.claude/projects/**/*.jsonl" "claude"' \
    --names "Rayon (sonic-rs + jemalloc)","Tokio V4 (sonic-rs + jemalloc + worker pool)"

echo -e "\n=== Statistical Analysis ==="
python3 << 'PYTHON_EOF'
import json
import math

with open('best_comparison.json', 'r') as f:
    data = json.load(f)

results = data['results']
rayon = results[0]
tokio = results[1]

# Extract statistics
rayon_mean = rayon['mean'] * 1000  # Convert to ms
rayon_stddev = rayon['stddev'] * 1000
rayon_min = rayon['min'] * 1000
rayon_max = rayon['max'] * 1000

tokio_mean = tokio['mean'] * 1000
tokio_stddev = tokio['stddev'] * 1000
tokio_min = tokio['min'] * 1000
tokio_max = tokio['max'] * 1000

print(f"Rayon Performance:")
print(f"  Mean:     {rayon_mean:.1f} ms ± {rayon_stddev:.1f} ms")
print(f"  Min/Max:  {rayon_min:.1f} ms / {rayon_max:.1f} ms")
print(f"  CV:       {(rayon_stddev/rayon_mean)*100:.1f}%")
print()

print(f"Tokio V4 Performance:")
print(f"  Mean:     {tokio_mean:.1f} ms ± {tokio_stddev:.1f} ms")
print(f"  Min/Max:  {tokio_min:.1f} ms / {tokio_max:.1f} ms")
print(f"  CV:       {(tokio_stddev/tokio_mean)*100:.1f}%")
print()

# Performance comparison
diff_ms = tokio_mean - rayon_mean
diff_pct = ((tokio_mean - rayon_mean) / rayon_mean) * 100

print(f"Performance Difference:")
print(f"  Absolute: {diff_ms:.1f} ms slower")
print(f"  Relative: {diff_pct:.1f}% slower than Rayon")
print()

# Statistical significance (simplified t-test approximation)
# This is a rough approximation - for real analysis use proper statistical tests
pooled_std = math.sqrt((rayon_stddev**2 + tokio_stddev**2) / 2)
t_stat = abs(tokio_mean - rayon_mean) / (pooled_std * math.sqrt(2/20))
print(f"Statistical Analysis:")
print(f"  t-statistic: {t_stat:.2f}")
print(f"  Significant: {'Yes' if t_stat > 2.0 else 'No'} (at p < 0.05)")
PYTHON_EOF

echo -e "\n=== Summary ==="
echo "Both implementations use:"
echo "- sonic-rs JSON parser"
echo "- jemalloc memory allocator"
echo "- ASCII lowercasing optimization"
echo ""
echo "Tokio V4 additionally uses:"
echo "- Worker pool pattern"
echo "- Buffer reuse"
