use ccms::{SearchEngineTrait, SearchOptions, SmolEngine, parse_query};
use codspeed_criterion_compat::{BenchmarkId, black_box};
use codspeed_criterion_compat::{Criterion, criterion_group, criterion_main};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

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

fn benchmark_async_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_search");

    // Use smol runtime for async benchmarks
    let ex = smol::Executor::new();

    for size in [100, 1000, 10000].iter() {
        let (_temp_dir, test_file) = create_test_jsonl(*size, 100);

        group.bench_with_input(BenchmarkId::new("file_size_lines", size), size, |b, _| {
            b.iter(|| {
                smol::block_on(ex.run(async {
                    let options = SearchOptions::default();
                    let engine = SmolEngine::new(options);
                    let query = parse_query("Message AND test").unwrap();
                    let (results, _, _) = engine.search(&test_file, query).unwrap();
                    black_box(results);
                }))
            });
        });
    }

    group.finish();
}

fn benchmark_query_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_complexity");
    let (_temp_dir, test_file) = create_test_jsonl(1000, 100);

    let ex = smol::Executor::new();

    let queries = vec![
        ("simple", "test"),
        ("and", "Message AND content"),
        ("or", "Message OR content OR test"),
        ("not", "Message AND NOT error"),
        ("complex", "(Message AND content) OR (test AND NOT error)"),
        ("regex", r"/Message.*test/"),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(BenchmarkId::new("query", name), &query_str, |b, q| {
            b.iter(|| {
                smol::block_on(ex.run(async {
                    let options = SearchOptions::default();
                    let engine = SmolEngine::new(options);
                    let query = parse_query(q).unwrap();
                    let (results, _, _) = engine.search(&test_file, query).unwrap();
                    black_box(results);
                }))
            });
        });
    }

    group.finish();
}

fn benchmark_concurrent_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_search");
    let (_temp_dir, test_file) = create_test_jsonl(1000, 100);

    group.bench_function("concurrent_4_searches", |b| {
        b.iter(|| {
            let ex = smol::Executor::new();
            smol::block_on(ex.run(async {
                let mut tasks = Vec::new();
                for _ in 0..4 {
                    let test_file = test_file.clone();
                    let task = ex.spawn(async move {
                        let options = SearchOptions::default();
                        let engine = SmolEngine::new(options);
                        let query = parse_query("Message").unwrap();
                        let (results, _, _) = engine.search(&test_file, query).unwrap();
                        black_box(results);
                    });
                    tasks.push(task);
                }

                // Wait for all tasks to complete
                for task in tasks {
                    task.await;
                }
            }))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_async_search,
    benchmark_query_complexity,
    benchmark_concurrent_search
);
criterion_main!(benches);
