#[cfg(all(feature = "async", feature = "mmap"))]
use ccms::search::OptimizedAsyncSearchEngineV4;
use ccms::{parse_query, SearchEngine, SearchOptions};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <pattern> <query>", args[0]);
        std::process::exit(1);
    }

    let pattern = &args[1];
    let query_str = &args[2];
    
    let query = parse_query(query_str)?;
    let options = SearchOptions {
        max_results: Some(50),
        verbose: false,
        ..Default::default()
    };

    println!("=== Performance Test with Memory-Mapped I/O ===");
    println!("Pattern: {}", pattern);
    println!("Query: {}", query_str);
    println!();

    // Test Rayon
    {
        println!("--- Rayon ---");
        let engine = SearchEngine::new(options.clone());
        
        let start = Instant::now();
        let (results, duration, total) = engine.search(pattern, query.clone())?;
        let total_time = start.elapsed();
        
        println!("Results: {}, Total: {}, Search: {:?}, Total: {:?}", 
                 results.len(), total, duration, total_time);
    }

    #[cfg(all(feature = "async", feature = "mmap"))]
    {
        println!("\n--- Tokio with mmap ---");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
            
        rt.block_on(async {
            let engine = OptimizedAsyncSearchEngineV4::new(options.clone());
            
            let start = Instant::now();
            let (results, duration, total) = engine.search(pattern, query.clone()).await?;
            let total_time = start.elapsed();
            
            println!("Results: {}, Total: {}, Search: {:?}, Total: {:?}", 
                     results.len(), total, duration, total_time);
            
            Ok::<_, anyhow::Error>(())
        })?;
    }

    Ok(())
}