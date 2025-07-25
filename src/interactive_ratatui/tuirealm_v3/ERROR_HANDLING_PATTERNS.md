# Error Handling Patterns for tui-realm v3

This document describes the consistent error handling patterns implemented across the tui-realm v3 interactive UI.

## Core Types

```rust
// Main error type
pub enum AppError {
    // I/O Errors
    FileReadError { path: String, source: Arc<std::io::Error> },
    
    // Parsing Errors
    InvalidQueryError { query: String, details: String },
    JsonParseError { details: String },
    
    // Component Errors
    ComponentInitError { component: String, details: String },
    ComponentUpdateError { component: String, details: String },
    
    // Service Errors
    SearchServiceError { details: String },
    SessionServiceError { details: String },
    ClipboardServiceError { details: String },
    
    // Threading Errors
    MutexPoisonError { resource: String },
    ChannelError { details: String },
    
    // Generic Errors
    Unknown { details: String },
}

// Type alias for Results
pub type AppResult<T> = Result<T, AppError>;
```

## Error Conversion

Automatic conversion from common error types:

```rust
// From std::io::Error
let file = File::open(path)?;  // Automatically converts to AppError::FileReadError

// From serde_json::Error
let data: MyType = serde_json::from_str(&json)?;  // Converts to AppError::JsonParseError

// From mutex poisoning
let guard = mutex.lock()?;  // Converts to AppError::MutexPoisonError

// From channel errors
tx.send(msg)?;  // Converts to AppError::ChannelError
```

## Service Error Handling

### Search Service
```rust
fn execute_search(...) -> AppResult<Vec<SearchResult>> {
    // Parse query with proper error context
    let condition = parse_query(&query).map_err(|e| AppError::InvalidQueryError {
        query: query.clone(),
        details: e.to_string(),
    })?;
    
    // Execute search with error propagation
    engine.search(&glob_pattern, condition)
        .map(|(results, _, _)| results)
        .map_err(|e| AppError::SearchServiceError {
            details: e.to_string(),
        })
}
```

### Session Service
```rust
pub fn load_session(&mut self, session_id: &str) -> AppResult<Vec<String>> {
    // Check cache first
    if let Some(messages) = self.cache.get(session_id) {
        return Ok(messages.clone());
    }
    
    // Find file with proper error handling
    let file_path = self.find_session_file(session_id)?;
    
    // Load messages with error propagation
    let messages = self.load_messages_from_file(&file_path, session_id)?;
    
    Ok(messages)
}
```

## Component Error Handling

```rust
// Component initialization with error handling
app.mount(
    ComponentId::SearchInput,
    Box::new(SearchInput::new()),
    vec![],
).map_err(|e| AppError::ComponentInitError {
    component: "SearchInput".to_string(),
    details: e.to_string(),
})?;

// Component update with error handling
Self::update_attr(
    app,
    &ComponentId::SearchInput,
    Attribute::Text,
    AttrValue::String(query),
).map_err(|e| AppError::ComponentUpdateError {
    component: "SearchInput".to_string(),
    details: e.to_string(),
})?;
```

## Mutex Handling

Safe mutex handling with poisoning recovery:

```rust
// Store operation with mutex recovery
let mut counter = match self.counter.lock() {
    Ok(guard) => guard,
    Err(poisoned) => {
        eprintln!("Warning: Counter mutex was poisoned, recovering...");
        poisoned.into_inner()
    }
};
```

## User-Facing Error Recovery

```rust
pub struct RecoverableError {
    pub error: AppError,
    pub recovery_suggestions: Vec<String>,
    pub can_retry: bool,
}

// Example usage
let recoverable = RecoverableError::new(error);
if recoverable.can_retry {
    // Show retry option to user
    self.state.set_message(recoverable.user_message());
}
```

## Error Display in UI

1. **Search Errors**: Displayed in status message area
2. **Session Load Errors**: Show error dialog with suggestions
3. **Clipboard Errors**: Brief notification with recovery hints
4. **Critical Errors**: Full error dialog with retry option

## Best Practices

1. **Always use AppResult<T>** for functions that can fail
2. **Provide context** when converting errors using map_err
3. **Log errors** before converting to user-friendly messages
4. **Offer recovery suggestions** for common errors
5. **Handle mutex poisoning** gracefully without panicking
6. **Use the ? operator** for clean error propagation
7. **Display errors appropriately** based on severity

## Error Boundary Pattern

For operations that should not crash the application:

```rust
let result = error_boundary(
    || risky_operation(),
    default_value
);
```

## Testing Error Conditions

```rust
#[test]
fn test_error_handling() {
    let error = AppError::FileReadError {
        path: "test.json".to_string(),
        source: Arc::new(std::io::Error::new(std::io::ErrorKind::NotFound, "")),
    };
    
    let recoverable = RecoverableError::new(error);
    assert!(recoverable.can_retry);
    assert!(!recoverable.recovery_suggestions.is_empty());
}
```