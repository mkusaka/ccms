use ccms::{SearchEngine, SearchOptions, parse_query};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn create_test_data(num_lines: usize) -> String {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    for i in 0..num_lines {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message {} with some test content"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i, i, i % 60
        ).unwrap();
    }

    test_file.to_string_lossy().to_string()
}

fn benchmark_simple_search(c: &mut Criterion) {
    let test_file = create_test_data(1000);
    let query = parse_query("test").unwrap();
    let options = SearchOptions::default();

    c.bench_function("simple_search_1000", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine.search(&test_file, black_box(query.clone())).unwrap();
            results
        });
    });
}

fn benchmark_complex_search(c: &mut Criterion) {
    let test_file = create_test_data(1000);
    let query = parse_query("test AND content").unwrap();
    let options = SearchOptions::default();

    c.bench_function("complex_search_1000", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine.search(&test_file, black_box(query.clone())).unwrap();
            results
        });
    });
}

fn benchmark_regex_search(c: &mut Criterion) {
    let test_file = create_test_data(1000);
    let query = parse_query("/Message.*content/i").unwrap();
    let options = SearchOptions::default();

    c.bench_function("regex_search_1000", |b| {
        b.iter(|| {
            let engine = SearchEngine::new(options.clone());
            let (results, _, _) = engine.search(&test_file, black_box(query.clone())).unwrap();
            results
        });
    });
}

fn benchmark_json_parsing(c: &mut Criterion) {
    let json_line = r#"{"type":"user","message":{"role":"user","content":"Test message with some content"},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#;

    c.bench_function("json_parse_single", |b| {
        b.iter(|| {
            let mut bytes = json_line.as_bytes().to_vec();
            let _: ccms::SessionMessage = simd_json::serde::from_slice(&mut bytes).unwrap();
        });
    });
}

fn benchmark_query_parsing(c: &mut Criterion) {
    c.bench_function("parse_simple_query", |b| {
        b.iter(|| parse_query(black_box("\"hello world\"")).unwrap());
    });

    c.bench_function("parse_complex_query", |b| {
        b.iter(|| parse_query(black_box("(hello OR world) AND NOT /test/i")).unwrap());
    });
}

criterion_group!(
    benches,
    benchmark_simple_search,
    benchmark_complex_search,
    benchmark_regex_search,
    benchmark_json_parsing,
    benchmark_query_parsing
);
criterion_main!(benches);
