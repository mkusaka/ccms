# CCMS Interactive Mode Specification

## Overview

The interactive mode provides a terminal-based user interface for searching Claude session messages in real-time. It uses the `ratatui` crate with crossterm backend for terminal control and implements features like incremental search, result navigation, role filtering, and clipboard operations.

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
| ↓ (Arrow Down) | Move selection down (with scrolling support) |
| Enter | View full details of selected result |
| Home | Jump to first result |
| End | Jump to last result |
| PageUp | Scroll up by visible height |
| PageDown | Scroll down by visible height |
| ? | Show help screen |
| Tab | Cycle through role filters: None → user → assistant → system → summary → None |
| Ctrl+R | Clear cache and reload all files |
| Ctrl+T | Toggle message truncation (Truncated/Full Text) |
| Ctrl+C (2x) | Exit interactive mode (press twice within 1 second) |

### Full Result View

When Enter is pressed on a result, a detailed view is shown:

```
────────────────────────────────────────────────────────────────────────────────
Role: [role]
Time: YYYY-MM-DD HH:MM:SS
File: [filename - automatically wraps if too long for terminal width]
Project: [project path - automatically wraps if too long for terminal width]
UUID: [uuid]
Session: [session_id]
────────────────────────────────────────────────────────────────────────────────
[Full message content with automatic word wrapping at terminal boundaries]
[Long lines are wrapped at word boundaries when possible]
[Unicode characters (Japanese, emoji) are safely handled]
[Scrollable with j/k or ↑/↓ arrows]
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
  [Esc] - Return to search results
```

**Message Display**: Messages are automatically displayed with word wrapping in the detail view, ensuring full readability without horizontal scrolling. Long lines wrap at word boundaries when possible, with proper Unicode character handling.

#### Scrolling Behavior

- Long messages can be scrolled using j/k or arrow keys
- Page up/down scrolls by 10 lines
- Scroll offset is reset when returning to search view
- Visible area adjusts based on terminal height

### Session Viewer

When 'S' is pressed in the full result view:

```
┌─ Session Viewer ──────────────────────────────────────────────────────────────┐
│ Session: [session_id]                                                          │
│ File: [filename]                                                              │
└────────────────────────────────────────────────────────────────────────────────┘
┌─ Search ───────────────────────────────────────────────────────────────────────┐
│ Filter: [query]                                                                │
└────────────────────────────────────────────────────────────────────────────────┘
┌─ Messages (N total[, M filtered]) ─────────────────────────────────────────────┐
│  1. [ROLE     ] MM/DD HH:MM Preview text of message...                        │
│> 2. [ROLE     ] MM/DD HH:MM Preview text of selected message...               │
│  3. [ROLE     ] MM/DD HH:MM Preview text of another message...                │
│  ...                                                                           │
│                                                                                │
│ Showing X-Y of Z messages ↑/↓ to scroll                                        │
└────────────────────────────────────────────────────────────────────────────────┘
Enter: View | ↑/↓: Navigate | /: Search | I: Copy Session ID | O: Sort | C: Copy All | Esc: Back
```

**Navigation**: Pressing Esc returns to the previous screen (typically ResultDetail), not directly to Search.

#### Session Viewer Features

1. **List View Display**:
   - Shows all messages in a scrollable list format
   - Each message displays: index, role (centered), timestamp, and preview text
   - Selected message is highlighted with ">" indicator and different background

2. **Interactive Search**:
   - Type to filter messages in real-time (no need to press '/')
   - Case-insensitive search across message content
   - Shows filtered count: "Messages (123 total, 45 filtered)"
   - Backspace to delete characters, Esc to clear search
   - **Search result highlighting**: Matched text is highlighted in message previews

3. **Navigation and Actions**:
   - ↑/↓: Move selection through messages
   - PageUp/PageDown: Jump 10 messages at a time
   - Enter: View full message in detail view
   - /: Start search mode (interactive filtering)
   - O: Toggle sort order (Ascending/Descending/Original)
   - C: Copy selected message
   - Shift+C: Copy all visible (filtered) messages
   - I: Copy session ID
   - Esc/Backspace: Return to previous screen
   - Maintains scroll position and selection state

4. **Message Content Search**:
   - Searches in both simple text content and array-based content
   - Handles various message structures:
     - Direct: `{"content": "text"}`
     - Nested: `{"message": {"content": "text"}}`
     - Array: `{"content": [{"type": "text", "text": "content"}]}`

## Search Functionality

### Query Processing

1. Queries are parsed using the query parser supporting:
   - Literal text search (case-insensitive)
   - Boolean operators: AND, OR, NOT
   - Parentheses for grouping
   - Regular expressions: `/pattern/flags`
   - Quoted strings: "multi word search" or 'single quoted'

2. Empty queries show all available results (no filtering applied)

3. Invalid queries (parse errors) return empty result sets

### Result Formatting

#### Result Line Format (List View)

```
[index]. [ROLE]    MM/DD HH:MM Preview...
```

- Index: 1-based numbering
- Role: Uppercase, displayed in yellow
- Timestamp: Formatted as MM/DD HH:MM
- Preview: Dynamically truncated to fit terminal width with ellipsis (...) when needed
  - Calculates available width based on terminal size
  - Preserves multibyte character boundaries
  - Newlines replaced by spaces

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

## Search Behavior

### Immediate Search Execution

- Search executes immediately on every keystroke (no debouncing)
- Empty queries show all available results (unfiltered)
- Each character input or backspace triggers a new search
- Search state indicator shows "searching..." during execution
- Initial load automatically searches with empty query to display all messages

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
- **Context-aware feedback** shows what was copied:
  - File paths: "✓ Copied file path"
  - Session IDs (UUID format): "✓ Copied session ID"
  - Short text (< 100 chars): "✓ Copied: [actual text]"
  - Long text: "✓ Copied message text"
  - Full result details: "✓ Copied full result details"

## Display Limits

### Result Display

- Default max_results: 50 (configurable via CLI)
- Maximum visible results in list view: dynamically calculated based on terminal height
- Results list supports scrolling with:
  - ↑/↓: Move selection one item
  - Home: Jump to first result
  - End: Jump to last result
  - PageUp: Move up by visible height
  - PageDown: Move down by visible height
- Total result count displayed
- Indication when more results exist beyond display limit
- Indication when max_results limit is reached

### Multibyte Character Handling

- Preview text truncation respects character boundaries
- Uses character-based operations (not byte-based) for:
  - Preview generation (dynamic width based on terminal)
  - Cursor positioning with role filters
  - Text display in all views
- Prevents crashes with Unicode text (Japanese, emoji, etc.)
- Dynamic ellipsis placement based on available terminal width

### Message Truncation Toggle

The Ctrl+T keyboard shortcut toggles between truncated and full text display modes in the search view:

#### Truncated Mode (Default)
- Messages are truncated to fit the terminal width
- Ellipsis (...) added when text is cut off
- Provides better overview of multiple results
- Applies to:
  - Search results list (single line with ellipsis)
  - Session viewer messages

#### Full Text Mode
- Messages are wrapped at word boundaries to fit terminal width
- Long words that exceed terminal width are broken at character boundaries
- Preserves readability while showing complete content
- Respects Unicode character boundaries (safe for Japanese text and emojis)
- Applies to:
  - Search results list (multi-line with word wrapping)
  - Session viewer messages (wrapped display)

#### Visual Indicators
- Status bar shows current mode: `[Truncated]` or `[Full Text]`
- Mode persists across search and session viewer
- Feedback message shown when toggling

Note: The Result Detail view always displays messages with word wrapping and is not affected by the truncation toggle.

### Session Viewer Display Limits

- Shows all messages in the session file in a scrollable list
- No longer uses 3-message pagination (replaced by continuous scrolling)
- Default order: Ascending (chronological)
- List view dynamically adjusts to terminal height
- Scroll indicators show position: "Showing X-Y of Z messages"
- Message preview dynamically truncated based on terminal width
- Filtered view shows subset count: "Messages (123 total, 45 filtered)"

## Exit Behavior

On exit (Ctrl+C pressed twice within 1 second from Search screen, or 'q' key):
1. Clears search area from screen
2. Displays "Goodbye!" message
3. Returns control to terminal

**Note**: Exit behavior:
- From Search screen: Press Ctrl+C twice within 1 second to exit
- From other screens: Esc returns to the previous screen in the navigation stack
- The Esc key no longer exits the application from the Search screen

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

The interactive mode uses a Model-View-Update (MVU) architecture with clean separation of concerns:

### Architecture Layers

1. **Domain Layer**: Core business entities and models
   - `Mode`: Current UI screen (Search, ResultDetail, SessionViewer, Help)
   - `SearchRequest`: Query and filter parameters
   - `SessionOrder`: Sort order for session messages

2. **Application Layer**: Business logic and services
   - `SearchService`: Handles search operations
   - `SessionService`: Manages session message loading
   - `CacheService`: File caching and invalidation

3. **UI Layer**: Presentation with MVU pattern
   - `AppState`: Centralized state management
   - `Message`: Events and user actions
   - `Command`: Side effects (search, load, copy)
   - `Renderer`: Component-based UI rendering

### Core State Structure

```rust
struct AppState {
    mode: Mode,                        // Current screen
    mode_stack: Vec<Mode>,            // Navigation history
    search: SearchState,              // Search-related state
    session: SessionState,            // Session viewer state
    ui: UIState,                      // UI-specific state
}

struct SearchState {
    query: String,                    // Current search query
    role_filter: Option<String>,      // Active role filter
    results: Vec<SearchResult>,       // Search results
    selected_index: usize,            // Selected result
    scroll_offset: usize,             // Scroll position
    is_searching: bool,               // Search in progress
    current_search_id: u64,           // Request tracking
}

struct SessionState {
    messages: Vec<String>,            // Raw JSONL messages
    filtered_indices: Vec<usize>,     // Filtered indices
    selected_index: usize,            // Selected message
    scroll_offset: usize,             // Scroll position
    query: String,                    // Search filter
    order: Option<SessionOrder>,      // Sort order
    file_path: Option<String>,        // Session file path
    session_id: Option<String>,       // Session identifier
}

struct UIState {
    truncation_enabled: bool,         // Message truncation mode
    detail_scroll_offset: usize,      // Detail view scroll
    selected_result: Option<SearchResult>, // Current result
    message: Option<String>,          // Feedback message
}
```

### Navigation Stack

The interactive mode maintains a navigation history stack that allows users to return to the previous screen:

- `screen_stack: Vec<Mode>` stores the navigation history
- `push_screen(mode)` navigates to a new screen
- `pop_screen()` returns to the previous screen
- Always maintains at least one screen (Search) in the stack

### Mode Transitions

- Search → ResultDetail: Enter key on result (pushes to stack)
- ResultDetail → Search: Esc or other keys (pops from stack, clears message and scroll offset)
- ResultDetail → SessionViewer: S key (pushes to stack)
- SessionViewer → ResultDetail: Q/Esc (pops from stack, returns to previous screen)
- Any → Help: ? key (pushes to stack)
- Help → Previous Screen: Any key (pops from stack)

**Important**: Esc/Q always returns to the previous screen in the navigation history, not directly to Search. This provides a more intuitive navigation experience when moving through multiple screens.

### Session Viewer State Management

When entering SessionViewer:
- Loads all messages from the session file
- Sets default order to Ascending
- Initializes filtered indices to show all messages
- Clears search query
- Resets scroll position and selection

When exiting SessionViewer:
- Clears all session-related state
- Returns to ResultDetail mode
- Preserves the selected result for continued navigation

## Project Path Extraction

Project paths are extracted from file paths using the pattern:
`~/.claude/projects/{encoded-project-path}/{session-id}.jsonl`

The encoded project path has slashes replaced with hyphens, which are decoded during extraction.

## Recent Architecture Improvements

### Clean Architecture Migration (2024)

The interactive mode was refactored from a monolithic 1882-line file to a clean architecture:

- **Before**: Single `InteractiveSearch` struct with 30+ fields (God Object anti-pattern)
- **After**: Layered architecture with clear separation of concerns

Key improvements:
1. **Domain Layer**: Pure business logic and models
2. **Application Layer**: Service orchestration without UI concerns
3. **UI Layer**: MVU pattern for predictable state management
4. **Component System**: Reusable UI components with trait-based design
5. **Comprehensive Testing**: Tests organized by architectural layers

### Enhanced Features

Recent enhancements include:
- Navigation history stack for intuitive back navigation
- Context-aware copy feedback messages
- Default full text display with proper word wrapping
- Empty query shows all results (no filtering)
- Session viewer metadata display and session ID copying
- Non-blocking UI with visual feedback during operations
- Unicode-safe text handling throughout
- Search result highlighting in Session Viewer
- Automatic text wrapping for long file paths in Result Detail and Session Viewer
- Unified exit mechanism (Ctrl+C twice) - ESC no longer exits from search screen