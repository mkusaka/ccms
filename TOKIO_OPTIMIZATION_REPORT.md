# Tokio Performance Optimization Report

## Executive Summary

We successfully optimized the Tokio async implementation to achieve a **17% performance improvement** over the original implementation. While Rayon remains faster (by ~21%), the optimized Tokio version significantly narrows the performance gap.

## Performance Benchmarks

| Implementation | Mean Time | vs Rayon | vs Original Tokio |
|----------------|-----------|----------|-------------------|
| Rayon (baseline) | 198.2ms | - | - |
| Tokio (original) | 288.6ms | +46% | - |
| Tokio (optimized) | 239.8ms | +21% | -17% |

## Successful Optimizations

### 1. sonic-rs JSON Parser (8.4% improvement)
- **What**: Replaced simd-json with sonic-rs for JSON parsing
- **Why**: sonic-rs provides better performance for our JSON structures
- **Implementation**: Added feature flag and conditional compilation
- **Result**: 8.4% faster for Rayon, 2.0% faster for Tokio

### 2. jemalloc Global Allocator (5.7% improvement)
- **What**: Replaced system allocator with jemalloc
- **Why**: Better memory allocation performance for concurrent workloads
- **Implementation**: Added as global allocator with feature flag
- **Result**: Additional 5.7% improvement when combined with sonic-rs

### 3. ASCII Lowercasing Optimization (~1% improvement)
- **What**: Optimized case-insensitive string comparisons for ASCII text
- **Why**: Most search text is ASCII, avoiding Unicode overhead
- **Implementation**: Fast path for ASCII strings
- **Result**: Reduced to_lowercase CPU usage from 5.51% to ~5%

### 4. Worker Pool Pattern (10% improvement in isolated tests)
- **What**: Fixed pool of workers instead of spawning task per file
- **Why**: Reduces task spawning overhead
- **Implementation**: Created OptimizedAsyncSearchEngineV4
- **Result**: ~10% improvement, also enables buffer reuse

### 5. Enhanced Profiling
- **What**: Text-based profiling reports for better analysis
- **Why**: SVG reports were hard to analyze programmatically
- **Implementation**: Enhanced profiler with comprehensive text reports
- **Result**: Enabled data-driven optimization decisions

## Optimizations Tested but Not Adopted

### 1. Memory-mapped I/O
- **Result**: Performance degraded, async overhead negated benefits
- **Decision**: Rejected

### 2. Increased Concurrency (200 concurrent files)
- **Result**: Made performance worse due to contention
- **Decision**: Kept at num_cpus * 2

### 3. Larger Buffer Sizes (256KB)
- **Result**: Worse performance for Tokio
- **Decision**: Kept at 64KB

### 4. 10MB File Size Threshold
- **Result**: Performance degradation
- **Decision**: Kept 1MB threshold

### 5. mimalloc Allocator
- **Result**: 1.59x slower than jemalloc
- **Decision**: Used jemalloc instead

### 6. Runtime Thread Tuning
- **Result**: Default configuration performed better
- **Decision**: Kept default Tokio runtime settings

## Key Learnings

1. **JSON Parsing is Critical**: sonic-rs provided the biggest single improvement
2. **Memory Allocation Matters**: jemalloc significantly improved concurrent performance
3. **Task Spawning Overhead**: Worker pools can reduce overhead for many small tasks
4. **Not All Optimizations Help**: Many "optimizations" actually made things worse
5. **Profiling is Essential**: Data-driven decisions led to actual improvements

## Remaining Bottlenecks

Based on profiling, the main remaining bottlenecks are:
1. Tokio's async overhead (~54% in task polling)
2. File I/O operations (~16%)
3. JSON parsing (still ~12% even with sonic-rs)
4. String operations and regex matching

## Future Optimization Opportunities

1. **LocalSet for CPU-bound work**: Move JSON parsing to dedicated threads
2. **Streaming JSON parsing**: For very large files
3. **io_uring support**: When Tokio adds support for Linux io_uring
4. **SIMD string matching**: Custom SIMD implementations for common patterns
5. **Zero-copy parsing**: Avoid allocations during JSON parsing

## Conclusion

We achieved a significant 17% performance improvement through careful profiling and targeted optimizations. The most impactful changes were switching to sonic-rs JSON parser and jemalloc allocator. While Rayon remains faster due to lower overhead for CPU-bound workloads, the optimized Tokio implementation is now much more competitive and provides better async I/O capabilities for future enhancements.