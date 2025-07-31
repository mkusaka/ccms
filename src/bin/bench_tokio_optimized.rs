use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::optimized_async_engine_v4::OptimizedAsyncSearchEngineV4;

fn main() -> Result<()> {
    // Build optimized runtime configuration based on o4-search recommendations
    let num_cpus = num_cpus::get();
    
    // For mixed I/O and CPU workloads
    // Use all cores for worker threads for maximum parallelism
    let worker_threads = num_cpus;
    
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(64)  // Cap for spawn_blocking
        .thread_keep_alive(std::time::Duration::from_secs(10))
        .enable_io()
        .enable_time()
        .thread_name("ccms-worker")
        .on_thread_start(|| {
            // Thread-local initialization if needed
        })
        .build()?;
    
    rt.block_on(async {
        let pattern = std::env::args().nth(1).unwrap_or_else(|| "~/.claude/projects/**/*.jsonl".to_string());
        let query_str = std::env::args().nth(2).unwrap_or_else(|| "claude".to_string());
        
        let query = parse_query(&query_str)?;
        let options = SearchOptions::default();
        
        // Use more workers than runtime threads for better throughput
        let engine = OptimizedAsyncSearchEngineV4::new(options)
            .with_num_workers(num_cpus * 2);  // 2x the number of cores
        
        let (results, duration, total) = engine.search(&pattern, query).await?;
        
        println!("Found {} results in {:?}", total, duration);
        
        Ok(())
    })
}