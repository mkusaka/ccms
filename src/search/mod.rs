#[cfg(feature = "async")]
pub mod async_engine;
pub mod engine;
pub mod file_discovery;
#[cfg(feature = "async")]
pub mod optimized_async_engine;
#[cfg(feature = "async")]
pub mod optimized_async_engine_v2;
#[cfg(feature = "async")]
pub mod optimized_async_engine_v3;

#[cfg(feature = "async")]
pub use async_engine::{AsyncSearchEngine, AsyncSearchOptions, AsyncSearchResult};
pub use engine::{SearchEngine, format_search_result};
pub use file_discovery::{default_claude_pattern, discover_claude_files, expand_tilde};
#[cfg(feature = "async")]
pub use optimized_async_engine::OptimizedAsyncSearchEngine;
#[cfg(feature = "async")]
pub use optimized_async_engine_v2::OptimizedAsyncSearchEngineV2;
#[cfg(feature = "async")]
pub use optimized_async_engine_v3::OptimizedAsyncSearchEngineV3;
