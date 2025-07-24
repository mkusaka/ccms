#[cfg(feature = "async")]
pub mod async_engine;
pub mod engine;
pub mod file_discovery;
#[cfg(feature = "duckdb")]
pub mod duckdb_engine;

#[cfg(feature = "async")]
pub use async_engine::{AsyncSearchEngine, AsyncSearchOptions, AsyncSearchResult};
pub use engine::{SearchEngine, format_search_result};
pub use file_discovery::{
    default_claude_pattern, discover_claude_files, discover_claude_files_with_filter, expand_tilde,
};
#[cfg(feature = "duckdb")]
pub use duckdb_engine::{DuckDBSearchEngine, DuckDBPersistentEngine};
