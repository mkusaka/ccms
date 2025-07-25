use std::fmt;

/// Central error type for the interactive iocraft module
#[derive(Debug)]
pub enum IocraftError {
    /// Service not found in context
    MissingService { service_name: &'static str },
    /// IO operation failed
    Io(std::io::Error),
    /// Clipboard operation failed
    Clipboard(String),
    /// Session loading failed
    SessionLoad { path: String, source: anyhow::Error },
    /// Search operation failed
    Search(anyhow::Error),
    /// Configuration error
    Config(String),
}

impl fmt::Display for IocraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IocraftError::MissingService { service_name } => {
                write!(f, "Required service '{}' not found in context. Ensure all services are properly initialized.", service_name)
            }
            IocraftError::Io(e) => write!(f, "IO operation failed: {}", e),
            IocraftError::Clipboard(e) => write!(f, "Clipboard operation failed: {}", e),
            IocraftError::SessionLoad { path, source } => {
                write!(f, "Failed to load session from '{}': {}", path, source)
            }
            IocraftError::Search(e) => write!(f, "Search operation failed: {}", e),
            IocraftError::Config(e) => write!(f, "Configuration error: {}", e),
        }
    }
}

impl std::error::Error for IocraftError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IocraftError::Io(e) => Some(e),
            IocraftError::SessionLoad { source, .. } => Some(source.as_ref()),
            IocraftError::Search(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for IocraftError {
    fn from(e: std::io::Error) -> Self {
        IocraftError::Io(e)
    }
}

impl From<anyhow::Error> for IocraftError {
    fn from(e: anyhow::Error) -> Self {
        IocraftError::Search(e)
    }
}

/// Result type alias for iocraft operations
pub type IocraftResult<T> = Result<T, IocraftError>;

/// Extension trait for better error context
pub trait ErrorContext<T> {
    fn context_service(self, service_name: &'static str) -> IocraftResult<T>;
    fn context_session(self, path: String) -> IocraftResult<T>;
}

impl<T> ErrorContext<T> for Option<T> {
    fn context_service(self, service_name: &'static str) -> IocraftResult<T> {
        self.ok_or(IocraftError::MissingService { service_name })
    }
    
    fn context_session(self, path: String) -> IocraftResult<T> {
        self.ok_or_else(|| IocraftError::SessionLoad {
            path,
            source: anyhow::anyhow!("Session not found"),
        })
    }
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn context_service(self, _service_name: &'static str) -> IocraftResult<T> {
        self.map_err(|e| IocraftError::Search(e.into()))
    }
    
    fn context_session(self, path: String) -> IocraftResult<T> {
        self.map_err(|e| IocraftError::SessionLoad {
            path,
            source: e.into(),
        })
    }
}