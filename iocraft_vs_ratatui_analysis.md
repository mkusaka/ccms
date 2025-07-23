# IoC raft vs Ratatui Implementation Analysis

## Architecture Differences

### 1. Component Architecture

**Ratatui Implementation:**
- Uses a trait-based component system with `Component` trait
- Each component implements `render()` and `handle_key()` methods
- Components maintain their own state internally
- Centralized rendering through `Renderer` class
- Clear separation between components (SearchBar, ResultList, ResultDetail, SessionViewer, HelpDialog)

**IoC raft Implementation:**
- Uses functional components with `#[component]` macro
- Component state is managed externally via props
- No direct keyboard handling in components - handled at App level
- Single file per view (search_view.rs, result_detail_view.rs, etc.)
- More React-like with declarative UI using `element!` macro

### 2. State Management

**Ratatui:**
- Centralized state in `AppState` with sub-states (SearchState, UIState, SessionState)
- MVU (Model-View-Update) pattern with Messages and Commands
- State updates through `update()` method returning Commands for side effects

**IoC raft:**
- State split into smaller `use_state` hooks (SearchState, DetailState, SessionState, UIState)
- Direct state mutation through `.write()` and `.read()` methods
- No explicit command/message pattern - side effects handled inline

### 3. Text Handling and Multibyte Characters

**Ratatui SearchBar:**
```rust
// Proper multibyte character handling
let char_pos = self.cursor_position;
let byte_pos = self.query.chars()
    .take(char_pos)
    .map(|c| c.len_utf8())
    .sum::<usize>();
self.query.insert(byte_pos, c);
```

**IoC raft SearchView:**
```rust
// Simple string push - no cursor position handling
KeyCode::Char(c) => {
    search_state.write().query.push(c);
    perform_search(search_state, search_service, pattern);
}
```

**Key Difference:** IoC raft implementation lacks proper cursor position management and multibyte character boundary handling.

### 4. Text Truncation

**Ratatui ResultList:**
```rust
pub fn truncate_message(text: &str, max_width: usize) -> String {
    let text = text.replace('\n', " ");
    let chars: Vec<char> = text.chars().collect();
    
    if chars.len() <= max_width {
        text
    } else {
        let truncated: String = chars.into_iter().take(max_width - 3).collect();
        format!("{truncated}...")
    }
}
```

**IoC raft SearchView:**
```rust
let preview = if props.ui_state.truncation_enabled {
    let text = result.text.chars().take(80).collect::<String>()
        .replace('\n', " ");
    if result.text.len() > 80 {
        format!("{text}...")
    } else {
        text
    }
}
```

**Key Difference:** Ratatui properly handles character-based truncation, while IoC raft uses byte length check (`result.text.len()`) which could break on multibyte characters.

### 5. Scroll Handling

**Ratatui ResultList:**
- Complex scroll calculation considering wrapped text in full-text mode
- `calculate_visible_range()` and `adjust_scroll_offset()` methods
- Handles both truncated and full-text modes differently

**IoC raft SearchView:**
- Simple offset-based scrolling: `.skip(props.search_state.scroll_offset).take(10)`
- No consideration for wrapped text or dynamic height calculation

### 6. Missing Features in IoC raft

1. **No SearchBar component** - search input is inline in SearchView
2. **No cursor movement** - can't move cursor with arrow keys, Home, End
3. **No mid-string editing** - can't insert/delete characters in middle of query
4. **No proper multibyte handling** - could break on Japanese/emoji input
5. **No wrapped text support** in full-text mode
6. **No dynamic height calculation** for result items
7. **No Component trait** - less modular architecture
8. **No proper text width calculation** - uses byte length instead of character count

### 7. Keyboard Event Handling

**Ratatui:**
- Components handle their own keyboard events
- Returns `Option<Message>` for state changes
- Proper handling of modifiers, special keys

**IoC raft:**
- All keyboard handling centralized in mod.rs
- Direct state mutation
- Less modular, harder to test individual components

## Critical Issues in IoC raft Implementation

1. **Multibyte Character Safety:**
   - `query.push(c)` without cursor position tracking
   - `result.text.len()` for length checks (should use `.chars().count()`)
   - No byte-boundary aware string manipulation

2. **Missing Search Input Features:**
   - No cursor position tracking
   - No ability to edit middle of string
   - No Delete key support
   - No Home/End key support

3. **Scroll Calculation:**
   - No consideration for terminal height
   - Fixed 10-item display regardless of available space
   - No wrapped text support in full-text mode

4. **Component Modularity:**
   - Keyboard handling mixed with business logic
   - Components can't be tested in isolation
   - No clear component boundaries

## Recommendations

1. Implement proper SearchBar component with cursor tracking
2. Add multibyte-safe string manipulation utilities
3. Implement proper scroll calculation based on terminal dimensions
4. Add Component trait or similar abstraction for modularity
5. Separate keyboard handling from business logic
6. Add comprehensive tests for multibyte character handling
7. Implement text wrapping for full-text mode
8. Add proper width calculation using character counts