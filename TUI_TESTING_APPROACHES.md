# TUI Testing Approaches for claude-search Interactive Module

## Overview
Testing Terminal User Interface (TUI) functionality in Rust presents unique challenges due to the interactive nature of terminal applications. This document outlines various approaches suitable for testing the `interactive.rs` module.

## Current Architecture Analysis

The interactive module uses:
- `console` crate for terminal operations
- Direct keyboard input handling via `term.read_key()`
- Terminal cursor manipulation and screen clearing
- Real-time search execution on keystroke
- Multiple UI states (search, result display, session viewer)

## Testing Approaches

### 1. Mock-based Testing with Terminal Abstraction

**Approach**: Create an abstraction layer over terminal operations that can be mocked during tests.

```rust
// Example trait for terminal operations
trait TerminalInterface {
    fn read_key(&self) -> Result<Key>;
    fn move_cursor_to(&mut self, x: usize, y: usize) -> Result<()>;
    fn clear_screen(&mut self) -> Result<()>;
    // ... other operations
}

// Production implementation
struct ConsoleTerminal(Term);

// Test implementation
struct MockTerminal {
    key_queue: VecDeque<Key>,
    screen_buffer: Vec<String>,
    cursor_position: (usize, usize),
}
```

**Pros:**
- Full control over input/output
- Deterministic test execution
- No external dependencies

**Cons:**
- Requires significant refactoring of existing code
- May not catch platform-specific issues

### 2. Integration Testing with PTY (Pseudo-Terminal)

**Approach**: Use a pseudo-terminal to run the application and interact with it programmatically.

```toml
[dev-dependencies]
portable-pty = "0.8"
```

```rust
#[test]
fn test_interactive_search_flow() {
    let pty_system = portable_pty::native_pty_system();
    let (master, slave) = pty_system.openpty(PtySize::default()).unwrap();
    
    // Spawn the application in the PTY
    let mut child = slave.spawn_command(/* ... */).unwrap();
    
    // Send keystrokes
    master.write_all(b"hello").unwrap();
    master.write_all(&[Key::Enter as u8]).unwrap();
    
    // Read and verify output
    let mut output = String::new();
    master.read_to_string(&mut output).unwrap();
    assert!(output.contains("Search:"));
}
```

**Pros:**
- Tests actual terminal behavior
- Can test platform-specific features
- No code refactoring needed

**Cons:**
- More complex test setup
- Slower test execution
- May have platform compatibility issues

### 3. Snapshot Testing with insta

**Approach**: Capture terminal output and compare against saved snapshots.

```toml
[dev-dependencies]
insta = { version = "1.34", features = ["glob"] }
```

```rust
#[test]
fn test_search_results_display() {
    let output = capture_terminal_output(|| {
        // Run interactive search with predefined inputs
    });
    
    insta::assert_snapshot!(output);
}
```

**Pros:**
- Easy to verify visual output
- Catches unintended UI changes
- Good for regression testing

**Cons:**
- Requires manual snapshot review
- May be sensitive to minor formatting changes
- Doesn't test interactivity directly

### 4. State Machine Testing

**Approach**: Model the TUI as a state machine and test state transitions.

```rust
#[derive(Debug, PartialEq)]
enum InteractiveState {
    Searching { query: String, results: Vec<SearchResult> },
    ViewingResult { result: SearchResult },
    ViewingSession { session_id: String },
}

fn handle_input(state: InteractiveState, input: Key) -> InteractiveState {
    // State transition logic
}

#[test]
fn test_state_transitions() {
    let initial = InteractiveState::Searching { query: "".into(), results: vec![] };
    let next = handle_input(initial, Key::Char('h'));
    // Assert expected state
}
```

**Pros:**
- Tests core logic without UI concerns
- Fast and deterministic
- Easy to test edge cases

**Cons:**
- Doesn't test actual terminal rendering
- Requires separating logic from UI

### 5. Headless Testing with Virtual Terminal

**Approach**: Use a virtual terminal buffer to capture output without actual terminal.

```toml
[dev-dependencies]
termwiz = "0.22"
```

```rust
use termwiz::caps::Capabilities;
use termwiz::terminal::{buffered::BufferedTerminal, Terminal};

#[test]
fn test_interactive_display() {
    let caps = Capabilities::new_from_env().unwrap();
    let mut terminal = BufferedTerminal::new(80, 24, 100, 100, caps).unwrap();
    
    // Run TUI operations
    interactive_search.render(&mut terminal).unwrap();
    
    // Verify buffer contents
    let screen = terminal.screen();
    assert!(screen.line_text(0).contains("Interactive Claude Search"));
}
```

**Pros:**
- Tests actual rendering logic
- No physical terminal needed
- Can inspect exact output

**Cons:**
- May require adapting code to work with different terminal types
- Limited to testing output, not input handling

## Recommended Approach for claude-search

Given the current architecture and the complexity of refactoring, I recommend a **hybrid approach**:

1. **Unit tests** for core logic (search execution, result formatting)
2. **Integration tests** using PTY for critical user flows
3. **Snapshot tests** for UI consistency
4. **Manual testing** for platform-specific features (clipboard operations)

### Implementation Plan

1. Extract testable components:
   ```rust
   // Extract formatting logic
   pub fn format_search_result(result: &SearchResult) -> String { /* ... */ }
   
   // Extract state management
   pub struct InteractiveState {
       query: String,
       results: Vec<SearchResult>,
       selected_index: usize,
       role_filter: Option<String>,
   }
   ```

2. Add integration tests for key flows:
   - Search and select result
   - Role filtering with Tab
   - Navigation with arrow keys
   - Session viewing

3. Add snapshot tests for output formatting:
   - Search result display
   - Full result view
   - Session viewer output

4. Document manual test cases:
   - Clipboard operations on different platforms
   - Terminal resize handling
   - Color output in different terminals

## Example Test Implementation

```rust
// src/interactive/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_format_result_line() {
        let result = SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "123".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "session1".to_string(),
            role: "user".to_string(),
            text: "This is a long message that should be truncated".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: QueryCondition::Literal { 
                pattern: "test".to_string(), 
                case_sensitive: false 
            },
            project_path: "/test/project".to_string(),
        };
        
        let search = InteractiveSearch::new(SearchOptions::default());
        let formatted = search.format_result_line(&result, 0);
        
        assert!(formatted.contains("1."));
        assert!(formatted.contains("[USER]"));
        assert!(formatted.contains("01/01"));
        assert!(formatted.len() < 80); // Should fit in terminal
    }
    
    // More tests...
}
```

## Conclusion

TUI testing requires a multi-faceted approach. While complete end-to-end testing of terminal interactions is challenging, a combination of unit tests for logic, integration tests for critical paths, and snapshot tests for UI consistency can provide good coverage. The key is to structure the code to separate business logic from terminal operations where possible.