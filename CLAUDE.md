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
   - Clean architecture with domain/application/UI layers
   - MVU (Model-View-Update) pattern for state management
   - Component-based UI design
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
- **Clean Architecture Layers**:
  - **Domain Layer**: Core business entities and models (`Mode`, `SearchRequest`, `SessionOrder`)
  - **Application Layer**: Business logic and services (`SearchService`, `SessionService`, `CacheService`)
  - **UI Layer**: Presentation logic with MVU pattern (`AppState`, `Message`, `Command`)
- **MVU (Model-View-Update) Pattern**:
  - **Model**: Centralized state in `AppState`
  - **View**: Component-based rendering with `Renderer`
  - **Update**: Message-driven state updates with side effects as `Command`s
- **Component-Based UI**:
  - Reusable components implementing `Component` trait
  - `SearchBar`, `ResultList`, `MessageDetail`, `SessionViewer`, `HelpDialog`
  - Each component manages its own rendering and input handling
- **Non-blocking Architecture**:
  - Event polling with 50ms timeout
  - Async search with message passing (mpsc channels)
  - Debounced search (300ms) for better UX
  - Visual feedback during async operations

### Critical Files

- `src/main.rs` - CLI entry point and argument parsing
- `src/search/engine.rs` - Core search implementation
- `src/query/parser.rs` - Query syntax parser
- `src/interactive_ratatui/mod.rs` - Interactive search entry point and coordination
- `src/interactive_ratatui/ui/app_state.rs` - Centralized state management with MVU pattern
- `src/interactive_ratatui/ui/renderer.rs` - Main UI renderer coordinating components
- `src/interactive_ratatui/domain/models.rs` - Core domain models and types

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

When adding new UI features to interactive mode:
1. **Add new Message** in `ui/events.rs` if needed
2. **Update State** in `ui/app_state.rs`:
   - Add new fields to relevant state structs
   - Handle new messages in `AppState::update()`
3. **Add Commands** in `ui/commands.rs` for side effects
4. **Update Components**:
   - Modify existing or create new components in `ui/components/`
   - Implement `Component` trait
   - Add component to `Renderer` if new
5. **Wire up in mod.rs**:
   - Handle commands in `execute_command()`
   - Add key bindings in `handle_input()`

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
- Clear separation between UI modes (Search, MessageDetail, SessionViewer, Help)
- Automatic cleanup on mode transitions (clear messages, reset scroll)
- Comprehensive caching system to minimize file I/O

### Testing Strategy

**Unit Testing Approach**:
The codebase follows a comprehensive testing strategy with tests organized by architectural layers:

1. **Domain Layer Tests** (`domain/*_test.rs`):
   - Test pure business logic and domain models
   - Focus on data structures and domain rules
   - Examples: `models_test.rs`, `filter_test.rs`

2. **Application Layer Tests** (`application/*_test.rs`):
   - Test service orchestration and business workflows
   - Mock external dependencies (file system, etc.)
   - Examples: `search_service_test.rs`, `session_service_test.rs`, `cache_service_test.rs`

3. **UI Layer Tests** (`ui/*_test.rs`):
   - Test state management and component behavior
   - Verify MVU pattern implementation
   - Examples: `app_state_test.rs`, component tests in `components/*_test.rs`

4. **Integration Tests** (`integration_tests.rs`):
   - Test interactions between layers
   - Verify end-to-end workflows
   - Test complete user scenarios

**Testing Best Practices**:
- Use descriptive test names that explain the scenario
- Each test should be independent and isolated
- Mock file system operations to avoid I/O dependencies
- Use builder patterns for complex test data setup
- Test edge cases (empty data, unicode, invalid input)

**Test Organization**:
```
src/interactive_ratatui/
├── domain/
│   ├── models_test.rs      # Domain model tests
│   └── filter_test.rs      # Filter logic tests
├── application/
│   ├── search_service_test.rs   # Search service tests
│   ├── session_service_test.rs  # Session service tests
│   └── cache_service_test.rs    # Cache service tests
├── ui/
│   ├── app_state_test.rs        # State management tests
│   └── components/
│       ├── search_bar_test.rs   # SearchBar component tests
│       └── result_list_test.rs  # ResultList component tests
└── integration_tests.rs         # Cross-layer integration tests
```

**Running Tests**:
```bash
# Run all tests
cargo test

# Run specific test module
cargo test interactive_ratatui::

# Run with output for debugging
cargo test -- --nocapture

# Run specific test function
cargo test test_search_filter
```

**Test Coverage Goals**:
- Critical business logic: 100% coverage
- UI components: Key interaction paths covered
- Error handling: All error cases tested
- Performance: Benchmarks for search operations

### Interactive Ratatui Architecture Details

**Directory Structure**:
```
src/interactive_ratatui/
├── mod.rs                    # Entry point, main event loop
├── domain/                   # Domain layer (business entities)
│   ├── models.rs            # Core types: Mode, SearchRequest, etc.
│   └── filter.rs            # Domain logic for filtering
├── application/             # Application layer (business logic)
│   ├── search_service.rs    # Search orchestration
│   ├── session_service.rs   # Session management
│   └── cache_service.rs     # File caching
└── ui/                      # UI layer (presentation)
    ├── app_state.rs         # Centralized state (MVU Model)
    ├── events.rs            # Message types (MVU Messages)
    ├── commands.rs          # Side effects (MVU Commands)
    ├── renderer.rs          # Main renderer
    └── components/          # Reusable UI components
        ├── search_bar.rs
        ├── result_list.rs
        ├── message_detail.rs
        ├── session_viewer.rs
        └── help_dialog.rs
```

**MVU Pattern Implementation**:

1. **Messages** (`ui/events.rs`):
   - User actions and system events
   - Examples: `QueryChanged`, `SearchCompleted`, `EnterMessageDetail`

2. **State** (`ui/app_state.rs`):
   - Single source of truth for UI state
   - `AppState::update(msg)` handles all state transitions
   - Returns `Command` for side effects

3. **Commands** (`ui/commands.rs`):
   - Side effects that can't be handled in pure update function
   - Examples: `ExecuteSearch`, `LoadSession`, `CopyToClipboard`

4. **Component Trait**:
   ```rust
   pub trait Component {
       fn render(&mut self, f: &mut Frame, area: Rect);
       fn handle_key(&mut self, key: KeyEvent) -> Option<Message>;
   }
   ```

**Key Architectural Benefits**:
- **Testability**: Pure functions for state updates
- **Maintainability**: Clear separation of concerns
- **Extensibility**: Easy to add new components or messages
- **Performance**: Efficient state updates and rendering

**Migration from Monolithic Design**:
The interactive UI was refactored from a 1882-line monolithic file to a clean architecture:
- **Before**: Single `InteractiveSearch` struct with 30+ fields (God Object)
- **After**: Layered architecture with clear responsibilities
- **Key Changes**:
  - Extracted domain models to separate layer
  - Created service layer for business logic
  - Implemented MVU pattern for predictable state management
  - Split UI into reusable components
  - Improved testability and maintainability