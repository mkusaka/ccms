#[cfg(feature = "async")]
use ccms::search::OptimizedAsyncSearchEngine;
use ccms::{parse_query, SearchEngine, SearchOptions};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <pattern> <query>", args[0]);
        std::process::exit(1);
    }

    let pattern = &args[1];
    let query_str = &args[2];
    let query = parse_query(query_str)?;
    
    let options = SearchOptions {
        max_results: Some(50),
        verbose: true, // Enable verbose output
        ..Default::default()
    };

    println!("=== Rayon Performance ===");
    let start = Instant::now();
    let engine = SearchEngine::new(options.clone());
    let (results, duration, total) = engine.search(pattern, query.clone())?;
    let total_time = start.elapsed();
    
    println!("Results found: {}", results.len());
    println!("Total matches: {}", total);
    println!("Search duration: {:?}", duration);
    println!("Total time: {:?}", total_time);
    println!();

    #[cfg(feature = "async")]
    {
        println!("=== Tokio Performance ===");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // Test different configurations
            let configs = vec![
                ("Default", None, None, None),
                ("High concurrency", Some(32), None, None),
                ("Large buffer", None, Some(128 * 1024), None),
                ("No hybrid parsing", None, None, Some(false)),
                ("Low concurrency", Some(4), None, None),
            ];
            
            for (name, concurrency, buffer_size, hybrid) in configs {
                println!("Configuration: {}", name);
                
                let start = Instant::now();
                let mut engine = OptimizedAsyncSearchEngine::new(options.clone());
                
                if let Some(c) = concurrency {
                    engine = engine.with_concurrency(c);
                }
                if let Some(b) = buffer_size {
                    engine = engine.with_buffer_size(b);
                }
                if let Some(h) = hybrid {
                    engine = engine.with_hybrid_parsing(h);
                }
                
                let (results, duration, total) = engine.search(pattern, query.clone()).await?;
                let total_time = start.elapsed();
                
                println!("  Results found: {}", results.len());
                println!("  Total matches: {}", total);
                println!("  Search duration: {:?}", duration);
                println!("  Total time: {:?}", total_time);
                println!();
            }
            
            Ok::<_, anyhow::Error>(())
        })?;
    }

    Ok(())
}