pub mod engine;
pub mod file_discovery;
#[cfg(feature = "async")]
pub mod async_engine;

pub use engine::{SearchEngine, format_search_result};
pub use file_discovery::{discover_claude_files, expand_tilde, default_claude_pattern};
#[cfg(feature = "async")]
pub use async_engine::{AsyncSearchEngine, AsyncSearchOptions, AsyncSearchResult};