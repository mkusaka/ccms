pub mod interactive_ratatui;
// pub mod interactive_iocraft; // Temporarily disabled to fix compilation
pub mod profiling;
pub mod query;
pub mod schemas;
pub mod search;

/// Arguments for interactive mode
#[derive(Debug, Clone)]
pub struct Args {
    pub query: Option<String>,
    pub file_patterns: Vec<String>,
    pub verbose: bool,
}

pub use query::{QueryCondition, SearchOptions, SearchResult, parse_query};
pub use schemas::{SessionMessage, ToolResult};
pub use search::{
    SearchEngine, default_claude_pattern, discover_claude_files, expand_tilde, format_search_result,
};

#[cfg(feature = "async")]
pub use search::{AsyncSearchEngine, AsyncSearchOptions, AsyncSearchResult};
