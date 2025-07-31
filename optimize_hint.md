# Cross-Engine Optimization Opportunities

This document tracks optimization techniques that can be applied across different search engines (Rayon, Tokio, Smol), showing which engines have implemented each optimization and which haven't.

## 1. sonic-rs JSON Parser (30-50% improvement potential)

High-performance JSON parser that significantly outperforms simd-json.

### Implementation Status:
- ✅ **Implemented:**
  - OptimizedRayon (`optimized_rayon_engine.rs`)
  - OptimizedRayonV2 (`optimized_rayon_engine_v2.rs`)
  - OptimizedRayonV3 (`optimized_rayon_engine_v3.rs`)
  - OptimizedSmol (`optimized_smol_engine.rs`)
  - Smol (`smol_engine.rs`)
  - OptimizedTokio (`optimized_async_engine.rs`, `optimized_async_engine_v2.rs`)
  - OptimizedAsyncV4 (`optimized_async_engine_v4.rs`)

- ❌ **Not Implemented:**
  - **Basic Rayon** (`engine.rs`) - Still using `simd_json::serde::from_slice`
  - **Basic Tokio** (`async_engine.rs`) - Still using `simd_json::serde::from_slice`

### Implementation Example:
```rust
// Replace:
simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes)

// With:
sonic_rs::from_str(&line)  // for string input
sonic_rs::from_slice(&json_bytes)  // for byte slice input
```

## 2. Buffer Size Optimization (3-5% improvement)

Consistent buffer sizes improve I/O efficiency.

### Implementation Status:
- 🟨 **Inconsistent Implementation:**
  - Basic Rayon: 32KB
  - Most Optimized engines: 64KB
  - Smol engines: 64KB
  - Interactive UI: 64KB (FILE_READ_BUFFER_SIZE)

### Recommendation:
Standardize to 64KB across all engines:
```rust
let reader = BufReader::with_capacity(64 * 1024, file);
```

## 3. Vec Pre-allocation (5-10% improvement)

Pre-allocating vectors reduces memory allocations during runtime.

### Implementation Status:
- ✅ **Implemented:**
  - OptimizedSmol: `Vec::with_capacity(32)`
  - AsyncEngine: `Vec::with_capacity(max_results)`
  - OptimizedAsyncV3: `Vec::with_capacity(self.result_batch_size)`

- ❌ **Not Implemented:**
  - Basic Rayon
  - OptimizedRayon (main results vector)
  - Basic Smol
  - Basic Tokio (partially)
  - Most other engines

### Implementation Example:
```rust
// Replace:
let mut results = Vec::new();

// With:
let mut results = Vec::with_capacity(32); // typical result size
```

## 4. Line Buffer Reuse (2-3% improvement)

Reusing line buffers reduces allocations in tight loops.

### Implementation Status:
- ✅ **Implemented:**
  - OptimizedRayon: `Vec::with_capacity(8 * 1024)`
  - OptimizedRayonV2: `String::with_capacity(8 * 1024)`

- ❌ **Not Implemented:**
  - All other engines

### Implementation Example:
```rust
let mut line_buffer = String::with_capacity(8 * 1024);
// or
let mut line_buffer = Vec::with_capacity(8 * 1024);
```

## 5. BytesMut Usage for Async (Memory efficiency)

Using `bytes::BytesMut` for efficient buffer management in async contexts.

### Implementation Status:
- ✅ **Implemented:**
  - All OptimizedAsync engines (V1, V2, V3)

- 🔷 **Alternative for Sync:**
  - Can use `String::with_capacity` or `Vec::with_capacity` for similar benefits

## 6. Early Filtering (Variable improvement)

Filtering before JSON parsing where possible.

### Implementation Status:
- 🟨 **Partial Implementation:**
  - Some engines check role/session filters after parsing
  - Could be optimized to filter based on file path or other metadata first

## 7. Memory-mapped I/O (Experimental)

Advanced I/O optimization for large files.

### Implementation Status:
- ✅ **Implemented:**
  - OptimizedAsyncV4 (experimental)

- ❌ **Not Implemented:**
  - All other engines

### Note:
Still experimental, needs more testing before widespread adoption.

## 8. Jemalloc Memory Allocator (10-15% improvement)

Efficient memory allocator that reduces fragmentation.

### Implementation Status:
- ✅ **Global Implementation:**
  - Enabled in `main.rs` with feature flag
  - Benefits all engines when `jemalloc` feature is enabled

## Priority Recommendations

1. **High Priority** (Quick wins with big impact):
   - Apply sonic-rs to basic Rayon and Tokio engines
   - Standardize buffer sizes to 64KB

2. **Medium Priority** (Good improvements):
   - Add Vec pre-allocation to all engines
   - Implement line buffer reuse where applicable

3. **Low Priority** (Marginal gains):
   - Early filtering optimizations
   - Memory-mapped I/O (needs more testing)

## Expected Combined Impact

If all optimizations are applied to basic engines:
- Basic Rayon: ~40-60% improvement expected
- Basic Tokio: ~40-60% improvement expected
- Already optimized engines: ~5-15% additional improvement possible