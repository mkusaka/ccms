use ccms::parse_query;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[cfg(feature = "async")]
use ccms::{AsyncSearchEngine, AsyncSearchOptions};

// Create test JSONL files for benchmarking
fn create_test_jsonl(num_lines: usize, line_size: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    let long_text = "x".repeat(line_size);

    for i in 0..num_lines {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message {} test content {}"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i,
            long_text,
            i,
            i % 60
        )
        .unwrap();
    }

    (temp_dir, test_file.to_string_lossy().to_string())
}

#[cfg(feature = "async")]
fn benchmark_async_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_search");

    let rt = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 1000, 10000].iter() {
        let (_temp_dir, test_file) = create_test_jsonl(*size, 100);
        let pattern = format!(
            "{}/*.jsonl",
            std::path::Path::new(&test_file).parent().unwrap().display()
        );

        let query = parse_query("test").unwrap();
        let options = AsyncSearchOptions {
            max_results: Some(50),
            ..Default::default()
        };

        group.bench_with_input(BenchmarkId::new("tokio", size), &pattern, |b, pattern| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = AsyncSearchEngine::new(options.clone());
                    let (results, _, _) = engine.search(pattern, query.clone()).await.unwrap();
                    black_box(results.len())
                })
            });
        });
    }

    group.finish();
}

#[cfg(feature = "async")]
fn benchmark_async_concurrency(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_concurrency");

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create multiple files
    let num_files = 20;
    let lines_per_file = 100;
    let mut _temp_dirs = Vec::new();
    let mut pattern = String::new();

    for i in 0..num_files {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join(format!("test{i}.jsonl"));
        let mut file = File::create(&test_file).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {}"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                j,
                j % 60
            )
            .unwrap();
        }

        if i == 0 {
            pattern = format!("{}/*.jsonl", temp_dir.path().display());
        }
        _temp_dirs.push(temp_dir);
    }

    let query = parse_query("Message").unwrap();

    for concurrency in [1, 10, 50, 100].iter() {
        let options = AsyncSearchOptions {
            max_results: Some(100),
            max_concurrent_files: *concurrency,
            ..Default::default()
        };

        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            &pattern,
            |b, pattern| {
                b.iter(|| {
                    rt.block_on(async {
                        let engine = AsyncSearchEngine::new(options.clone());
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
fn benchmark_async_search(_c: &mut Criterion) {
    println!("Async benchmarks require --features async");
}

#[cfg(not(feature = "async"))]
fn benchmark_async_concurrency(_c: &mut Criterion) {
    println!("Async benchmarks require --features async");
}

criterion_group!(benches, benchmark_async_search, benchmark_async_concurrency);
criterion_main!(benches);
