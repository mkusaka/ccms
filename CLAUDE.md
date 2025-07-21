# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Standard release build (optimized)
cargo build --release

# Development build (faster compilation, debugging enabled)
cargo build

# Build with profiling support
cargo build --release --features profiling

# Build with async support
cargo build --release --features async

# Build with all features
cargo build --release --all-features
```

## Test Commands

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for a specific module
cargo test query::

# Run with verbose output
cargo test -- --test-threads=1 --nocapture
```

## Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench search_benchmark

# Component benchmarks
cargo bench component_benchmark

# Async benchmarks (requires async feature)
cargo bench async_benchmark
```

## Development Commands

```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Update dependencies
cargo update

# Generate documentation
cargo doc --open

# Run with verbose logging
RUST_LOG=debug cargo run -- "query"

# Profile with flamegraph (requires profiling feature)
cargo run --release --features profiling -- --profile search_profile "query"
```

## Architecture Overview

### Module Structure

The codebase is organized into five main modules:

1. **query** - Query parsing and condition evaluation
   - `parser.rs`: Nom-based parser for search query syntax (AND/OR/NOT/regex)
   - `condition.rs`: Query condition types and evaluation logic
   
2. **schemas** - Data structures for Claude session messages
   - `session_message.rs`: Message types (User, Assistant, System, Summary)
   - `tool_result.rs`: Tool execution result parsing
   
3. **search** - Core search engine implementation
   - `engine.rs`: Main search logic with parallel file processing
   - `file_discovery.rs`: File pattern matching and discovery
   - `async_engine.rs`: Optional async implementation using tokio
   
4. **interactive_ratatui** - Interactive fzf-like search interface
   - Terminal UI using ratatui crate with crossterm backend
   - Real-time search with keyboard navigation
   
5. **profiling** - Performance profiling utilities
   - Flamegraph generation using pprof
   - Tracing integration for debugging

### Key Design Patterns

**Performance Optimizations**:
- SIMD-accelerated JSON parsing with `simd-json`
- Parallel file processing using `rayon`
- Memory-mapped file I/O for large files
- Early filtering to minimize allocations

**Query Processing Flow**:
1. Parse query string into `QueryCondition` AST
2. Discover files matching glob patterns
3. Process files in parallel thread pool
4. Parse JSONL with SIMD acceleration
5. Evaluate query conditions against messages
6. Apply filters (role, session, timestamp)
7. Format and display results

**Interactive Mode Architecture**:
- Uses `ratatui` crate with crossterm backend for terminal control
- Non-blocking input handling with event polling
- Maintains search state and cursor position
- Executes search with debouncing (300ms)
- Supports role filtering via Tab key
- Implements session viewer and clipboard operations

### Critical Files

- `src/main.rs` - CLI entry point and argument parsing
- `src/search/engine.rs` - Core search implementation
- `src/query/parser.rs` - Query syntax parser
- `src/interactive_ratatui/mod.rs` - Interactive search UI

### Feature Flags

- **profiling**: Enables flamegraph generation and performance profiling
- **async**: Enables tokio-based async search engine (experimental)

### Error Handling

Uses `anyhow` for error propagation with context. Critical errors include:
- Invalid query syntax (handled by parser)
- File I/O errors (handled gracefully)
- JSON parsing errors (logged if verbose, otherwise skipped)

### Testing Strategy

- Unit tests for query parser and conditions
- Integration tests for search engine
- Benchmarks for performance regression testing
- Component benchmarks for specific operations

### Common Modifications

When adding new search features:
1. Update `QueryCondition` enum in `query/condition.rs`
2. Extend parser in `query/parser.rs`
3. Add evaluation logic to `evaluate()` method
4. Update CLI args in `main.rs` if needed

When optimizing performance:
1. Run benchmarks before changes: `cargo bench`
2. Profile with flamegraph: `cargo run --release --features profiling -- --profile baseline "query"`
3. Make changes
4. Compare benchmark results
5. Generate new flamegraph to verify improvements

### Development Methodology

**Version Control Practices**:
- Commit frequently after completing small, logical units of work
- Each commit should represent a single, coherent change
- Write clear commit messages that explain the "why" not just the "what"
- When asked to make changes, implement → test → commit before moving to next task

**Test-Driven Development (TDD)**:
The interactive UI was developed using TDD methodology:
1. Write specifications first (see `spec.md`)
2. Create comprehensive tests before implementation
3. Implement features to make tests pass
4. Refactor while maintaining test coverage

**Non-blocking UI Implementation**:
The interactive mode uses non-blocking input handling to prevent UI freezing:
- Uses `crossterm::event::poll()` with 50ms timeout
- Implements debouncing (300ms) for search queries
- Provides visual feedback ("typing...", "searching...")
- Maintains separate search state to prevent race conditions

**Multibyte Character Safety**:
- All string operations use character-based indexing, not byte-based
- Prevents Unicode boundary errors with Japanese text and emojis
- Dynamic text truncation respects character boundaries

**State Management**:
- Clear separation between UI modes (Search, ResultDetail, SessionViewer, Help)
- Automatic cleanup on mode transitions (clear messages, reset scroll)
- Comprehensive caching system to minimize file I/O