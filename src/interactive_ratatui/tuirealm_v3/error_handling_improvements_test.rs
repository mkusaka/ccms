/// Tests for error handling improvements
use super::error::{AppError, RecoverableError, error_boundary};
use std::sync::Arc;

#[test]
fn test_app_error_display() {
    // Test Display trait implementation
    let error = AppError::FileReadError {
        path: "test.json".to_string(),
        source: Arc::new(std::io::Error::new(std::io::ErrorKind::NotFound, "")),
    };
    assert_eq!(error.to_string(), "Failed to read file: test.json");
    
    let error = AppError::InvalidQueryError {
        query: "test query".to_string(),
        details: "invalid syntax".to_string(),
    };
    assert_eq!(error.to_string(), "Invalid query syntax: test query - invalid syntax");
}

#[test]
fn test_recoverable_error() {
    // Test recoverable error with suggestions
    let error = AppError::FileReadError {
        path: "test.json".to_string(),
        source: Arc::new(std::io::Error::new(std::io::ErrorKind::NotFound, "")),
    };
    
    let recoverable = RecoverableError::new(error);
    assert!(recoverable.can_retry);
    assert_eq!(recoverable.recovery_suggestions.len(), 2);
    assert!(recoverable.recovery_suggestions.contains(&"Check if the file exists".to_string()));
    
    let message = recoverable.user_message();
    assert!(message.contains("Error: Failed to read file"));
    assert!(message.contains("Suggestions:"));
    assert!(message.contains("Press 'r' to retry"));
}

#[test]
fn test_error_boundary() {
    // Test error boundary catches errors
    let result = error_boundary(
        || Err(AppError::Unknown { details: "test error".to_string() }),
        42
    );
    assert_eq!(result, 42);
    
    // Test error boundary passes through success
    let result = error_boundary(
        || Ok(100),
        42
    );
    assert_eq!(result, 100);
}


#[test]
fn test_error_clone() {
    // Test that AppError implements Clone
    let error1 = AppError::SearchServiceError {
        details: "test error".to_string(),
    };
    let error2 = error1.clone();
    assert_eq!(error1.to_string(), error2.to_string());
}

#[test]
fn test_error_source() {
    // Test error source chain
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let error = AppError::FileReadError {
        path: "test.json".to_string(),
        source: Arc::new(io_error),
    };
    
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn test_non_retryable_errors() {
    let error = AppError::InvalidQueryError {
        query: "bad query".to_string(),
        details: "syntax error".to_string(),
    };
    
    let recoverable = RecoverableError::new(error);
    assert!(!recoverable.can_retry);
    
    let message = recoverable.user_message();
    assert!(!message.contains("Press 'r' to retry"));
}

#[test]
fn test_error_without_suggestions() {
    let error = AppError::Unknown {
        details: "unknown error".to_string(),
    };
    
    let recoverable = RecoverableError::new(error);
    assert!(recoverable.recovery_suggestions.is_empty());
    assert!(!recoverable.can_retry);
}