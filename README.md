# CCMS (Claude Code Message Searcher)

[![CI](https://github.com/mkusaka/ccms/actions/workflows/ci.yml/badge.svg)](https://github.com/mkusaka/ccms/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

High-performance CLI for searching Claude session JSONL files with an interactive TUI mode.

## Features

- 🚀 **Blazing Fast**: SIMD-accelerated JSON parsing with parallel file processing
- 🔍 **Powerful Query Syntax**: Boolean operators (AND/OR/NOT), regex, and quoted literals
- 🎯 **Smart Filtering**: Filter by role, session ID, timestamp ranges, and project paths
- 💻 **Interactive Mode**: fzf-like TUI for real-time search and navigation
- 📊 **Multiple Output Formats**: Text, JSON, or JSONL with customizable formatting
- 🎨 **Beautiful Output**: Colored terminal output with match highlighting
- 🔧 **Robust Testing**: Comprehensive test suite with cargo-nextest support
- 🚀 **Shell Completion**: Auto-completion support for bash, zsh, and fish shells

## Installation

### From GitHub (Recommended)

```bash
# Install directly from GitHub
cargo install --git https://github.com/mkusaka/ccms

# Or install a specific version/tag
cargo install --git https://github.com/mkusaka/ccms --tag v0.0.1
```

### From Source

```bash
# Clone the repository
git clone https://github.com/mkusaka/ccms.git
cd ccms

# Build and install
cargo install --path .
```

### Manual Build

```bash
# Build release version
cargo build --release

# Copy to your PATH
cp target/release/ccms ~/.local/bin/
# or
sudo cp target/release/ccms /usr/local/bin/
```

## Shell Completion

CCMS supports shell completion for bash, zsh, and fish. To enable it:

### Bash

```bash
# Generate completion script to file
ccms --completion bash > ~/.bash_completion.d/ccms

# Or add to .bashrc for persistent completion
echo 'source <(ccms --completion bash)' >> ~/.bashrc

# Or enable completion immediately in current shell
source <(ccms --completion bash)
```

### Zsh

```bash
# Generate completion script to file
ccms --completion zsh > ~/.zsh/completions/_ccms

# Or add to .zshrc for persistent completion
echo 'source <(ccms --completion zsh)' >> ~/.zshrc

# Or enable completion immediately in current shell
source <(ccms --completion zsh)
```

### Fish

```bash
# Generate completion script
ccms --completion fish > ~/.config/fish/completions/ccms.fish
```

After installation, restart your shell or source your configuration file to enable completions.

## Usage

### Basic Search

```bash
# Search for "error" in all Claude sessions
ccms "error"

# Search in specific files
ccms -p "~/.claude/projects/myproject/*.jsonl" "bug"

# Filter by role
ccms -r user "how to"
ccms -r assistant "I can help"

# Filter by current project directory
ccms --project "$(pwd)" "TODO"
```

### Interactive Mode (TUI)

Launch an interactive search interface similar to fzf. All filtering options work in interactive mode:

```bash
# Interactive search in default location
ccms -i

# Interactive search in specific directory
ccms -i -p "~/my-project/*.jsonl"

# Interactive search with filters
ccms -i --project $(pwd)                    # Current project only
ccms -i --since "1 day ago"                  # Recent messages only
ccms -i -r user                              # Pre-filter by role
ccms -i --project $(pwd) --since "2 hours ago"  # Combine filters

# All standard filters are supported
ccms -i -s "session-id"                      # Filter by session
ccms -i --after "2024-01-01T00:00:00Z"       # Time range filters
ccms -i -n 100                               # Adjust result limit
```

**Interactive Mode Controls:**
- Type to search in real-time
- `↑/↓` - Navigate results
- `Enter` - View full message
- `Tab` - Cycle role filters (all → user → assistant → system → summary)
- `Ctrl+R` - Clear cache and reload files
- `Ctrl+T` - Toggle message truncation (Truncated/Full Text)
- `Alt+←` - Navigate back through history
- `Alt+→` - Navigate forward through history
- `Ctrl+C (2x)` - Exit (press twice within 1 second)
- `Esc` - Go back to previous screen (does not exit from search screen)

**Note on Filters in Interactive Mode:**
- All command-line filters (`--project`, `--since`, `--after`, `--before`, `-s`, etc.) are applied as base filters
- The `-r` flag sets the initial role filter, but you can still cycle through roles with Tab
- Filters persist throughout the interactive session
- Results are limited by the `-n` flag (default: 50, but 20x more are loaded for scrolling)

**Result Actions:**
- `S` - View full session (Session Viewer)
- `F` - Copy file path to clipboard
- `I` - Copy session ID to clipboard
- `P` - Copy project path to clipboard
- `M` - Copy message text to clipboard
- `R` - Copy raw JSON to clipboard

**Session Viewer Controls:**
- `↑/↓` or `j/k` - Navigate messages
- `Tab` - Cycle role filters (all → user → assistant → system)
- `/` - Search within session (Tab works in search mode too)
- `O` - Toggle sort order
- `Enter` - View message detail
- `C` - Copy selected message
- `Shift+C` - Copy all visible messages
- `I` - Copy session ID
- `F` - Copy file path
- `Esc` - Return to previous screen

### Advanced Queries

```bash
# AND operator
ccms "error AND connection"

# OR operator
ccms "warning OR error"

# NOT operator
ccms "response NOT error"

# Complex queries with parentheses
ccms "(error OR warning) AND NOT /test/i"

# Regular expressions
ccms "/failed.*connection/i"
ccms "/^Error:.*\d+/m"
```

### Filtering Options

```bash
# Limit results
ccms -n 100 "search term"

# Filter by session ID
ccms -s "session-123" "query"

# Filter by timestamp
ccms --after "2024-01-01T00:00:00Z" "recent"
ccms --before "2024-12-31T23:59:59Z" "old"

# Filter using relative time or Unix timestamp
ccms --since "1 day ago" "recent activity"
ccms --since "2 hours ago" "latest changes"
ccms --since "yesterday" "yesterday's work"
ccms --since "last week" "weekly review"
ccms --since "3 days ago" "recent work"
ccms --since 1720000000 "since Unix timestamp"

# Filter by project path
ccms --project "/Users/me/project" "bug"

# Combine filters
ccms -r user -n 20 --after "2024-06-01T00:00:00Z" "question"
```

### Output Formats

```bash
# Default text output with colors
ccms "query"

# Disable colors
ccms --no-color "query"

# Show full message text
ccms --full-text "query"

# Show raw JSON of matched messages
ccms --raw "query"

# JSON output
ccms -f json "query" > results.json

# JSONL output (one JSON per line)
ccms -f jsonl "query" > results.jsonl

# Verbose output with debug info
ccms -v "query"
```

## CLI Options

### General Options
- `-p, --pattern <PATTERN>` - File pattern to search (default: `~/.claude/projects/**/*.jsonl`)
- `-n, --max-results <N>` - Maximum number of results to return (default: 50)
- `-f, --format <FORMAT>` - Output format: `text`, `json`, or `jsonl` (default: text)
- `-v, --verbose` - Enable verbose output
- `--no-color` - Disable colored output
- `--full-text` - Show full message text without truncation
- `--raw` - Show raw JSON of matched messages

### Filtering Options
- `-r, --role <ROLE>` - Filter by message role: `user`, `assistant`, `system`, or `summary`
- `-s, --session-id <ID>` - Filter by session ID
- `--project <PATH>` - Filter by project path (e.g., current directory: `$(pwd)`)
- `--before <TIMESTAMP>` - Filter messages before this timestamp (RFC3339 format)
- `--after <TIMESTAMP>` - Filter messages after this timestamp (RFC3339 format)
- `--since <TIME>` - Filter messages since this time (relative time like "1 day ago" or Unix timestamp)

### Interactive Mode
- `-i, --interactive` - Launch interactive search mode (fzf-like TUI)

### Other Options
- `--help-query` - Show query syntax help
- `--completion <SHELL>` - Generate shell completion script for bash, zsh, or fish
- `--profile <NAME>` - Generate profiling report (requires --features profiling)
- `-h, --help` - Print help information
- `-V, --version` - Print version information

## Query Syntax Reference

### Basic Queries
- `hello` - Case-insensitive literal search
- `"hello world"` - Quoted literal (preserves spaces)
- `'hello world'` - Single-quoted literal
- `/pattern/flags` - Regular expression with optional flags

### Operators
- `AND` - Both terms must be present
- `OR` - Either term must be present  
- `NOT` - Term must not be present
- `()` - Grouping for complex expressions

### Regex Flags
- `i` - Case insensitive
- `m` - Multi-line mode
- `s` - Dot matches newline

### Query Examples
```bash
# Find errors in connection handling
error AND /failed.*connection/i

# Find user messages excluding tests
"user message" AND NOT test

# Find warnings or errors with timestamps
(warning OR error) AND timestamp

# Find specific error patterns
/^Error:.*\d+/m

# Complex nested query
(("connection failed" OR "timeout") AND error) NOT debug
```

## Development

### Prerequisites

- Rust 1.75 or later
- cargo-nextest (for enhanced testing)
- clippy (for linting)

### Setup

```bash
# Clone the repository
git clone https://github.com/mkusaka/ccmeta.git
cd ccmeta/schema/ccms

# Install development tools
cargo install cargo-nextest --locked
rustup component add clippy

# Build the project
cargo build

# Run tests
cargo nextest run

# Run clippy
cargo clippy -- -D warnings
```

### Running Tests

**Note on Clipboard Tests:** Some tests that verify clipboard functionality are marked with `#[ignore]` as they require clipboard utilities (`pbcopy` on macOS, `xclip`/`xsel` on Linux) that may not be available in CI environments. To run these tests locally:

```bash
# Run all tests including ignored ones
cargo test -- --ignored

# Run only the clipboard tests
cargo test clipboard -- --ignored
```

```bash
# Run all tests with cargo-nextest
cargo nextest run

# Run specific test
cargo nextest run test_name

# Run tests with standard cargo
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench search_benchmark

# Profile with flamegraph (requires profiling feature)
cargo run --release --features profiling -- --profile baseline "query"
```

### Project Structure

```
ccms/
├── src/
│   ├── main.rs                    # CLI entry point
│   ├── lib.rs                     # Library exports
│   ├── interactive_ratatui/       # Interactive TUI mode (Clean Architecture)
│   │   ├── mod.rs                 # Main event loop
│   │   ├── domain/                # Domain layer (models, business rules)
│   │   ├── application/           # Application layer (services)
│   │   └── ui/                    # UI layer (MVU pattern, components)
│   ├── query/                     # Query parsing and evaluation
│   │   ├── parser.rs              # Nom-based query parser
│   │   └── condition.rs           # Query condition types
│   ├── schemas/                   # Claude message schemas
│   │   ├── session_message.rs
│   │   └── tool_result.rs
│   ├── search/                    # Search engine implementation
│   │   ├── engine.rs              # Core search logic
│   │   ├── file_discovery.rs
│   │   └── async_engine.rs
│   └── profiling.rs               # Performance profiling
├── benches/                       # Benchmarks
├── tests/                         # Integration tests
├── CLAUDE.md                      # Guidance for Claude Code
├── spec.md                        # Detailed interactive mode specification
└── PERFORMANCE.md                 # Performance characteristics and benchmarks
```

## Performance

This tool is optimized for maximum performance:

- **SIMD JSON Parsing**: Uses simd-json for hardware-accelerated parsing
- **Parallel Processing**: Leverages all CPU cores with Rayon
- **Zero-Copy Design**: Minimizes allocations and string copies
- **Smart Filtering**: Early termination and efficient predicate evaluation
- **Memory-Mapped I/O**: Efficient handling of large files

## Configuration

### Default Search Location

By default, searches in `~/.claude/projects/**/*.jsonl`

### Custom Patterns

```bash
# Search in specific project
ccms -p "~/.claude/projects/myproject/*.jsonl" "query"

# Search in current directory
ccms -p "$(pwd)/**/*.jsonl" "query"

# Search single file
ccms -p "/path/to/specific/session.jsonl" "query"
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Run tests and ensure they pass (`cargo nextest run`)
4. Run clippy and fix any warnings (`cargo clippy -- -D warnings`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Code Style

- Follow Rust standard style guidelines
- Use `cargo fmt` before committing
- Ensure `cargo clippy` passes with no warnings
- Add tests for new functionality
- Update documentation as needed

## Troubleshooting

### No results found
- Check file permissions on Claude session files
- Verify the search pattern matches existing files
- Use `-v` flag for verbose output to debug file discovery

### Performance issues
- Use `-n` to limit results for large datasets
- Consider using more specific search patterns
- Enable profiling with `--features profiling` to identify bottlenecks

### Interactive mode issues
- Ensure terminal supports ANSI escape codes
- Check that required clipboard utilities are installed (pbcopy/xclip/xsel)
- Try running with `--no-color` if display issues occur

## License

MIT License - see [LICENSE](LICENSE) file for details

## Acknowledgments

- Built with [nom](https://github.com/rust-bakery/nom) for parsing
- Uses [simd-json](https://github.com/simd-lite/simd-json) for fast JSON parsing
- Parallel processing powered by [rayon](https://github.com/rayon-rs/rayon)
- Interactive UI built with [ratatui](https://github.com/ratatui-org/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm)