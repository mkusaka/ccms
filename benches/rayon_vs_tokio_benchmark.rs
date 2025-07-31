use codspeed_criterion_compat::{Criterion, criterion_group, criterion_main, BenchmarkId, black_box};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use ccms::{SearchEngine, SearchOptions, parse_query};
#[cfg(feature = "async")]
use ccms::{AsyncSearchEngine, AsyncSearchOptions};
#[cfg(feature = "async")]
use ccms::search::OptimizedAsyncSearchEngine;

// Create test JSONL files for benchmarking
fn create_test_jsonl(num_lines: usize, line_size: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    let long_text = "x".repeat(line_size);

    for i in 0..num_lines {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message {} test content {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i,
            long_text,
            i,
            i % 60
        )
        .unwrap();
    }

    (temp_dir, test_file.to_string_lossy().to_string())
}

// Create multiple files for concurrency testing
fn create_multiple_files(num_files: usize, lines_per_file: usize) -> (Vec<tempfile::TempDir>, String) {
    let mut temp_dirs = Vec::new();
    let parent_dir = tempdir().unwrap();
    
    for i in 0..num_files {
        let test_file = parent_dir.path().join(format!("test{i}.jsonl"));
        let mut file = File::create(&test_file).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                j,
                j % 60
            )
            .unwrap();
        }
    }
    
    let pattern = format!("{}/*.jsonl", parent_dir.path().display());
    temp_dirs.push(parent_dir);
    (temp_dirs, pattern)
}

fn benchmark_single_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_file_search");
    
    #[cfg(feature = "async")]
    let rt = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 1000, 10000].iter() {
        let (_temp_dir, test_file) = create_test_jsonl(*size, 100);
        let query = parse_query("test").unwrap();
        
        // Benchmark Rayon
        group.bench_with_input(BenchmarkId::new("rayon", size), &test_file, |b, pattern| {
            b.iter(|| {
                let options = SearchOptions {
                    max_results: Some(50),
                    ..Default::default()
                };
                let engine = SearchEngine::new(options);
                let (results, _, _) = engine.search(pattern, query.clone()).unwrap();
                black_box(results.len())
            });
        });
        
        // Benchmark Basic Tokio
        #[cfg(feature = "async")]
        group.bench_with_input(BenchmarkId::new("tokio_basic", size), &test_file, |b, pattern| {
            b.iter(|| {
                rt.block_on(async {
                    let options = AsyncSearchOptions {
                        max_results: Some(50),
                        ..Default::default()
                    };
                    let engine = AsyncSearchEngine::new(options);
                    let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                    black_box(results.len())
                })
            });
        });
        
        // Benchmark Optimized Tokio
        #[cfg(feature = "async")]
        group.bench_with_input(BenchmarkId::new("tokio_optimized", size), &test_file, |b, pattern| {
            b.iter(|| {
                rt.block_on(async {
                    let options = SearchOptions {
                        max_results: Some(50),
                        ..Default::default()
                    };
                    let engine = OptimizedAsyncSearchEngine::new(options);
                    let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                    black_box(results.len())
                })
            });
        });
    }
    
    group.finish();
}

fn benchmark_multiple_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_files_search");
    
    #[cfg(feature = "async")]
    let rt = tokio::runtime::Runtime::new().unwrap();

    for num_files in [10, 50, 100].iter() {
        let (_temp_dirs, pattern) = create_multiple_files(*num_files, 1000);
        let query = parse_query("Message").unwrap();
        
        // Benchmark Rayon
        group.bench_with_input(
            BenchmarkId::new("rayon", num_files),
            &pattern,
            |b, pattern| {
                b.iter(|| {
                    let options = SearchOptions {
                        max_results: Some(100),
                        ..Default::default()
                    };
                    let engine = SearchEngine::new(options);
                    let (results, _, _) = engine.search(pattern, query.clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Basic Tokio
        #[cfg(feature = "async")]
        group.bench_with_input(
            BenchmarkId::new("tokio_basic", num_files),
            &pattern,
            |b, pattern| {
                b.iter(|| {
                    rt.block_on(async {
                        let options = AsyncSearchOptions {
                            max_results: Some(100),
                            ..Default::default()
                        };
                        let engine = AsyncSearchEngine::new(options);
                        let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                        black_box(results.len())
                    })
                });
            },
        );
        
        // Benchmark Optimized Tokio
        #[cfg(feature = "async")]
        group.bench_with_input(
            BenchmarkId::new("tokio_optimized", num_files),
            &pattern,
            |b, pattern| {
                b.iter(|| {
                    rt.block_on(async {
                        let options = SearchOptions {
                            max_results: Some(100),
                            ..Default::default()
                        };
                        let engine = OptimizedAsyncSearchEngine::new(options);
                        let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                        black_box(results.len())
                    })
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_files_search");
    
    #[cfg(feature = "async")]
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test with files larger than 1MB threshold
    for size in [50000, 100000].iter() {
        let (_temp_dir, test_file) = create_test_jsonl(*size, 200);
        let query = parse_query("test").unwrap();
        
        // Benchmark Rayon
        group.bench_with_input(BenchmarkId::new("rayon", size), &test_file, |b, pattern| {
            b.iter(|| {
                let options = SearchOptions {
                    max_results: Some(100),
                    ..Default::default()
                };
                let engine = SearchEngine::new(options);
                let (results, _, _) = engine.search(pattern, query.clone()).unwrap();
                black_box(results.len())
            });
        });
        
        // Benchmark Optimized Tokio
        #[cfg(feature = "async")]
        group.bench_with_input(BenchmarkId::new("tokio_optimized", size), &test_file, |b, pattern| {
            b.iter(|| {
                rt.block_on(async {
                    let options = SearchOptions {
                        max_results: Some(100),
                        ..Default::default()
                    };
                    let engine = OptimizedAsyncSearchEngine::new(options);
                    let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                    black_box(results.len())
                })
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "async")]
fn benchmark_concurrency_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency_levels");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_temp_dirs, pattern) = create_multiple_files(50, 1000);
    let query = parse_query("Message").unwrap();
    
    // Test different concurrency levels for optimized tokio
    for concurrency in [1, 4, 8, 16, 32, 64].iter() {
        group.bench_with_input(
            BenchmarkId::new("tokio_optimized", concurrency),
            &pattern,
            |b, pattern| {
                b.iter(|| {
                    rt.block_on(async {
                        let options = SearchOptions {
                            max_results: Some(100),
                            ..Default::default()
                        };
                        let engine = OptimizedAsyncSearchEngine::new(options)
                            .with_concurrency(*concurrency);
                        let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                        black_box(results.len())
                    })
                });
            },
        );
    }
    
    group.finish();
}

#[cfg(not(feature = "async"))]
fn benchmark_concurrency_levels(_c: &mut Criterion) {
    println!("Concurrency benchmarks require --features async");
}

criterion_group!(
    benches,
    benchmark_single_file,
    benchmark_multiple_files,
    benchmark_large_files,
    benchmark_concurrency_levels
);
criterion_main!(benches);