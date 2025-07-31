# Smol Blocking Threads Optimization

## Summary

The Smol runtime's blocking thread pool defaults to 500 threads, which causes significant context switching overhead. By setting the `BLOCKING_MAX_THREADS` environment variable to match CPU core count, we achieved an 8% performance improvement.

## Performance Results

Test environment: 10-core CPU

| Configuration | Mean Time | Relative Performance |
|--------------|-----------|---------------------|
| Default (500 threads) | 231.9ms | Baseline |
| BLOCKING_MAX_THREADS=1 | 1319ms | 5.7x slower |
| BLOCKING_MAX_THREADS=4 | 381.5ms | 1.6x slower |
| BLOCKING_MAX_THREADS=8 | 234.1ms | 1.0x slower |
| BLOCKING_MAX_THREADS=10 | **213.8ms** | **8% faster** |
| BLOCKING_MAX_THREADS=12 | 216.9ms | 6% faster |
| BLOCKING_MAX_THREADS=16 | 233.3ms | ~same |
| BLOCKING_MAX_THREADS=20 | 231.3ms | ~same |

## Key Findings

1. **Optimal Value**: Setting `BLOCKING_MAX_THREADS` to the CPU core count provides the best performance
2. **Default Overhead**: The default 500 threads cause excessive context switching
3. **No Unsafe Code**: This optimization can be applied without modifying the code

## Usage

Run the Smol engine with optimized thread pool:

```bash
BLOCKING_MAX_THREADS=$(sysctl -n hw.ncpu) ./target/release/bench_smol
```

Or set it to a specific value:

```bash
BLOCKING_MAX_THREADS=10 ./target/release/bench_smol
```

## Technical Details

The Smol blocking thread pool is used for CPU-intensive operations like JSON parsing. The default 500 threads create unnecessary overhead:

- **Context Switching**: With 500 threads on a 10-core CPU, threads constantly switch contexts
- **Cache Thrashing**: Excessive threads lead to poor CPU cache utilization
- **Scheduling Overhead**: The OS scheduler struggles with 500 threads competing for 10 cores

By matching the thread count to CPU cores, we:
- Minimize context switching
- Improve CPU cache utilization
- Reduce scheduling overhead

## Recommendation

For production deployments of Smol-based applications, always set:
```bash
export BLOCKING_MAX_THREADS=$(nproc)  # Linux
export BLOCKING_MAX_THREADS=$(sysctl -n hw.ncpu)  # macOS
```

This provides optimal performance without code changes or unsafe blocks.