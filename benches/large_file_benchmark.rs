use ccms::{SearchEngineTrait, SmolEngine, SearchOptions, parse_query};
use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn create_large_test_data(num_lines: usize) -> String {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("large_test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    for i in 0..num_lines {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message {} with some test content that is longer to simulate real messages"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i, i, i % 60
        ).unwrap();
    }

    test_file.to_string_lossy().to_string()
}

fn benchmark_large_file_search(c: &mut Criterion) {
    let test_file = create_large_test_data(10000);
    let query = parse_query("test").unwrap();
    let options = SearchOptions::default();

    c.bench_function("search_10k_lines", |b| {
        b.iter(|| {
            let engine = SmolEngine::new(options.clone());
            let (results, _, _) = engine.search(&test_file, black_box(query.clone())).unwrap();
            results
        });
    });
}

fn benchmark_very_large_file_search(c: &mut Criterion) {
    let test_file = create_large_test_data(50000);
    let query = parse_query("test AND content").unwrap();
    let options = SearchOptions::default();

    c.bench_function("search_50k_lines", |b| {
        b.iter(|| {
            let engine = SmolEngine::new(options.clone());
            let (results, _, _) = engine.search(&test_file, black_box(query.clone())).unwrap();
            results
        });
    });
}

criterion_group!(
    benches,
    benchmark_large_file_search,
    benchmark_very_large_file_search
);
criterion_main!(benches);
