pub mod engine;
pub mod file_discovery;
pub use engine::{SearchEngine, format_search_result};
pub use file_discovery::{default_claude_pattern, discover_claude_files, expand_tilde};
