use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_rayon_engine_v2::OptimizedRayonEngineV2;

fn main() -> Result<()> {
    let pattern = "~/.claude/projects/**/*.jsonl";
    let query = parse_query("claude")?;
    let options = SearchOptions::default();
    
    let engine = OptimizedRayonEngineV2::new(options);
    let (_results, _duration, total) = engine.search(pattern, query)?;
    println!("Found {} results", total);
    
    Ok(())
}