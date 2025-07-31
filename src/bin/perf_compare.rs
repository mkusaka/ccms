#[cfg(feature = "async")]
use ccms::search::{OptimizedAsyncSearchEngine, OptimizedAsyncSearchEngineV2, OptimizedAsyncSearchEngineV3};
use ccms::{parse_query, SearchEngine, SearchOptions};
use std::time::Instant;

#[cfg(feature = "profiling")]
use ccms::profiling_enhanced::EnhancedProfiler;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <pattern> <query> [--profile]", args[0]);
        eprintln!("Compares performance of different search engines");
        std::process::exit(1);
    }

    let pattern = &args[1];
    let query_str = &args[2];
    let profile = args.len() > 3 && args[3] == "--profile";
    
    let query = parse_query(query_str)?;
    let options = SearchOptions {
        max_results: Some(50),
        verbose: true,
        ..Default::default()
    };

    println!("=== Performance Comparison ===");
    println!("Pattern: {}", pattern);
    println!("Query: {}", query_str);
    println!();

    // Test Rayon
    {
        println!("--- Rayon (baseline) ---");
        let engine = SearchEngine::new(options.clone());
        
        #[cfg(feature = "profiling")]
        let mut profiler = if profile {
            Some(EnhancedProfiler::new("rayon")?)
        } else {
            None
        };

        let start = Instant::now();
        let (results, duration, total) = engine.search(pattern, query.clone())?;
        let total_time = start.elapsed();
        
        println!("Results: {}, Total: {}, Search: {:?}, Total: {:?}", 
                 results.len(), total, duration, total_time);
        
        #[cfg(feature = "profiling")]
        if let Some(mut profiler) = profiler {
            let report = profiler.generate_comprehensive_report("profile_compare_rayon")?;
            println!("\n{}", report);
        }
    }

    #[cfg(feature = "async")]
    {
        println!("\n--- Tokio V1 (original) ---");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
            
        rt.block_on(async {
            let engine = OptimizedAsyncSearchEngine::new(options.clone());
            
            #[cfg(feature = "profiling")]
            let mut profiler = if profile {
                Some(EnhancedProfiler::new("tokio_v1")?)
            } else {
                None
            };

            let start = Instant::now();
            let (results, duration, total) = engine.search(pattern, query.clone()).await?;
            let total_time = start.elapsed();
            
            println!("Results: {}, Total: {}, Search: {:?}, Total: {:?}", 
                     results.len(), total, duration, total_time);
            
            #[cfg(feature = "profiling")]
            if let Some(mut profiler) = profiler {
                let report = profiler.generate_comprehensive_report("profile_compare_tokio_v1")?;
                println!("\n{}", report);
            }
            
            Ok::<_, anyhow::Error>(())
        })?;
    }

    #[cfg(feature = "async")]
    {
        println!("\n--- Tokio V2 (batch processing) ---");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
            
        rt.block_on(async {
            let engine = OptimizedAsyncSearchEngineV2::new(options.clone());
            
            #[cfg(feature = "profiling")]
            let mut profiler = if profile {
                Some(EnhancedProfiler::new("tokio_v2")?)
            } else {
                None
            };

            let start = Instant::now();
            let (results, duration, total) = engine.search(pattern, query.clone()).await?;
            let total_time = start.elapsed();
            
            println!("Results: {}, Total: {}, Search: {:?}, Total: {:?}", 
                     results.len(), total, duration, total_time);
            
            #[cfg(feature = "profiling")]
            if let Some(mut profiler) = profiler {
                let report = profiler.generate_comprehensive_report("profile_compare_tokio_v2")?;
                println!("\n{}", report);
            }
            
            Ok::<_, anyhow::Error>(())
        })?;
        
        // Test different batch sizes
        println!("\n--- Tokio V2 with different batch sizes ---");
        for batch_size in [1, 5, 10, 20, 50] {
            rt.block_on(async {
                let engine = OptimizedAsyncSearchEngineV2::new(options.clone())
                    .with_files_per_batch(batch_size);
                
                let start = Instant::now();
                let (results, _, _) = engine.search(pattern, query.clone()).await?;
                let total_time = start.elapsed();
                
                println!("Batch size {}: {:?} ({} results)", 
                         batch_size, total_time, results.len());
                
                Ok::<_, anyhow::Error>(())
            })?;
        }
    }

    #[cfg(feature = "async")]
    {
        println!("\n--- Tokio V3 (channel optimization) ---");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
            
        rt.block_on(async {
            let engine = OptimizedAsyncSearchEngineV3::new(options.clone());
            
            #[cfg(feature = "profiling")]
            let mut profiler = if profile {
                Some(EnhancedProfiler::new("tokio_v3")?)
            } else {
                None
            };

            let start = Instant::now();
            let (results, duration, total) = engine.search(pattern, query.clone()).await?;
            let total_time = start.elapsed();
            
            println!("Results: {}, Total: {}, Search: {:?}, Total: {:?}", 
                     results.len(), total, duration, total_time);
            
            #[cfg(feature = "profiling")]
            if let Some(mut profiler) = profiler {
                let report = profiler.generate_comprehensive_report("profile_compare_tokio_v3")?;
                println!("\n{}", report);
            }
            
            Ok::<_, anyhow::Error>(())
        })?;
        
        // Test different batch sizes for V3
        println!("\n--- Tokio V3 with different result batch sizes ---");
        for batch_size in [16, 32, 64, 128] {
            rt.block_on(async {
                let engine = OptimizedAsyncSearchEngineV3::new(options.clone())
                    .with_result_batch_size(batch_size);
                
                let start = Instant::now();
                let (results, _, _) = engine.search(pattern, query.clone()).await?;
                let total_time = start.elapsed();
                
                println!("Batch size {}: {:?} ({} results)", 
                         batch_size, total_time, results.len());
                
                Ok::<_, anyhow::Error>(())
            })?;
        }
    }

    Ok(())
}