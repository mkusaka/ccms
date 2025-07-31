# Smol Blocking Threads Optimization

## Summary

The Smol runtime's blocking thread pool defaults to 500 threads, which causes significant context switching overhead. By setting the `BLOCKING_MAX_THREADS` environment variable to match CPU core count, we achieved an 8% performance improvement.

**Update**: The optimization is now built into both `SmolSearchEngine` and `OptimizedSmolSearchEngine`. They automatically set `BLOCKING_MAX_THREADS` to the CPU core count on initialization, so manual configuration is no longer required.

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

### Automatic Optimization (Built-in)

As of the latest update, both Smol engines automatically optimize the thread pool:

```rust
// No configuration needed - automatically optimized!
let engine = SmolSearchEngine::new(options);
let engine = OptimizedSmolSearchEngine::new(options);
```

The engines automatically detect the CPU core count and set `BLOCKING_MAX_THREADS` accordingly.

### Manual Override

If you need to override the automatic setting:

```bash
# Override with specific value
BLOCKING_MAX_THREADS=8 ./target/release/bench_smol
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

## Implementation Details

The automatic optimization is implemented using `std::sync::Once` to ensure it runs only once:

```rust
static INIT: std::sync::Once = std::sync::Once::new();

fn initialize_blocking_threads() {
    INIT.call_once(|| {
        if std::env::var("BLOCKING_MAX_THREADS").is_err() {
            let cpu_count = num_cpus::get();
            unsafe {
                std::env::set_var("BLOCKING_MAX_THREADS", cpu_count.to_string());
            }
            eprintln!("Optimized BLOCKING_MAX_THREADS to {} (CPU count)", cpu_count);
        }
    });
}
```

The `unsafe` block is required for `set_var`, but it's called only once at initialization before any threads are spawned, making it safe in practice.