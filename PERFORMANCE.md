# Performance Optimizations

This document describes the performance optimizations implemented in CCMS.

## Implemented Optimizations

### 1. Regex Compilation Cache
- **Implementation**: LRU cache for compiled regex patterns
- **Location**: `src/query/regex_cache.rs`
- **Impact**: Reduces repeated regex compilation overhead

### 2. Memory-Mapped File I/O
- **Implementation**: Using `memmap2` for large file reading
- **Location**: `src/search/mmap_reader.rs`
- **Impact**: Faster file access for large JSONL files

### 3. SIMD-Accelerated JSON Parsing
- **Implementation**: Using `sonic-rs` for JSON parsing
- **Impact**: Hardware-accelerated JSON deserialization

### 4. Parallel File Processing
- **Implementation**: Using `rayon` for parallel search across multiple files
- **Impact**: Utilizes all CPU cores for multi-file searches

## Benchmark Results

### Realistic Workload Performance
- **10 files × 1,000 lines**:
  - Simple search ("error"): ~12.4ms
  - Complex search ("error AND code"): ~13.4ms
  - Regex search ("/error.*\\d+/i"): ~47.4ms
  
- **Single file with 100,000 lines**:
  - Simple search: ~191ms

### Performance Characteristics
- Linear scaling with file size
- Regex searches are ~3-4x slower than literal searches
- Multi-file searches benefit from parallel processing

## Future Optimization Opportunities

### Fast JSON Scanner (Implemented but not integrated)
- **Location**: `src/search/fast_json_scanner.rs`
- **Purpose**: Two-phase filtering to avoid full JSON parsing for non-matching lines
- **Potential Impact**: Could reduce parsing overhead by 30-50% for selective queries

### Optimized Search Engine (Implemented but not integrated)
- **Location**: `src/search/optimized_engine.rs`
- **Purpose**: Combines fast scanning with query hints for better performance
- **Potential Impact**: Could improve performance for specific query patterns

## Usage Tips for Best Performance

1. **Use literal searches when possible** - They are significantly faster than regex
2. **Use specific filters** - Role, session, and time filters reduce the search space
3. **Consider using `--project` flag** - Limits search to specific project directories
4. **For large datasets, use time filters** - `--since` flag can dramatically reduce search scope