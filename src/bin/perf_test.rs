#[cfg(feature = "async")]
use ccms::search::OptimizedAsyncSearchEngine;
use ccms::{parse_query, SearchEngine, SearchOptions};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <engine> <pattern> <query>", args[0]);
        eprintln!("Engine: rayon or tokio");
        std::process::exit(1);
    }

    let engine = &args[1];
    let pattern = &args[2];
    let query_str = &args[3];

    let query = parse_query(query_str)?;
    let options = SearchOptions {
        max_results: Some(50),
        verbose: false,
        ..Default::default()
    };

    match engine.as_str() {
        "rayon" => {
            let start = Instant::now();
            let engine = SearchEngine::new(options);
            let (results, duration, total) = engine.search(pattern, query)?;
            let total_time = start.elapsed();
            
            println!("Engine: Rayon");
            println!("Results found: {}", results.len());
            println!("Total matches: {}", total);
            println!("Search duration: {:?}", duration);
            println!("Total time: {:?}", total_time);
        }
        #[cfg(feature = "async")]
        "tokio" => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let start = Instant::now();
                let engine = OptimizedAsyncSearchEngine::new(options);
                let (results, duration, total) = engine.search(pattern, query).await?;
                let total_time = start.elapsed();
                
                println!("Engine: Tokio (Optimized)");
                println!("Results found: {}", results.len());
                println!("Total matches: {}", total);
                println!("Search duration: {:?}", duration);
                println!("Total time: {:?}", total_time);
                
                Ok::<_, anyhow::Error>(())
            })?;
        }
        #[cfg(not(feature = "async"))]
        "tokio" => {
            eprintln!("Tokio engine requires --features async");
            std::process::exit(1);
        }
        _ => {
            eprintln!("Unknown engine: {}. Use 'rayon' or 'tokio'", engine);
            std::process::exit(1);
        }
    }

    Ok(())
}