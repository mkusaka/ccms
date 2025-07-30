#[cfg(feature = "async")]
use ccms::search::OptimizedAsyncSearchEngine;
use ccms::{parse_query, SearchEngine, SearchOptions};
use std::time::Instant;

#[cfg(feature = "profiling")]
use ccms::profiling_enhanced::EnhancedProfiler;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <engine> <pattern> <query> [--profile]", args[0]);
        eprintln!("Engine: rayon or tokio");
        eprintln!("Options:");
        eprintln!("  --profile    Enable CPU profiling");
        std::process::exit(1);
    }

    let engine = &args[1];
    let pattern = &args[2];
    let query_str = &args[3];
    let profile = args.len() > 4 && args[4] == "--profile";

    #[cfg(feature = "profiling")]
    let mut profiler = if profile {
        Some(EnhancedProfiler::new(engine)?)
    } else {
        None
    };

    let query = parse_query(query_str)?;
    let options = SearchOptions {
        max_results: Some(50),
        verbose: true,  // Enable verbose for detailed timing
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
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
                
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

    #[cfg(feature = "profiling")]
    if let Some(mut profiler) = profiler {
        let report = profiler.generate_comprehensive_report(&format!("profile_{}", engine))?;
        println!("\n{}", report);
    }

    Ok(())
}