# CCMS Interactive Mode Specification

## Overview

The interactive mode provides a terminal-based user interface for searching Claude session messages in real-time. It uses the `console` crate for terminal control and implements features like incremental search, result navigation, role filtering, and clipboard operations.

## User Interface Layout

### Initial Screen

```
Interactive Claude Search
Type to search, ↑/↓ to navigate, Enter to select, Tab for role filter, Ctrl+R to reload, Esc/Ctrl+C to exit

Search: [cursor]
```

### Search Results Display

When a query is entered, the interface shows:

```
Interactive Claude Search
Type to search, ↑/↓ to navigate, Enter to select, Tab for role filter, Ctrl+R to reload, Esc/Ctrl+C to exit

Search: [query]
Found N results (limit reached if applicable)

> 1. [ROLE]    MM/DD HH:MM Preview text up to 40 chars...
  2. [ROLE]    MM/DD HH:MM Preview text up to 40 chars...
  3. [ROLE]    MM/DD HH:MM Preview text up to 40 chars...
  ...
  10. [ROLE]   MM/DD HH:MM Preview text up to 40 chars...

... and X more results
```

### Role Filter Display

When a role filter is active:

```
Search [role]: [query]
```

## Key Bindings

### Main Search Interface

| Key | Action |
|-----|--------|
| Any character | Append to search query and execute search |
| Backspace | Remove last character from query and re-search |
| ↑ (Arrow Up) | Move selection up (with bounds checking) |
| ↓ (Arrow Down) | Move selection down (limited to visible results, max 10) |
| Enter | View full details of selected result |
| Tab | Cycle through role filters: None → user → assistant → system → summary → None |
| Ctrl+R | Clear cache and reload all files |
| Esc or Ctrl+C | Exit interactive mode |

### Full Result View

When Enter is pressed on a result, a detailed view is shown:

```
────────────────────────────────────────────────────────────────────────────────
Role: [role]
Time: YYYY-MM-DD HH:MM:SS
File: [filename]
Project: [project path]
UUID: [uuid]
Session: [session_id]
────────────────────────────────────────────────────────────────────────────────
[Full message content - scrollable with j/k or ↑/↓ arrows]
────────────────────────────────────────────────────────────────────────────────

Actions:
  [S] - View full session
  [F] - Copy file path
  [I] - Copy session ID
  [P] - Copy project path
  [M] - Copy message text
  [R] - Copy raw JSON
  [J/↓] - Scroll down
  [K/↑] - Scroll up
  [PageDown] - Scroll down 10 lines
  [PageUp] - Scroll up 10 lines
  [Any other key] - Continue
```

#### Scrolling Behavior

- Long messages can be scrolled using j/k or arrow keys
- Page up/down scrolls by 10 lines
- Scroll offset is reset when returning to search view
- Visible area adjusts based on terminal height

### Session Viewer

When 'S' is pressed in the full result view:

```
Session Viewer
Session: [session_id]
File: [filename]

[A]scending / [D]escending / [Q]uit

[After choosing order]
────────────────────────────────────────────────────────────────────────────────
Message 1/N
Role: [role]
Time: [timestamp]

[message content]
────────────────────────────────────────────────────────────────────────────────
[Additional messages...]

Press any key to continue, Q to quit... [shown every 3 messages]
```

## Search Functionality

### Query Processing

1. Queries are parsed using the query parser supporting:
   - Literal text search (case-insensitive)
   - Boolean operators: AND, OR, NOT
   - Parentheses for grouping
   - Regular expressions: `/pattern/flags`
   - Quoted strings: "multi word search" or 'single quoted'

2. Empty queries return no results

3. Invalid queries (parse errors) return empty result sets

### Result Formatting

#### Result Line Format (List View)

```
[index]. [ROLE]    MM/DD HH:MM Preview...
```

- Index: 1-based numbering
- Role: Uppercase, displayed in yellow
- Timestamp: Formatted as MM/DD HH:MM
- Preview: First 40 characters with newlines replaced by spaces

#### Timestamp Handling

- Input: RFC3339 format (e.g., "2024-01-01T12:00:00Z")
- List display: MM/DD HH:MM
- Full display: YYYY-MM-DD HH:MM:SS

## Caching System

### Cache Structure

The system maintains a `MessageCache` that stores:

```rust
struct CachedFile {
    messages: Vec<SessionMessage>,    // Parsed messages
    raw_lines: Vec<String>,          // Original JSONL lines
    last_modified: SystemTime,       // File modification time
}
```

### Cache Behavior

1. **Automatic Loading**: Files are loaded and cached on first access
2. **Change Detection**: Files are reloaded if modification time changes
3. **Manual Reload**: Ctrl+R clears entire cache forcing reload
4. **Performance**: Uses 32KB buffer for file reading

### File Discovery

Files are discovered using:
- Single file if provided path is a file
- Pattern matching for directories using `discover_claude_files`
- Tilde expansion for home directory paths

## Filtering System

### Role Filter

Cycles through: None → user → assistant → system → summary → None

Applied before other filters in the search pipeline.

### Base Options Filters

1. **Session ID**: Filters messages by session_id field
2. **Timestamp Filters**:
   - `before`: RFC3339 timestamp - excludes messages after this time
   - `after`: RFC3339 timestamp - excludes messages before this time

### Filter Application Order

1. Query condition evaluation
2. Role filter (if active)
3. Session ID filter (if specified)
4. Timestamp filters (if specified)
5. Sort by timestamp (newest first)
6. Limit to max_results

## Query Consistency

To prevent issues with keyboard buffering during search:

1. `execute_search` returns both results and the query string used
2. Results are only updated if the current query matches the search query
3. This prevents stale results when users type quickly

## Clipboard Operations

### Platform-Specific Commands

- **macOS**: `pbcopy`
- **Linux**: `xclip -selection clipboard` (fallback to `xsel --clipboard --input`)
- **Windows**: `clip`

### Copyable Fields

- File path (F)
- Session ID (I)
- Project path (P)
- Message text (M)
- Raw JSON (R) - if available

### Copy Feedback

- Success messages show with "✓" symbol in green
- Warning messages show with "⚠" symbol in yellow
- Feedback remains visible in detail view (does not return to search)
- Messages are cleared when transitioning between modes

## Display Limits

### Result Display

- Default max_results: 50 (configurable via CLI)
- Maximum visible results in list view: dynamically calculated based on terminal height
- Total result count displayed
- Indication when more results exist beyond display limit
- Indication when max_results limit is reached

### Multibyte Character Handling

- Preview text truncation respects character boundaries
- Uses character-based operations (not byte-based) for:
  - Preview generation (40 characters max)
  - Cursor positioning with role filters
  - Text display in all views
- Prevents crashes with Unicode text (Japanese, emoji, etc.)

### Session Viewer

- Shows all messages in the session file
- Pauses every 3 messages for readability
- Supports ascending/descending order
- Handles multiple JSON formats:
  - Direct content: `{"content": "text"}`
  - Nested message content: `{"message": {"content": "text"}}`
  - Array content: `{"message": {"content": [{"type": "text", "text": "content"}]}}`
- Properly resolves file paths from search results
- Displays formatted messages with role, timestamp, and content

## Exit Behavior

On exit (Esc or Ctrl+C):
1. Clears search area from screen
2. Displays "Goodbye!" message
3. Returns control to terminal

## Error Handling

### Graceful Degradation

- Invalid JSON lines are skipped silently
- File read errors are propagated
- Parse errors return empty results
- Missing clipboard commands show error message

### File Processing

- Empty files return empty results
- Mixed valid/invalid JSON processes valid lines only
- Empty lines in files are skipped

## Terminal Control

### Cursor Management

- Cursor positioned at end of search prompt during input
- Result area cleared and redrawn on each update
- Screen cleared for full result display
- Proper restoration after viewing sessions

### Color Scheme

- Headers: Cyan
- Role indicators: Yellow
- Dimmed text: Gray (timestamps, previews, instructions)
- Success messages: Green
- Warnings: Yellow
- Selected item: Bold with cyan ">" indicator

## Performance Characteristics

### Search Execution

- Triggered on every keystroke
- Uses cached data to avoid file I/O
- Parallel file processing via Rayon
- SIMD-accelerated JSON parsing

### Memory Usage

- Entire file contents cached in memory
- Raw JSON lines preserved for clipboard operations
- LRU cache for compiled regex patterns

## State Management

The `InteractiveSearch` struct maintains:

```rust
struct InteractiveSearch {
    base_options: SearchOptions,     // Filters and configuration
    max_results: usize,             // Result limit
    cache: MessageCache,            // File cache
    mode: Mode,                     // Current UI mode
    query: String,                  // Current search query
    role_filter: Option<String>,    // Active role filter
    results: Vec<SearchResult>,     // Current search results
    selected_index: usize,          // Selected result index
    selected_result: Option<SearchResult>, // Detail view result
    session_messages: Vec<String>,  // Session viewer messages
    session_order: Option<SessionOrder>, // Session display order
    detail_scroll_offset: usize,    // Scroll position in detail view
    message: Option<String>,        // Feedback message
}
```

### Mode Transitions

- Search → ResultDetail: Enter key on result
- ResultDetail → Search: Esc or other keys (clears message and scroll offset)
- ResultDetail → SessionViewer: S key
- SessionViewer → Search: Q or completion
- Any → Help: ? key in search mode
- Help → Search: Any key

## Project Path Extraction

Project paths are extracted from file paths using the pattern:
`~/.claude/projects/{encoded-project-path}/{session-id}.jsonl`

The encoded project path has slashes replaced with hyphens, which are decoded during extraction.