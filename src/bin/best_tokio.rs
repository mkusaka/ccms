use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_async_engine_v4::OptimizedAsyncSearchEngineV4;

#[tokio::main]
async fn main() -> Result<()> {
    let pattern = std::env::args().nth(1).unwrap_or_else(|| "~/.claude/projects/**/*.jsonl".to_string());
    let query_str = std::env::args().nth(2).unwrap_or_else(|| "claude".to_string());
    
    let query = parse_query(&query_str)?;
    let options = SearchOptions::default();
    
    let engine = OptimizedAsyncSearchEngineV4::new(options);
    let (_results, _duration, _total) = engine.search(&pattern, query).await?;
    
    Ok(())
}
