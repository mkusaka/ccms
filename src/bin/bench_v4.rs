use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_async_engine_v4::OptimizedAsyncSearchEngineV4;

#[tokio::main]
async fn main() -> Result<()> {
    let pattern = "~/.claude/projects/**/*.jsonl";
    let query = parse_query("claude")?;
    let options = SearchOptions::default();
    let engine = OptimizedAsyncSearchEngineV4::new(options);
    let (_results, _duration, total) = engine.search(pattern, query).await?;
    println!("Found {} results", total);
    Ok(())
}
