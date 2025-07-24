# Interactive Search Application Architecture with tui-realm v3

## Overview

This document describes the architecture for migrating the interactive search application to tui-realm v3, following the framework's best practices for unidirectional data flow, component-based design, and proper state management.

## Core Architectural Principles

### 1. Unidirectional Data Flow

```
┌─────────────┐     ┌──────────┐     ┌────────────┐
│   Events    │ ──► │  Update  │ ──► │   State    │
└─────────────┘     └──────────┘     └────────────┘
       ▲                                     │
       │            ┌──────────┐             │
       └──────────  │   View   │ ◄───────────┘
                    └──────────┘
```

- **Events**: User input, async results, system events
- **Update**: Pure function that processes events and returns new state
- **State**: Single source of truth for the entire application
- **View**: Renders components based on current state

### 2. Component-Based Architecture

Components in tui-realm v3 are:
- Self-contained UI elements with their own state
- Communicate via message passing
- Subscribe to specific events
- Update their state through attributes

### 3. State Management Principles

- **No State Duplication**: Each piece of state has a single owner
- **Attribute System**: Components receive data through attributes
- **Message Passing**: Components communicate through messages
- **Event Subscriptions**: Components subscribe to relevant events only

## Application Structure

```
interactive_search/
├── main.rs              # Entry point and event loop
├── app.rs               # Main application struct
├── model.rs             # Application state model
├── update.rs            # Update logic for state transitions
├── view.rs              # View composition and layout
├── messages.rs          # Message definitions
├── components/          # UI components
│   ├── mod.rs
│   ├── search_input.rs  # Search input with debouncing
│   ├── result_list.rs   # Result list with navigation
│   ├── result_detail.rs # Detailed result view
│   ├── session_viewer.rs# Session viewer
│   ├── help_dialog.rs   # Help dialog
│   └── status_bar.rs    # Status information
├── services/            # Business logic services
│   ├── search.rs        # Async search service
│   ├── session.rs       # Session loading service
│   └── clipboard.rs     # Clipboard operations
└── utils/               # Utilities
    ├── debounce.rs      # Debouncing logic
    └── shortcuts.rs     # Keyboard shortcut definitions
```

## Component Hierarchy and Design

### Component Tree

```
Application
├── SearchInput        # Search query input
├── ResultList         # List of search results
├── ResultDetail       # Detailed view of selected result
├── SessionViewer      # Full session viewer
├── HelpDialog         # Help overlay
└── StatusBar          # Status and mode information
```

### Component Definitions

#### SearchInput Component

```rust
// Attributes
- query: String          // Current query text
- is_searching: bool     // Search in progress indicator
- cursor_position: usize // Cursor position for editing

// Internal State
- debounce_timer: Option<Instant>
- pending_query: Option<String>

// Subscriptions
- Key events (all text input, editing shortcuts)
- SearchComplete message

// Emits Messages
- QueryChanged(String)
- SearchRequested(String)
- CursorMoved(usize)
```

#### ResultList Component

```rust
// Attributes
- results: Vec<SearchResult>
- selected_index: usize
- visible_range: Range<usize>
- is_loading: bool

// Internal State
- scroll_offset: usize

// Subscriptions
- Key events (navigation: j/k, Ctrl-n/p, PgUp/PgDn)
- SearchComplete message
- ResultSelected message

// Emits Messages
- ResultSelected(usize)
- ResultNavigated(Direction)
- RequestResultDetail(SearchResult)
```

#### ResultDetail Component

```rust
// Attributes
- result: Option<SearchResult>
- content: String
- scroll_position: usize

// Internal State
- viewport_height: u16
- content_height: usize

// Subscriptions
- Key events (scrolling, copy operations)
- ResultDetailRequested message

// Emits Messages
- CopyToClipboard(String)
- CloseDetail
- ScrollDetail(Direction)
```

#### SessionViewer Component

```rust
// Attributes
- session_messages: Vec<SessionMessage>
- current_message_index: usize
- is_loading: bool

// Internal State
- scroll_offset: usize
- viewport: ViewportState

// Subscriptions
- Key events (navigation, scrolling)
- SessionLoaded message

// Emits Messages
- NavigateMessage(Direction)
- CloseSession
- ScrollSession(Direction)
```

#### HelpDialog Component

```rust
// Attributes
- is_visible: bool
- shortcuts: Vec<ShortcutInfo>
- selected_category: usize

// Internal State
- scroll_position: usize

// Subscriptions
- Key events (Esc, navigation)
- ToggleHelp message

// Emits Messages
- CloseHelp
- NavigateHelp(Direction)
```

## State Model

```rust
// Main application state
struct AppState {
    // Mode
    mode: AppMode,
    
    // Search state
    search: SearchState,
    
    // Results state
    results: ResultsState,
    
    // Session state
    session: SessionState,
    
    // UI state
    ui: UIState,
}

enum AppMode {
    Search,
    ResultDetail,
    SessionViewer,
    Help,
}

struct SearchState {
    query: String,
    is_searching: bool,
    cursor_position: usize,
    search_handle: Option<AsyncHandle>,
}

struct ResultsState {
    items: Vec<SearchResult>,
    selected_index: usize,
    total_count: usize,
    current_detail: Option<SearchResult>,
}

struct SessionState {
    session_id: Option<String>,
    messages: Vec<SessionMessage>,
    current_index: usize,
    is_loading: bool,
}

struct UIState {
    terminal_size: (u16, u16),
    show_help: bool,
    status_message: Option<String>,
    clipboard_content: Option<String>,
}
```

## Message Flow

### Message Definitions

```rust
#[derive(Debug, Clone)]
enum AppMessage {
    // Search messages
    QueryChanged(String),
    SearchRequested(String),
    SearchComplete(Vec<SearchResult>),
    SearchError(String),
    
    // Navigation messages
    ResultSelected(usize),
    NavigateResults(Direction),
    EnterResultDetail,
    ExitResultDetail,
    
    // Session messages
    LoadSession(String),
    SessionLoaded(Vec<SessionMessage>),
    NavigateSession(Direction),
    ExitSession,
    
    // UI messages
    ToggleHelp,
    CopyToClipboard(String),
    ClipboardCopied,
    ShowStatus(String),
    
    // System messages
    Resize(u16, u16),
    Quit,
    Error(String),
}

#[derive(Debug, Clone)]
enum Direction {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}
```

### Message Flow Examples

#### Search Flow
```
1. User types in SearchInput
2. SearchInput emits QueryChanged(query)
3. Update function starts debounce timer
4. After debounce, Update emits SearchRequested(query)
5. Search service receives request, starts async search
6. Search completes, emits SearchComplete(results)
7. Update function updates ResultsState
8. ResultList component re-renders with new results
```

#### Navigation Flow
```
1. User presses 'j' in ResultList
2. ResultList emits NavigateResults(Down)
3. Update function increments selected_index
4. ResultList re-renders with new selection
5. If Enter pressed, emits EnterResultDetail
6. Update function changes mode to ResultDetail
7. ResultDetail component becomes active
```

## Event Subscription Patterns

### Subscription Registry

```rust
impl Application {
    fn register_subscriptions(&mut self) {
        // SearchInput subscribes to text input and editing keys
        self.search_input.subscribe(&[
            EventType::Key(KeyCode::Char(_)),
            EventType::Key(KeyCode::Backspace),
            EventType::Key(KeyCode::Delete),
            EventType::Key(KeyCode::Left),
            EventType::Key(KeyCode::Right),
            EventType::Key(KeyCode::Home),
            EventType::Key(KeyCode::End),
            // Ctrl+A, Ctrl+E, Ctrl+U, Ctrl+K, etc.
        ]);
        
        // ResultList subscribes to navigation keys
        self.result_list.subscribe(&[
            EventType::Key(KeyCode::Char('j')),
            EventType::Key(KeyCode::Char('k')),
            EventType::Key(KeyCode::Up),
            EventType::Key(KeyCode::Down),
            EventType::Key(KeyCode::PageUp),
            EventType::Key(KeyCode::PageDown),
            EventType::Key(KeyCode::Enter),
            // Ctrl+N, Ctrl+P
        ]);
        
        // Global subscriptions handled by Application
        self.subscribe_global(&[
            EventType::Key(KeyCode::Char('?')), // Help
            EventType::Key(KeyCode::Esc),       // Mode exit
            EventType::Key(KeyCode::Char('q')), // Quit
            EventType::Resize,                  // Terminal resize
        ]);
    }
}
```

### Event Routing

```rust
impl Application {
    fn route_event(&mut self, event: Event) -> Option<AppMessage> {
        match self.model.mode {
            AppMode::Search => {
                // Route to active components in search mode
                if let Some(msg) = self.search_input.on_event(event) {
                    return Some(msg);
                }
                if let Some(msg) = self.result_list.on_event(event) {
                    return Some(msg);
                }
            }
            AppMode::ResultDetail => {
                if let Some(msg) = self.result_detail.on_event(event) {
                    return Some(msg);
                }
            }
            AppMode::SessionViewer => {
                if let Some(msg) = self.session_viewer.on_event(event) {
                    return Some(msg);
                }
            }
            AppMode::Help => {
                if let Some(msg) = self.help_dialog.on_event(event) {
                    return Some(msg);
                }
            }
        }
        
        // Handle global events
        self.handle_global_event(event)
    }
}
```

## Attribute Usage Patterns

### Attribute Updates

```rust
impl Application {
    fn update_component_attributes(&mut self) {
        // Update SearchInput attributes
        self.search_input
            .attr(Attribute::Value, AttrValue::String(self.model.search.query.clone()))
            .attr(Attribute::Custom("cursor"), AttrValue::Size(self.model.search.cursor_position))
            .attr(Attribute::Custom("searching"), AttrValue::Flag(self.model.search.is_searching));
        
        // Update ResultList attributes
        self.result_list
            .attr(Attribute::Content, AttrValue::Table(self.format_results()))
            .attr(Attribute::Value, AttrValue::Size(self.model.results.selected_index))
            .attr(Attribute::Custom("loading"), AttrValue::Flag(self.model.search.is_searching));
        
        // Update ResultDetail attributes
        if let Some(ref result) = self.model.results.current_detail {
            self.result_detail
                .attr(Attribute::Text, AttrValue::Text(result.content.clone()))
                .attr(Attribute::Custom("title"), AttrValue::String(result.title.clone()));
        }
        
        // Update SessionViewer attributes
        self.session_viewer
            .attr(Attribute::Content, AttrValue::Table(self.format_session()))
            .attr(Attribute::Value, AttrValue::Size(self.model.session.current_index));
    }
}
```

### Attribute Best Practices

1. **Use Standard Attributes**: Prefer built-in attributes like `Value`, `Text`, `Content`
2. **Custom Attributes**: Use for component-specific data
3. **Avoid Duplication**: Don't store the same data in multiple places
4. **Immutable Updates**: Always create new values, don't mutate
5. **Batch Updates**: Update all attributes at once for efficiency

## Async Operations

### Search Service Integration

```rust
struct SearchService {
    tx: mpsc::Sender<AppMessage>,
}

impl SearchService {
    async fn search(&self, query: String) {
        let results = match self.perform_search(&query).await {
            Ok(results) => results,
            Err(e) => {
                let _ = self.tx.send(AppMessage::SearchError(e.to_string())).await;
                return;
            }
        };
        
        let _ = self.tx.send(AppMessage::SearchComplete(results)).await;
    }
}
```

### Debouncing Implementation

```rust
struct Debouncer {
    delay: Duration,
    pending: Option<(Instant, String)>,
}

impl Debouncer {
    fn trigger(&mut self, value: String) -> Option<String> {
        self.pending = Some((Instant::now(), value));
        None
    }
    
    fn poll(&mut self) -> Option<String> {
        if let Some((time, value)) = &self.pending {
            if time.elapsed() >= self.delay {
                return self.pending.take().map(|(_, v)| v);
            }
        }
        None
    }
}
```

## Keyboard Shortcuts Preservation

### Readline/Emacs Shortcuts

```rust
fn handle_search_input_shortcuts(key: KeyEvent) -> Option<AppMessage> {
    match key {
        // Movement
        KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::CursorHome)
        }
        KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::CursorEnd)
        }
        KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::CursorForward)
        }
        KeyEvent { code: KeyCode::Char('b'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::CursorBackward)
        }
        
        // Deletion
        KeyEvent { code: KeyCode::Char('u'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::DeleteToStart)
        }
        KeyEvent { code: KeyCode::Char('k'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::DeleteToEnd)
        }
        KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::DeleteWord)
        }
        KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::CONTROL, .. } => {
            Some(AppMessage::DeleteChar)
        }
        
        _ => None,
    }
}
```

## Separation of Concerns

### Layer Responsibilities

1. **Components**: UI rendering and local state
2. **Model**: Application state and business data
3. **Update**: State transitions and business logic
4. **Services**: External operations (search, clipboard, file I/O)
5. **View**: Layout composition and component arrangement

### Data Flow Rules

1. Components never directly modify application state
2. All state changes go through the Update function
3. Services communicate via messages only
4. Components receive data through attributes
5. No component-to-component direct communication

## Migration Strategy

### Phase 1: Core Structure
1. Set up tui-realm v3 application skeleton
2. Define message types and state model
3. Implement basic update function

### Phase 2: Component Migration
1. Migrate SearchInput with shortcuts
2. Migrate ResultList with navigation
3. Migrate ResultDetail and SessionViewer
4. Add HelpDialog and StatusBar

### Phase 3: Services Integration
1. Port search service with async support
2. Implement debouncing
3. Add clipboard operations
4. Session loading service

### Phase 4: Polish
1. Optimize rendering performance
2. Add comprehensive error handling
3. Implement all keyboard shortcuts
4. Testing and refinement

## Performance Considerations

1. **Virtualized Lists**: Only render visible items
2. **Debounced Search**: Avoid excessive API calls
3. **Lazy Loading**: Load session content on demand
4. **Attribute Diffing**: Only update changed attributes
5. **Event Filtering**: Components only receive relevant events

## Testing Strategy

1. **Component Tests**: Test each component in isolation
2. **Update Tests**: Verify state transitions
3. **Integration Tests**: Test message flow
4. **Service Tests**: Mock async operations
5. **Shortcut Tests**: Verify all keyboard combinations