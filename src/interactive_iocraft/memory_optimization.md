# Memory Optimization for Large Files

## Current Implementation Analysis

The current `CacheService` loads entire files into memory, which can be problematic for very large session files (>100MB).

## Optimization Strategies

### 1. Streaming JSON Parser
Instead of loading entire files into memory, implement a streaming JSON parser that processes messages one at a time.

### 2. Memory-Mapped Files
Use memory-mapped files for large files to avoid loading entire content into RAM.

### 3. LRU Cache with Size Limits
Implement an LRU (Least Recently Used) cache with configurable size limits to automatically evict old entries.

### 4. Lazy Loading
Load messages on-demand rather than all at once, especially for session viewing.

### 5. Compression
Store cached messages in compressed format to reduce memory usage.

## Implementation Plan

1. **Update CacheService** to use streaming parser
2. **Add memory limits** to cache configuration
3. **Implement LRU eviction** when cache size exceeds limit
4. **Add compression** for cached data
5. **Use iterators** instead of loading all messages at once