use ccms::query::{parse_query, SearchOptions};
use ccms::search::SearchEngine;
#[cfg(feature = "duckdb")]
use ccms::search::DuckDBSearchEngine;
use codspeed_criterion_compat::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn create_test_data(num_messages: usize) -> std::path::PathBuf {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    // Create diverse test data
    for i in 0..num_messages {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let content = match i % 5 {
            0 => format!("Error: Connection timeout after {} attempts", i),
            1 => format!("Warning: Deprecated function used at line {}", i),
            2 => format!("Info: Process {} completed successfully", i),
            3 => format!("Debug: Variable state at iteration {}", i),
            _ => format!("Hello world from message {}", i),
        };
        
        if role == "user" {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"{content}"}},"uuid":"uuid-{i}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                i % 60
            ).unwrap();
        } else {
            writeln!(
                file,
                r#"{{"type":"assistant","message":{{"id":"msg{i}","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"{content}"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"uuid-{i}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":"uuid-{}","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                i % 60,
                i - 1
            ).unwrap();
        }
    }

    test_file
}

fn benchmark_single_search(c: &mut Criterion) {
    let test_sizes = vec![100, 1000, 10000];
    let queries = vec![
        ("simple", "Error"),
        ("and", "Error AND Connection"),
        ("or", "Error OR Warning"),
        ("not", "NOT Debug"),
        ("complex", "(Error OR Warning) AND NOT test"),
        ("regex", "/Error.*\\d+/"),
    ];

    let mut group = c.benchmark_group("single_search");
    group.sample_size(10); // Reduce sample size for faster benchmarking

    for &size in &test_sizes {
        let test_file = create_test_data(size);
        let file_path = test_file.to_str().unwrap();

        for &(query_name, query_str) in &queries {
            let query = parse_query(query_str).unwrap();
            let options = SearchOptions::default();

            // Benchmark original implementation
            group.bench_with_input(
                BenchmarkId::new(format!("original_{}", query_name), size),
                &file_path,
                |b, &file_path| {
                    b.iter(|| {
                        let engine = SearchEngine::new(options.clone());
                        let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
                        results.len()
                    });
                },
            );

            // Benchmark DuckDB implementation
            #[cfg(feature = "duckdb")]
            group.bench_with_input(
                BenchmarkId::new(format!("duckdb_{}", query_name), size),
                &file_path,
                |b, &file_path| {
                    b.iter(|| {
                        let engine = DuckDBSearchEngine::new(options.clone()).unwrap();
                        let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
                        results.len()
                    });
                },
            );
        }
    }

    group.finish();
}

fn benchmark_with_filters(c: &mut Criterion) {
    let test_file = create_test_data(5000);
    let file_path = test_file.to_str().unwrap();
    
    let mut group = c.benchmark_group("filtered_search");
    group.sample_size(10);

    let query = parse_query("Error").unwrap();

    // Test with role filter
    let options_with_role = SearchOptions {
        role: Some("user".to_string()),
        ..Default::default()
    };

    group.bench_function("original_role_filter", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options_with_role.clone());
            let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
            results.len()
        });
    });

    #[cfg(feature = "duckdb")]
    group.bench_function("duckdb_role_filter", |b| {
        b.iter(|| {
            let engine = DuckDBSearchEngine::new(options_with_role.clone()).unwrap();
            let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
            results.len()
        });
    });

    // Test with timestamp filter
    let options_with_timestamp = SearchOptions {
        after: Some("2024-01-01T00:00:30Z".to_string()),
        ..Default::default()
    };

    group.bench_function("original_timestamp_filter", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options_with_timestamp.clone());
            let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
            results.len()
        });
    });

    #[cfg(feature = "duckdb")]
    group.bench_function("duckdb_timestamp_filter", |b| {
        b.iter(|| {
            let engine = DuckDBSearchEngine::new(options_with_timestamp.clone()).unwrap();
            let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
            results.len()
        });
    });

    group.finish();
}

fn benchmark_large_file(c: &mut Criterion) {
    // Create a large file with 100k messages
    let test_file = create_test_data(100000);
    let file_path = test_file.to_str().unwrap();
    
    let mut group = c.benchmark_group("large_file");
    group.sample_size(5); // Even smaller sample size for large files

    let queries = vec![
        ("rare", "Connection timeout after 99999"), // Rare match
        ("common", "Error"), // Common match
        ("no_match", "ThisDoesNotExist"), // No matches
    ];

    for &(query_name, query_str) in &queries {
        let query = parse_query(query_str).unwrap();
        let options = SearchOptions::default();

        group.bench_with_input(
            BenchmarkId::new(format!("original_{}", query_name), "100k"),
            &file_path,
            |b, &file_path| {
                b.iter(|| {
                    let engine = SearchEngine::new(options.clone());
                    let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
                    results.len()
                });
            },
        );

        #[cfg(feature = "duckdb")]
        group.bench_with_input(
            BenchmarkId::new(format!("duckdb_{}", query_name), "100k"),
            &file_path,
            |b, &file_path| {
                b.iter(|| {
                    let engine = DuckDBSearchEngine::new(options.clone()).unwrap();
                    let (results, _, _) = engine.search(file_path, query.clone()).unwrap();
                    results.len()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_search,
    benchmark_with_filters,
    benchmark_large_file
);
criterion_main!(benches);