use ccms::{SearchEngine, SearchOptions, parse_query};
use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

fn create_test_project(num_files: usize, lines_per_file: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    let projects_dir = temp_dir.path().join(".claude").join("projects").join("test-project");
    std::fs::create_dir_all(&projects_dir).unwrap();
    
    for i in 0..num_files {
        let file_path = projects_dir.join(format!("session{}.jsonl", i));
        let mut file = File::create(&file_path).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {} with some test content to search through"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                j,
                j % 60
            )
            .unwrap();
        }
    }

    let pattern = format!("{}/**/*.jsonl", temp_dir.path().display());
    (temp_dir, pattern)
}

// Helper to run search in a subprocess with specific BLOCKING_MAX_THREADS
fn run_search_subprocess(pattern: &str, query: &str, blocking_threads: Option<usize>) -> std::time::Duration {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(&["--", pattern, query]);
    
    if let Some(threads) = blocking_threads {
        cmd.env("BLOCKING_MAX_THREADS", threads.to_string());
    }
    
    let start = std::time::Instant::now();
    let output = cmd.output().expect("Failed to execute search");
    let elapsed = start.elapsed();
    
    if !output.status.success() {
        panic!("Search failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    elapsed
}

fn benchmark_search_engine_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_engine_comparison");
    
    // Create test data
    let (_temp_dir, pattern) = create_test_project(100, 500); // 100 files, 500 lines each
    let query = parse_query("test").unwrap();
    let options = SearchOptions {
        max_results: Some(100),
        ..Default::default()
    };
    
    // Benchmark current Smol implementation
    group.bench_function("smol_current", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine.search(&pattern, query.clone()).unwrap();
            black_box(results.len())
        });
    });
    
    // Note: To properly test different BLOCKING_MAX_THREADS values, you would need to:
    // 1. Run each benchmark as a separate process
    // 2. Or modify the SearchEngine to allow runtime configuration
    
    group.finish();
}

// Alternative: Simple comparison of file processing approaches
fn benchmark_file_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_processing");
    
    // Create test files
    let temp_dir = tempdir().unwrap();
    let mut files = Vec::new();
    
    for i in 0..50 {
        let file_path = temp_dir.path().join(format!("test{}.jsonl", i));
        let mut file = File::create(&file_path).unwrap();
        
        for j in 0..200 {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} with test content"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j, j
            ).unwrap();
        }
        
        files.push(file_path.to_string_lossy().to_string());
    }
    
    // Rayon implementation
    group.bench_function("rayon", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let count: usize = files
                .par_iter()
                .map(|file| {
                    let content = std::fs::read_to_string(file).unwrap();
                    content.lines().filter(|line| line.contains("test")).count()
                })
                .sum();
            black_box(count)
        });
    });
    
    // Smol implementation (with current BLOCKING_MAX_THREADS)
    group.bench_function("smol", |b| {
        b.iter(|| {
            let count = smol::block_on(async {
                use futures_lite::stream::{self, StreamExt};
                
                let stream = stream::iter(files.iter()).map(|file| {
                    let file = file.clone();
                    smol::spawn(async move {
                        // This uses blocking::unblock internally
                        let content = async_fs::read_to_string(file).await.unwrap();
                        content.lines().filter(|line| line.contains("test")).count()
                    })
                });
                
                let mut total = 0;
                let mut stream = Box::pin(stream);
                while let Some(task) = stream.next().await {
                    total += task.await;
                }
                total
            });
            black_box(count)
        });
    });
    
    // Direct blocking implementation for comparison
    group.bench_function("smol_blocking", |b| {
        b.iter(|| {
            let count = smol::block_on(async {
                use futures_lite::stream::{self, StreamExt};
                
                let stream = stream::iter(files.iter()).map(|file| {
                    let file = file.clone();
                    smol::spawn(async move {
                        blocking::unblock(move || {
                            let content = std::fs::read_to_string(file).unwrap();
                            content.lines().filter(|line| line.contains("test")).count()
                        }).await
                    })
                });
                
                let mut total = 0;
                let mut stream = Box::pin(stream);
                while let Some(task) = stream.next().await {
                    total += task.await;
                }
                total
            });
            black_box(count)
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_search_engine_comparison, benchmark_file_processing);
criterion_main!(benches);