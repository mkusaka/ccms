use anyhow::Result;
use ccms::{parse_query, SearchOptions};
use ccms::search::smol_engine::SmolSearchEngine;
use ccms::search::optimized_smol_engine::{OptimizedSmolSearchEngine, init_executor};

fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let optimized = args.contains(&"--optimized".to_string());
    let query_str = args.get(1).unwrap_or(&"claude".to_string()).clone();
    
    // Parse query
    let query = parse_query(&query_str)?;
    
    // Create search options
    let options = SearchOptions {
        max_results: Some(50),
        role: None,
        session_id: None,
        before: None,
        after: None,
        verbose: true,
        project_path: None,
    };
    
    let pattern = ccms::default_claude_pattern();
    
    if optimized {
        eprintln!("Using OptimizedSmolSearchEngine with multi-threaded executor");
        // Initialize multi-threaded executor
        init_executor();
        
        // Run search with optimized engine
        smol::block_on(async {
            let engine = OptimizedSmolSearchEngine::new(options);
            let (results, duration, total) = engine.search(&pattern, query).await?;
            
            println!("\nFound {} results in {}ms (total: {})", 
                     results.len(), 
                     duration.as_millis(),
                     total);
            
            Ok::<(), anyhow::Error>(())
        })?;
    } else {
        eprintln!("Using standard SmolSearchEngine");
        
        // Run search with standard engine
        smol::block_on(async {
            let engine = SmolSearchEngine::new(options);
            let (results, duration, total) = engine.search(&pattern, query).await?;
            
            println!("\nFound {} results in {}ms (total: {})", 
                     results.len(), 
                     duration.as_millis(),
                     total);
            
            Ok::<(), anyhow::Error>(())
        })?;
    }
    
    Ok(())
}