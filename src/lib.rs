pub mod interactive_ratatui;
pub mod profiling;
#[cfg(feature = "profiling")]
pub mod profiling_enhanced;
pub mod query;
pub mod schemas;
pub mod search;

pub use query::{QueryCondition, SearchOptions, SearchResult, parse_query};
pub use schemas::{SessionMessage, ToolResult};
pub use search::{
    SearchEngine, default_claude_pattern, discover_claude_files, expand_tilde, format_search_result,
};
