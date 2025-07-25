/// Error handling types for tui-realm v3 implementation
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
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

pub type AppResult<T> = Result<T, AppError>;

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::FileReadError { path, .. } => write!(f, "Failed to read file: {}", path),
            AppError::InvalidQueryError { query, details } => write!(f, "Invalid query syntax: {} - {}", query, details),
            AppError::JsonParseError { details } => write!(f, "JSON parsing error: {}", details),
            AppError::ComponentInitError { component, details } => write!(f, "Component initialization failed: {} - {}", component, details),
            AppError::ComponentUpdateError { component, details } => write!(f, "Component update failed: {} - {}", component, details),
            AppError::SearchServiceError { details } => write!(f, "Search service error: {}", details),
            AppError::SessionServiceError { details } => write!(f, "Session service error: {}", details),
            AppError::ClipboardServiceError { details } => write!(f, "Clipboard service error: {}", details),
            AppError::MutexPoisonError { resource } => write!(f, "Mutex poisoned for resource: {}", resource),
            AppError::ChannelError { details } => write!(f, "Channel communication error: {}", details),
            AppError::Unknown { details } => write!(f, "Unknown error: {}", details),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::FileReadError { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Error with recovery suggestions
#[derive(Debug)]
pub struct RecoverableError {
    pub error: AppError,
    pub recovery_suggestions: Vec<String>,
    pub can_retry: bool,
}

impl RecoverableError {
    pub fn new(error: AppError) -> Self {
        let (suggestions, can_retry) = match &error {
            AppError::FileReadError { .. } => (
                vec![
                    "Check if the file exists".to_string(),
                    "Verify file permissions".to_string(),
                ],
                true
            ),
            AppError::ClipboardServiceError { .. } => (
                vec![
                    "Check clipboard access permissions".to_string(),
                    "Try copying smaller content".to_string(),
                ],
                true
            ),
            AppError::InvalidQueryError { .. } => (
                vec![
                    "Check query syntax".to_string(),
                    "Remove special characters if any".to_string(),
                ],
                false
            ),
            AppError::JsonParseError { .. } => (
                vec![
                    "Check JSON format".to_string(),
                    "Verify file is not corrupted".to_string(),
                ],
                false
            ),
            AppError::SearchServiceError { .. } => (
                vec![
                    "Check search parameters".to_string(),
                    "Try a simpler query".to_string(),
                ],
                true
            ),
            AppError::SessionServiceError { .. } => (
                vec![
                    "Verify session file exists".to_string(),
                    "Check session ID format".to_string(),
                ],
                true
            ),
            AppError::ComponentInitError { .. } => (
                vec![
                    "Restart the application".to_string(),
                ],
                false
            ),
            AppError::ComponentUpdateError { .. } => (
                vec![
                    "Try the operation again".to_string(),
                ],
                true
            ),
            AppError::MutexPoisonError { .. } => (
                vec![
                    "Restart the application".to_string(),
                    "Internal state may be corrupted".to_string(),
                ],
                false
            ),
            AppError::ChannelError { .. } => (
                vec![
                    "Check if search is still running".to_string(),
                    "Try the operation again".to_string(),
                ],
                true
            ),
            AppError::Unknown { .. } => (vec![], false),
        };
        
        Self {
            error,
            recovery_suggestions: suggestions,
            can_retry,
        }
    }
    
    /// Format error for user display
    pub fn user_message(&self) -> String {
        let mut message = format!("Error: {}", self.error);
        
        if !self.recovery_suggestions.is_empty() {
            message.push_str("\n\nSuggestions:");
            for suggestion in &self.recovery_suggestions {
                message.push_str(&format!("\n• {}", suggestion));
            }
        }
        
        if self.can_retry {
            message.push_str("\n\nPress 'r' to retry");
        }
        
        message
    }
}


/// Conversion traits for common error types
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::FileReadError {
            path: String::new(),
            source: Arc::new(err),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonParseError {
            details: err.to_string(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for AppError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        AppError::MutexPoisonError {
            resource: "Unknown".to_string(),
        }
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for AppError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        AppError::ChannelError {
            details: "Failed to send message".to_string(),
        }
    }
}

/// Helper macro for error context
#[macro_export]
macro_rules! context {
    ($result:expr, $msg:expr) => {
        $result.map_err(|e| AppError::Unknown {
            details: format!("{}: {}", $msg, e),
        })
    };
}

/// Wrap operations in error boundary
pub fn error_boundary<F, T>(operation: F, fallback: T) -> T
where
    F: FnOnce() -> AppResult<T>,
    T: Clone,
{
    match operation() {
        Ok(value) => value,
        Err(e) => {
            eprintln!("Error boundary caught: {}", e);
            fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_recovery() {
        let error = AppError::FileReadError {
            path: "test.json".to_string(),
            source: Arc::new(std::io::Error::new(std::io::ErrorKind::NotFound, "")),
        };
        
        let recoverable = RecoverableError::new(error);
        assert!(recoverable.can_retry);
        assert!(!recoverable.recovery_suggestions.is_empty());
    }
    
    #[test]
    fn test_error_boundary() {
        let result = error_boundary(
            || Err(AppError::Unknown { details: "test".to_string() }),
            42
        );
        assert_eq!(result, 42);
    }
    
}