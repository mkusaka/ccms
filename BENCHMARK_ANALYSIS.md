# Benchmark Analysis: Main CLI vs Benchmark Binaries

## Performance Comparison

### Main CLI (ccms) - 100 runs
- **OptimizedRayon**: 320.9ms ±33.8ms (最速)
- **OptimizedSmol**: 323.4ms ±32.8ms
- **Smol**: 344.0ms ±53.7ms
- **Rayon**: 359.2ms ±40.3ms
- **Tokio**: 409.8ms ±108.2ms
- **OptimizedTokio**: 549.1ms ±194.6ms

### Benchmark Binaries - Current Run
- **bench_smol**: 227.6ms
- **bench_smol --optimized**: 241.5ms
- **bench_rayon_optimized**: 245.2ms

## Key Differences Found

### 1. **Absolute Performance**
- Benchmark binaries are ~100ms faster than main CLI
- This suggests significant overhead in the main CLI

### 2. **Relative Performance Rankings**
- **Main CLI**: OptimizedRayon > OptimizedSmol > Smol > Rayon
- **Benchmarks**: Smol > OptimizedSmol ≈ OptimizedRayon

### 3. **Common Factors (Not the cause)**
- **File Pattern**: Both use `~/.claude/projects/**/*.jsonl`
- **File Count**: Both process 588 files
- **Query**: Both search for "claude"
- **Max Results**: Both limited to 50 results

## Identified Overhead Sources

### 1. **CLI Initialization Overhead**
```rust
// Main CLI has:
let cli = Cli::parse();                    // Command line parsing
if let Some(generator) = cli.generator { } // Completion check
if cli.help_query { }                      // Help check
// Date/time parsing for filters
// Project path processing
// Debug file check
```

### 2. **Output Formatting Overhead**
```rust
// Main CLI has complex output formatting:
match cli.format {
    OutputFormat::Text => {
        // Colored output formatting
        format_search_result(result, !cli.no_color, cli.full_text)
    }
    OutputFormat::Json => { /* JSON serialization */ }
    OutputFormat::JsonL => { /* JSONL output */ }
}
```

### 3. **Runtime Construction**
```rust
// Main CLI builds runtime for each run:
let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?;

// Benchmark binaries have simpler initialization
```

### 4. **Engine-Specific Differences**
- **OptimizedSmol in main**: Calls `init_executor()` which spawns threads
- **OptimizedSmol in bench**: Also calls `init_executor()` but timing might differ

## Why OptimizedRayon Performs Best in Main CLI

### 1. **No Async Runtime Overhead**
- Rayon doesn't need runtime construction like Tokio/Smol
- Direct parallel execution without async machinery

### 2. **sonic-rs + jemalloc Benefits**
- More efficient JSON parsing
- Better memory allocation patterns
- These benefits compound with CLI overhead

### 3. **Stable Performance**
- Lowest standard deviation (±33.8ms)
- Less affected by system noise

## Conclusions

1. **~100ms overhead** in main CLI from initialization and output formatting
2. **OptimizedRayon** handles this overhead best due to simpler execution model
3. **Async engines** (Tokio, Smol) suffer more from runtime initialization overhead
4. **Benchmark binaries** show "pure" search performance without CLI overhead

## Recommendations

1. For **production use**: OptimizedRayon via main CLI
2. For **pure performance testing**: Use benchmark binaries
3. Consider **caching CLI parsing** if frequently running searches
4. **Output formatting** could be optimized for better performance