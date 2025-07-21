#[cfg(feature = "async")]
pub mod async_engine;
pub mod engine;
pub mod file_discovery;
mod mmap_reader;
mod fast_json_scanner;
#[allow(dead_code)]
mod optimized_engine;

#[cfg(feature = "async")]
pub use async_engine::{AsyncSearchEngine, AsyncSearchOptions, AsyncSearchResult};
pub use engine::{SearchEngine, format_search_result};
pub use file_discovery::{default_claude_pattern, discover_claude_files, discover_claude_files_with_filter, expand_tilde};
