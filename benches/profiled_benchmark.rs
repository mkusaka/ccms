use ccms::{SearchEngineTrait, SmolEngine, RayonEngine, SearchOptions, parse_query};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[cfg(feature = "profiling")]
use pprof::ProfilerGuardBuilder;

fn main() {
    // プロファイリング開始
    #[cfg(feature = "profiling")]
    let guard = ProfilerGuardBuilder::default()
        .frequency(1000) // 1000Hz sampling
        .blocklist(&["libc", "libgcc", "pthread"])
        .build()
        .expect("Failed to build profiler");
    
    // テストデータ作成
    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();
    
    for i in 0..20 {
        let file_path = temp_dir.path().join(format!("session{}.jsonl", i));
        let mut file = File::create(&file_path).unwrap();
        
        for j in 0..1000 {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} with error code {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s{}","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j, j, format!("{}-{}", i, j), j % 60, i
            ).unwrap();
        }
        files.push(file_path);
    }
    
    let pattern = format!("{}/*.jsonl", temp_dir.path().display());
    let options = SearchOptions::default();
    
    // 実際のベンチマーク
    println!("Running benchmark with profiling...");
    
    // Smol
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let engine = SmolEngine::new(options.clone());
        let query = parse_query("error").unwrap();
        let (results, _, _) = engine.search(&pattern, query).unwrap();
        println!("Smol: {} results", results.len());
    }
    let smol_time = start.elapsed();
    
    // Rayon
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let engine = RayonEngine::new(options.clone());
        let query = parse_query("error").unwrap();
        let (results, _, _) = engine.search(&pattern, query).unwrap();
        println!("Rayon: {} results", results.len());
    }
    let rayon_time = start.elapsed();
    
    println!("\nResults:");
    println!("Smol: {:?} (avg: {:?})", smol_time, smol_time / 10);
    println!("Rayon: {:?} (avg: {:?})", rayon_time, rayon_time / 10);
    
    // プロファイル保存
    #[cfg(feature = "profiling")]
    {
        if let Ok(report) = guard.report().build() {
            // Save flamegraph
            let svg_file = std::fs::File::create("benchmark_profile.svg").unwrap();
            if let Err(e) = report.flamegraph(svg_file) {
                eprintln!("Failed to write flamegraph: {}", e);
            } else {
                println!("\nFlamegraph saved to benchmark_profile.svg");
                println!("Open with: open benchmark_profile.svg");
            }
        } else {
            eprintln!("Failed to generate profile report");
        }
    }
}