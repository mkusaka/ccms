use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_async_engine::OptimizedAsyncSearchEngine;
use ccms::search::optimized_async_engine_v4::OptimizedAsyncSearchEngineV4;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    let pattern = "~/.claude/projects/**/*.jsonl";
    let query = parse_query("claude")?;
    let options = SearchOptions::default();
    
    // Test V1 (current optimized version)
    println!("Testing V1 (current optimized version)...");
    let start = Instant::now();
    let engine_v1 = OptimizedAsyncSearchEngine::new(options.clone());
    let (results_v1, _, total_v1) = engine_v1.search(pattern, query.clone()).await?;
    let v1_time = start.elapsed();
    println!("V1: Found {} results in {:?}", total_v1, v1_time);
    
    // Test V4 (worker pool version)
    println!("\nTesting V4 (worker pool version)...");
    let start = Instant::now();
    let engine_v4 = OptimizedAsyncSearchEngineV4::new(options);
    let (results_v4, _, total_v4) = engine_v4.search(pattern, query).await?;
    let v4_time = start.elapsed();
    println!("V4: Found {} results in {:?}", total_v4, v4_time);
    
    // Compare results
    println!("\nComparison:");
    println!("V1 time: {:?}", v1_time);
    println!("V4 time: {:?}", v4_time);
    let improvement = ((v1_time.as_millis() as f64 - v4_time.as_millis() as f64) / v1_time.as_millis() as f64) * 100.0;
    if improvement > 0.0 {
        println!("V4 is {:.1}% faster", improvement);
    } else {
        println!("V4 is {:.1}% slower", -improvement);
    }
    
    Ok(())
}
