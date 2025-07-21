pub mod schemas;
pub mod query;
pub mod search;
pub mod profiling;
pub mod interactive;

pub use query::{QueryCondition, SearchOptions, SearchResult, parse_query};
pub use search::{SearchEngine, discover_claude_files, expand_tilde, default_claude_pattern, format_search_result};
pub use schemas::{SessionMessage, ToolResult};

#[cfg(feature = "async")]
pub use search::{AsyncSearchEngine, AsyncSearchOptions, AsyncSearchResult};