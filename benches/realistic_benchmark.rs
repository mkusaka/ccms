use ccms::{SearchEngine, SearchOptions, parse_query};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

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

fn benchmark_multi_file_search(c: &mut Criterion) {
    let env = TestEnvironment::new(10, 1000);
    let pattern = env._temp_dir.path().join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let mut group = c.benchmark_group("multi_file");

    // Simple search
    let query = parse_query("error").unwrap();
    let options = SearchOptions::default();
    group.bench_function("simple_10x1000", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine
                .search(&pattern_str, black_box(query.clone()))
                .unwrap();
            results
        });
    });

    // Complex search
    let query = parse_query("error AND code").unwrap();
    group.bench_function("complex_10x1000", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine
                .search(&pattern_str, black_box(query.clone()))
                .unwrap();
            results
        });
    });

    // Regex search
    let query = parse_query("/error.*\\d+/i").unwrap();
    group.bench_function("regex_10x1000", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine
                .search(&pattern_str, black_box(query.clone()))
                .unwrap();
            results
        });
    });

    group.finish();
}

fn benchmark_single_large_file(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.jsonl");
    let mut file = File::create(&file_path).unwrap();

    // Create 100k lines
    for i in 0..100_000 {
        let content = match i % 10 {
            0 => format!("Error: Connection failed with code {i}"),
            1 => format!("Warning: Deprecated function used in module {i}"),
            2 => format!("Info: Processing request {i}"),
            3 => format!("Debug: Variable value is {i}"),
            4 => format!("Error: File not found at path {i}"),
            5 => format!("Success: Operation completed for task {i}"),
            6 => format!("Error: Invalid input parameter {i}"),
            7 => format!("Info: Starting process {i}"),
            8 => format!("Warning: Memory usage high for process {i}"),
            _ => format!("Debug: Checkpoint reached at step {i}"),
        };

        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"{}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"large","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            content, i, i % 60
        ).unwrap();
    }

    let file_str = file_path.to_string_lossy().to_string();
    let mut group = c.benchmark_group("large_file");

    // Search for "error" in 100k lines
    let query = parse_query("error").unwrap();
    let options = SearchOptions::default();
    group.bench_function("search_100k", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine.search(&file_str, black_box(query.clone())).unwrap();
            results
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_multi_file_search,
    benchmark_single_large_file
);
criterion_main!(benches);
