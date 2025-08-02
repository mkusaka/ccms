use ccms::{SearchEngineTrait, SmolEngine, RayonEngine, SearchOptions, parse_query};
use ccms::search::rayon_limited_engine::RayonLimitedEngine;
use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main, BenchmarkId};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use std::process::Command;

struct TestEnvironment {
    _temp_dir: TempDir,
    #[allow(dead_code)]
    test_files: Vec<PathBuf>,
}

impl TestEnvironment {
    fn new(num_files: usize, lines_per_file: usize) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let mut test_files = Vec::new();

        for file_idx in 0..num_files {
            let file_path = temp_dir.path().join(format!("session_{file_idx}.jsonl"));
            let mut file = File::create(&file_path).unwrap();

            for line_idx in 0..lines_per_file {
                let content = match line_idx % 5 {
                    0 => format!("Writing code for feature {line_idx}"),
                    1 => format!("Debugging issue with error code {line_idx}"),
                    2 => format!("Testing functionality of component {line_idx}"),
                    3 => format!("Optimizing performance of algorithm {line_idx}"),
                    _ => format!("Implementing new feature request {line_idx}"),
                };

                writeln!(
                    file,
                    r#"{{"type":"user","message":{{"role":"user","content":"{content}"}},"uuid":"{file_idx}-{line_idx}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session{file_idx}","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                    line_idx % 60
                ).unwrap();
            }

            test_files.push(file_path);
        }

        TestEnvironment {
            _temp_dir: temp_dir,
            test_files,
        }
    }
}

// Benchmark cold start - simulates real CLI invocation
fn benchmark_cold_start_cli(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_start_cli");
    group.sample_size(10); // Fewer samples for process spawning
    
    for engine in ["smol", "rayon"] {
        group.bench_with_input(
            BenchmarkId::new("full_process", engine),
            &engine,
            |b, &engine| {
                b.iter(|| {
                    let output = Command::new("./target/release/ccms")
                        .args(&["claude", "--engine", engine])
                        .env("CCMS_NO_COLOR", "1") // Disable color for consistent output
                        .output()
                        .expect("Failed to execute command");
                    black_box(output);
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark engine startup overhead
fn benchmark_startup_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_overhead");
    let options = SearchOptions::default();
    
    group.bench_function("smol_new", |b| {
        b.iter(|| {
            let engine = SmolEngine::new(options.clone());
            black_box(engine);
        });
    });
    
    group.bench_function("rayon_new", |b| {
        b.iter(|| {
            let engine = RayonEngine::new(options.clone());
            black_box(engine);
        });
    });
    
    group.bench_function("rayon_limited_new", |b| {
        b.iter(|| {
            let engine = RayonLimitedEngine::new(options.clone());
            black_box(engine);
        });
    });
    
    group.finish();
}

fn benchmark_multi_file_search(c: &mut Criterion) {
    let env = TestEnvironment::new(10, 1000);
    let pattern = env._temp_dir.path().join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let mut group = c.benchmark_group("multi_file");

    // Simple search
    let query = parse_query("error").unwrap();
    let options = SearchOptions::default();
    
    group.bench_with_input(
        BenchmarkId::new("smol", "simple_10x1000"),
        &(&pattern_str, &query, &options),
        |b, (pattern, query, options)| {
            b.iter(|| {
                let engine = SmolEngine::new((*options).clone());
                let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                black_box(results.len())
            });
        },
    );
    
    group.bench_with_input(
        BenchmarkId::new("rayon", "simple_10x1000"),
        &(&pattern_str, &query, &options),
        |b, (pattern, query, options)| {
            b.iter(|| {
                let engine = RayonEngine::new((*options).clone());
                let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                black_box(results.len())
            });
        },
    );

    // Complex search
    let query = parse_query("error AND code").unwrap();
    
    group.bench_with_input(
        BenchmarkId::new("smol", "complex_10x1000"),
        &(&pattern_str, &query, &options),
        |b, (pattern, query, options)| {
            b.iter(|| {
                let engine = SmolEngine::new((*options).clone());
                let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                black_box(results.len())
            });
        },
    );
    
    group.bench_with_input(
        BenchmarkId::new("rayon", "complex_10x1000"),
        &(&pattern_str, &query, &options),
        |b, (pattern, query, options)| {
            b.iter(|| {
                let engine = RayonEngine::new((*options).clone());
                let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                black_box(results.len())
            });
        },
    );

    group.finish();
}

// Benchmark first search (cold cache)
fn benchmark_first_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("first_search");
    
    // Create a small test file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&file_path).unwrap();
    
    for i in 0..100 {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Test message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i, i
        ).unwrap();
    }
    
    let pattern = file_path.to_str().unwrap();
    let query = parse_query("test").unwrap();
    let options = SearchOptions::default();
    
    // First search includes all initialization
    group.bench_function("smol_cold", |b| {
        b.iter(|| {
            let engine = SmolEngine::new(options.clone());
            let (results, _, _) = engine.search(pattern, query.clone()).unwrap();
            black_box(results.len())
        });
    });
    
    group.bench_function("rayon_cold", |b| {
        b.iter(|| {
            let engine = RayonEngine::new(options.clone());
            let (results, _, _) = engine.search(pattern, query.clone()).unwrap();
            black_box(results.len())
        });
    });
    
    group.finish();
}

// Benchmark with real-world query patterns
fn benchmark_real_queries(c: &mut Criterion) {
    let env = TestEnvironment::new(20, 500); // 20 files, 500 lines each
    let pattern = env._temp_dir.path().join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();
    
    let mut group = c.benchmark_group("real_queries");
    
    let queries = vec![
        ("simple", "error"),
        ("phrase", "\"error code\""),
        ("and", "error AND debug"),
        ("or", "error OR warning OR info"),
        ("not", "NOT test"),
        ("complex", "(error OR warning) AND NOT test"),
    ];
    
    let options = SearchOptions {
        max_results: Some(50), // Real usage typically limits results
        ..Default::default()
    };
    
    for (name, query_str) in queries {
        let query = parse_query(query_str).unwrap();
        
        group.bench_with_input(
            BenchmarkId::new("smol", name),
            &(&pattern_str, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = SmolEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("rayon", name),
            &(&pattern_str, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_startup_overhead,
    benchmark_first_search,
    benchmark_multi_file_search,
    benchmark_real_queries,
    benchmark_cold_start_cli
);
criterion_main!(benches);
